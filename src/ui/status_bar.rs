
use std::rc::Rc;
use std::time::Duration;
use gtk4::{self as gtk, glib, Orientation};
use starship_battery::{Battery, Manager, State};

#[derive(Clone)]
pub struct StatusBar {
    container: gtk::CenterBox,
    time_label: gtk::Label,
    date_label: gtk::Label,
    icons_label: gtk::Label,
    battery: Option<Rc<Battery>>,
}
impl StatusBar {
    pub fn new() -> Self {

        let container = gtk::CenterBox::builder()
            .orientation(Orientation::Horizontal)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let time_label = gtk::Label::builder()
            .label(Self::get_time())
            .css_classes(["orbit-clock"])
            .halign(gtk::Align::Start)
            .margin_start(12)
            .build();

        container.set_start_widget(Some(&time_label));

        let date_label = gtk::Label::builder()
            .label(Self::get_date())
            .halign(gtk::Align::Center)
            .build();

        container.set_center_widget(Some(&date_label));

        let icons_label = gtk::Label::builder()
            .label("")
            .css_classes(["orbit-battery"])
            .halign(gtk::Align::End)
            .margin_end(12)
            .build();

        container.set_end_widget(Some(&icons_label));

        let battery_manager = match Manager::new() {
            Ok(m) => Some(m),
            Err(_) => None
        };

        let mut battery: Option<Rc<Battery>> = None;

        if let Some(manager) = battery_manager {
            if let Ok(batteries) = manager.batteries() {
                for bat in batteries.flatten() {
                    battery = Some(Rc::new(bat));
                    break;
                }
            }
        }

        let status_bar = Self {
            container,
            time_label,
            date_label,
            icons_label,
            battery
        };

        let self_rc = Rc::new(status_bar.clone());

        glib::timeout_add_local(
            Duration::from_secs(1),
             move || {
                self_rc.tick();

                glib::ControlFlow::Continue
            }
        );

        status_bar
    }

    pub fn widget(&self) -> &gtk::CenterBox {
        &self.container
    }

    pub fn tick(&self) {
        self.time_label.set_label(&Self::get_time());
        self.date_label.set_label(&Self::get_date());
        self.icons_label.set_label(&self.get_icons());
    }

    fn get_time() -> String {
        chrono::Local::now()
            .format("%I:%M %p")
            .to_string()
    }

    fn get_date() -> String {
        chrono::Local::now()
            .format("%a, %e %b %Y")
            .to_string()
    }

    fn get_icons(&self) -> String {
        // 1. Check VPN Status (looking for common interfaces like tun0, wg0)
        let vpn_interfaces = ["tun0", "wg0", "ppp0", "nordlynx"];
        let is_vpn = vpn_interfaces.iter().any(|face| {
            std::path::Path::new("/sys/class/net").join(face).exists()
        });
        let vpn_icon = if is_vpn { "🔒 " } else { "" };

        // 2. Check Internet Connectivity Status via Linux sysfs
        let mut is_connected = false;
        if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
            for entry in entries.flatten() {
                // Ignore loopback (lo) and VPN interfaces when checking for main connection
                let name = entry.file_name().to_string_lossy().into_owned();
                if name != "lo" && !vpn_interfaces.contains(&name.as_str()) {
                    if let Ok(carrier) = std::fs::read_to_string(entry.path().join("carrier")) {
                        if carrier.trim() == "1" {
                            is_connected = true;
                            break;
                        }
                    }
                }
            }
        }
        let net_icon = if is_connected { "🌐 " } else { "✈️ " };

        // 3. Process Battery Status if available
        let battery_part = if let Some(battery) = &self.battery {
            let mut charge: i32 = (battery.state_of_charge().value * 100.0) as i32;
            let charging = matches!(battery.state(), State::Charging);
            charge = charge.clamp(0, 100);

            let bat_icon = if charging { "🔌" } else if charge <= 10 { "🪫" } else { "🔋" };
            format!("{} {}%", bat_icon, charge)
        } else {
            "".to_string()
        };

        // 4. Combine all icons cleanly with spacing
        format!("{}{}{}", vpn_icon, net_icon, battery_part).trim().to_string()
    }
}
