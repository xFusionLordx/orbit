use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Orientation};

use image::{DynamicImage, ImageBuffer};

use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{ApiBackend, CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::Camera;

use std::process::Command;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
    mpsc,
};
use std::thread;
use std::time::Duration;
use gtk4_layer_shell::{KeyboardMode, Layer, LayerShell};
use tempfile::tempdir;

// =====================
// Messages
// =====================
enum CameraMessage {
    Frame {
        bytes: Vec<u8>,
        width: u32,
        height: u32,
    },
    VpnDetected {
        payload: String,
        vpn_type: VpnType,
    },
    Error(String),
}

#[derive(Clone, Copy)]
enum VpnType {
    WireGuard,
    OpenVpn,
}

// =====================
// Main entry
// =====================
pub fn launch_qr_preview_dialog(parent_window: &gtk4::ApplicationWindow) {
    // 1. Build the window without presenting it yet
    let dialog = ApplicationWindow::builder()
        .transient_for(parent_window)
        .modal(true)
        .title("Scan VPN QR Code")
        .css_classes(["orbit-panel"])
        .resizable(false)
        .decorated(true)
        .build();

    // 2. LAYER SHELL INITIALIZATION MUST HAPPEN FIRST BEFORE ANY SIZE OR SHOW CALLS
    dialog.init_layer_shell();
    dialog.set_layer(Layer::Overlay);
    dialog.set_keyboard_mode(KeyboardMode::Exclusive);

    // Explicitly set sizes after layer shell takes ownership of the surface
    dialog.set_default_size(640, 480);

    // 3. UI Widget Setup
    let root = gtk4::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    let picture = gtk4::Picture::new();
    picture.set_hexpand(true);
    picture.set_vexpand(true);

    let status = gtk4::Label::new(Some("Scanning QR code..."));

    let cancel = gtk4::Button::builder()
        .label("Cancel")
        .css_classes(["orbit-button", "flat"])
        .build();

    root.append(&picture);
    root.append(&status);
    root.append(&cancel);
    dialog.set_child(Some(&root));

    // 4. NOW PRESENT THE WINDOW - Wayland will correctly map it directly to the Overlay layer
    dialog.present();

    // ---------------- state ----------------
    let should_stop = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel::<CameraMessage>();

    // ---------------- cancel ----------------
    {
        let should_stop = should_stop.clone();
        let dialog = dialog.clone();
        cancel.connect_clicked(move |_| {
            should_stop.store(true, Ordering::Relaxed);
            dialog.close();
        });
    }

    {
        let should_stop = should_stop.clone();
        dialog.connect_close_request(move |_| {
            should_stop.store(true, Ordering::Relaxed);
            glib::Propagation::Proceed
        });
    }

    // =========================
    // CAMERA THREAD
    // =========================
    {
        let tx = tx.clone();
        let should_stop = should_stop.clone();
        thread::spawn(move || {
            let index = CameraIndex::Index(0);
            let format = RequestedFormat::new::<RgbFormat>(
                RequestedFormatType::AbsoluteHighestFrameRate,
            );

            let mut camera = match Camera::new(index, format) {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(CameraMessage::Error(e.to_string()));
                    return;
                }
            };

            if camera.open_stream().is_err() {
                let _ = tx.send(CameraMessage::Error("Failed to open camera".into()));
                return;
            }

            let decoder = bardecoder::default_decoder();
            let mut frame_counter = 0usize;

            while !should_stop.load(Ordering::Relaxed) {
                let Ok(frame) = camera.frame() else { continue };
                let Ok(img) = frame.decode_image::<RgbFormat>() else { continue };

                let width = img.width();
                let height = img.height();
                let raw = img.into_raw();

                let _ = tx.send(CameraMessage::Frame {
                    bytes: raw.clone(),
                    width,
                    height,
                });

                frame_counter += 1;
                if frame_counter % 5 != 0 {
                    continue;
                }

                let Some(rgb) = ImageBuffer::from_raw(width, height, raw.clone()) else { continue; };
                let dynamic = DynamicImage::ImageRgb8(rgb);
                let results = decoder.decode(&dynamic);

                for result in results.into_iter().flatten() {
                    if let Some(vpn_type) = detect_vpn_type(&result) {
                        let _ = tx.send(CameraMessage::VpnDetected {
                            payload: result,
                            vpn_type,
                        });
                        should_stop.store(true, Ordering::Relaxed);
                        break;
                    }
                }
                thread::sleep(Duration::from_millis(15));
            }
            let _ = camera.stop_stream();
        });
    }

    // =========================
    // GTK LOOP (mpsc polling)
    // =========================
    glib::timeout_add_local(Duration::from_millis(16), move || {
        while let Ok(msg) = rx.try_recv() {
            match msg {
                CameraMessage::Frame { bytes, width, height } => {
                    let stride = (width * 3) as usize;
                    let bytes = glib::Bytes::from(&bytes);
                    let texture = gdk::MemoryTexture::new(
                        width as i32,
                        height as i32,
                        gdk::MemoryFormat::R8g8b8,
                        &bytes,
                        stride,
                    );
                    picture.set_paintable(Some(&texture));
                }
                CameraMessage::VpnDetected { payload, vpn_type } => {
                    status.set_label("Enter a display name for this VPN");
                    root.remove(&picture);
                    root.remove(&cancel);

                    // Update sizes for the input phase
                    dialog.set_default_size(320, 128);

                    let entry = gtk4::Entry::new();
                    entry.set_placeholder_text(Some("VPN Name"));
                    let save = gtk4::Button::with_label("Save VPN");

                    root.append(&entry);
                    root.append(&save);
                    entry.grab_focus();

                    let payload = payload.clone();
                    let dialog = dialog.clone();
                    save.connect_clicked(move |_| {
                        let name = entry.text().trim().to_string();
                        if !name.is_empty() {
                            match import_vpn_from_qr_string(&payload, vpn_type, name) {
                                Ok(_) => println!("VPN imported successfully"),
                                Err(e) => eprintln!("VPN import failed: {}", e),
                            }
                            dialog.close();
                        }
                    });
                    return glib::ControlFlow::Break;
                }
                CameraMessage::Error(e) => {
                    status.set_label(&e);
                    dialog.close();
                    return glib::ControlFlow::Break;
                }
            }
        }
        glib::ControlFlow::Continue
    });
}


