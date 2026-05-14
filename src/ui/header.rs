use gtk4::prelude::*;
use gtk4::{self as gtk, Orientation};
use gtk4::{gdk};
use gdk_pixbuf::PixbufLoader;
use std::cell::RefCell;
use std::rc::Rc;
use starship_battery::{Manager};

const ORBIT_LOGO: &[u8] = include_bytes!("../../assets/Logo.png");

#[derive(Clone)]
pub struct Header {
    container: gtk::Box,
    wifi_tab: gtk::Button,
    bluetooth_tab: gtk::Button,
    vpn_tab: gtk::Button,
    wired_button: gtk::Button,
    power_switch: gtk::Switch,
    power_box: gtk::Box,
    power_label: gtk::Label,
    is_programmatic_update: Rc<RefCell<bool>>,
}

impl Header {
    pub fn new() -> Self {
        let container = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .css_classes(["orbit-header"])
            .spacing(8)
            .build();

        use std::rc::Rc;
        use std::time::Duration;

        let battery_manager = match Manager::new() {
            Ok(m) => Some(m),
            Err(e) => {
                eprintln!("Battery subsystem disabled (system may lack a battery): {}", e);
                None
            }
        };

        if let Some(manager) = battery_manager {
            let battery_track = gtk::Grid::builder()
                .orientation(Orientation::Horizontal)
                .css_classes(["battery-track"])
                .build();

            battery_track.set_size_request(120, 40);

            let battery_fill = gtk::Box::builder()
                .orientation(Orientation::Horizontal)
                .spacing(0)
                .css_classes(["battery-fill"])
                .build();

            let spacer = gtk::Box::builder()
                .orientation(Orientation::Horizontal)
                .spacing(0)
                .build();

            battery_track.attach(&battery_fill, 0, 0, 1, 1);
            battery_track.attach(&spacer, 1, 0, 1, 1);

            container.append(&battery_track);

            // Wrap GTK widgets so they can move into the timer closure
            let battery_track = Rc::new(battery_track);
            let battery_fill = Rc::new(battery_fill);
            let spacer = Rc::new(spacer);
            let manager = Rc::new(manager);

            gtk::glib::timeout_add_local(Duration::from_secs(5), move || {
                // ---- get battery ----
                let mut percentage: u32 = 0;

                if let Ok(batteries) = manager.batteries() {
                    for b in batteries.flatten() {
                        let state = b.state_of_charge();
                        percentage = (state.value * 100.0) as u32;
                    }
                }

                let charge = percentage.clamp(0, 100);
                let empty = 100 - charge;

                // ---- styling ----
                if charge <= 20 {
                    battery_fill.remove_css_class("normal");
                    battery_fill.add_css_class("low");
                } else {
                    battery_fill.remove_css_class("low");
                    battery_fill.add_css_class("normal");
                }

                // ---- update layout ----
                battery_track.remove(&*battery_fill);
                battery_track.remove(&*spacer);

                battery_track.attach(&*battery_fill, 0, 0, charge.max(1) as i32, 1);
                battery_track.attach(&*spacer, charge.max(1) as i32, 0, empty.max(1) as i32, 1);

                gtk::glib::ControlFlow::Continue
            });
        }

        let title_row = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(4)
            .build();

        let loader = PixbufLoader::new();
        loader.write(ORBIT_LOGO).expect("Failed to load embedded Orbit logo");
        loader.close().expect("Failed to close logo loader");
        let pixbuf = loader.pixbuf().expect("Failed to get logo pixbuf");
        let logo_texture = gdk::Texture::for_pixbuf(&pixbuf);

        let orbit_icon = gtk::Image::builder()
            .paintable(&logo_texture)
            .pixel_size(64)
            .build();

        let logo_container = gtk::Box::builder()
            .css_classes(["orbit-logo-container"])
            .valign(gtk::Align::Center)
            .build();
        logo_container.append(&orbit_icon);

        let title = gtk::Label::builder()
            .label("FusionPanel")
            .css_classes(["orbit-title"])
            .hexpand(true)
            .halign(gtk::Align::Start)
            .build();

        let power_switch = gtk::Switch::builder()
            .css_classes(["orbit-toggle-switch"])
            .active(false)
            .sensitive(false)
            .build();

        let power_label = gtk::Label::builder()
            .label("WiFi")
            .css_classes(["orbit-status"])
            .build();

        let power_box = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .valign(gtk::Align::Center)
            .build();
        power_box.append(&power_label);
        power_box.append(&power_switch);

        let wired_button = gtk::Button::builder()
            .icon_name("network-wired-symbolic")
            .css_classes(["orbit-button", "flat", "orbit-wired-button"])
            .tooltip_text("Wired Connections")
            .valign(gtk::Align::Center)
            .build();

        title_row.append(&logo_container);
        title_row.append(&title);
        title_row.append(&power_box);
        title_row.append(&wired_button);

        let tab_bar = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .css_classes(["orbit-tab-bar"])
            .homogeneous(true)
            .build();

        let wifi_tab = gtk::Button::builder()
            .label("WiFi")
            .css_classes(["orbit-tab", "flat", "active"])
            .hexpand(true)
            .build();

        let bluetooth_tab = gtk::Button::builder()
            .label("Bluetooth")
            .css_classes(["orbit-tab", "flat"])
            .hexpand(true)
            .build();

        let vpn_tab = gtk::Button::builder()
            .label("VPN")
            .css_classes(["orbit-tab", "flat"])
            .hexpand(true)
            .build();

        tab_bar.append(&wifi_tab);
        tab_bar.append(&bluetooth_tab);
        tab_bar.append(&vpn_tab);

        container.append(&title_row);
        container.append(&tab_bar);

        Self {
            container,
            wifi_tab,
            bluetooth_tab,
            vpn_tab,
            wired_button,
            power_switch,
            power_box,
            power_label,
            is_programmatic_update: Rc::new(RefCell::new(false)),
        }
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }

    pub fn set_power_state(&self, enabled: bool) {
        *self.is_programmatic_update.borrow_mut() = true;
        self.power_switch.set_sensitive(true);
        self.power_switch.set_active(enabled);
        *self.is_programmatic_update.borrow_mut() = false;
    }

    pub fn is_programmatic_update(&self) -> bool {
        *self.is_programmatic_update.borrow()
    }

    pub fn power_switch(&self) -> &gtk::Switch {
        &self.power_switch
    }

    pub fn wifi_tab(&self) -> &gtk::Button {
        &self.wifi_tab
    }

    pub fn bluetooth_tab(&self) -> &gtk::Button {
        &self.bluetooth_tab
    }

    pub fn vpn_tab(&self) -> &gtk::Button {
        &self.vpn_tab
    }

    pub fn wired_button(&self) -> &gtk::Button {
        &self.wired_button
    }

    pub fn set_tab(&self, tab: &str) {
        self.wifi_tab.remove_css_class("active");
        self.bluetooth_tab.remove_css_class("active");
        self.vpn_tab.remove_css_class("active");

        match tab {
            "wifi" | "saved" => {
                self.wifi_tab.add_css_class("active");
                self.power_box.set_visible(true);
                self.power_label.set_label("WiFi");
                self.wired_button.set_visible(true);
            }
            "bluetooth" => {
                self.bluetooth_tab.add_css_class("active");
                self.power_box.set_visible(true);
                self.power_label.set_label("Bluetooth");
                self.wired_button.set_visible(false);
            }
            "vpn" => {
                self.vpn_tab.add_css_class("active");
                self.power_box.set_visible(false);
                self.wired_button.set_visible(false);
            }
            _ => {}
        }
    }
}