// =====================
// QR detection
// =====================
fn detect_vpn_type(s: &str) -> Option<VpnType> {
    if s.contains("[Interface]") && s.contains("PrivateKey") {
        return Some(VpnType::WireGuard);
    }

    if s.contains("client")
        || s.contains("<ca>")
        || s.contains("remote ")
    {
        return Some(VpnType::OpenVpn);
    }

    None
}

// =====================
// Name prompt
// =====================

// =====================
// VPN import
// =====================
fn import_vpn_from_qr_string(
    qr: &str,
    vpn_type: VpnType,
    name: String,
) -> Result<(), String> {
    let dir = tempdir().map_err(|e| e.to_string())?;

    // 1. Resolve a valid, unique filename matching NetworkManager requirements
    let file = match vpn_type {
        VpnType::WireGuard => {
            // Find an unused local interface identifier (e.g. wg0, wg1, wg2...)
            let mut index = 0;
            let mut selected_iface = format!("wg{}", index);

            while let Ok(output) = Command::new("nmcli").args(["connection", "show"]).output() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if !stdout.contains(&selected_iface) {
                    break; // Found a free interface slot!
                }
                index += 1;
                selected_iface = format!("wg{}", index);

                if index > 255 {
                    return Err("Too many virtual WireGuard interfaces configured".into());
                }
            }
            format!("{}.conf", selected_iface)
        }
        VpnType::OpenVpn => {
            // OpenVPN profiles do not dictate kernel driver names from filenames,
            // so we can use a basic sanitized string here.
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() % 100000)
                .unwrap_or(0);
            format!("ovpn_{}.ovpn", timestamp)
        }
    };

    let nm_type = match vpn_type {
        VpnType::WireGuard => "wireguard",
        VpnType::OpenVpn => "openvpn",
    };

    let path = dir.path().join(&file);
    std::fs::write(&path, qr).map_err(|e| e.to_string())?;

    // 2. Import the profile via the safe filename
    let output = Command::new("nmcli")
        .args([
            "connection",
            "import",
            "type",
            nm_type,
            "file",
            path.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into());
    }

    // 3. Extract the placeholder connection label assigned by nmcli
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let connection_source_name = stdout_str
        .split('\'')
        .nth(1)
        .unwrap_or(file.strip_suffix(".conf").or_else(|| file.strip_suffix(".ovpn")).unwrap())
        .to_string();

    // 4. Rename the user-visible profile identity to the chosen dynamic title
    // NetworkManager connections can use spaces and capital letters freely
    let timestamp_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() % 1000)
        .unwrap_or(0);
    let target_display_id = format!("{} ({})", name, timestamp_suffix);

    let rename_output = Command::new("nmcli")
        .args([
            "connection",
            "modify",
            &connection_source_name,
            "connection.id",
            &target_display_id,
        ])
        .output()
        .map_err(|e| e.to_string())?;

    if !rename_output.status.success() {
        return Err(String::from_utf8_lossy(&rename_output.stderr).into());
    }

    println!("Successfully configured multiple profiles. Saved as: {}", target_display_id);
    Ok(())
}



pub(crate) fn has_camera() -> bool {
    match nokhwa::query(ApiBackend::Auto) {
        Ok(devices) => !devices.is_empty(),
        Err(_) => false,
    }
}