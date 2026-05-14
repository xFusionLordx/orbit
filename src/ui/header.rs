use gtk4::prelude::*;
use gtk4::{self as gtk, CssProvider, Orientation};
use gtk4::{gdk, glib};
use gdk_pixbuf::PixbufLoader;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use starship_battery::{Manager, State};

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

        let battery_manager = match Manager::new() {
            Ok(m) => Some(m),
            Err(e) => {
                eprintln!("Battery subsystem disabled (system may lack a battery): {}", e);
                None
            }
        };
        if let Some(manager) = battery_manager {
            let battery_bar = gtk::Box::builder()
                .orientation(Orientation::Horizontal)
                .spacing(4)
                .build();

            battery_bar.add_css_class("battery-meter");
            battery_bar.set_visible(false);

            let label = gtk::Label::builder()
                .label("Battery: Loading...")
                .margin_start(8)
                .margin_end(8)
                .margin_top(4)
                .margin_bottom(4)
                .build();

            battery_bar.append(&label);
            // This does not overwrite or conflict with your existing application-wide provider
            let local_provider = CssProvider::new();

            battery_bar.style_context().add_provider(
                &local_provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1 // Higher priority ensures local overrides win
            );
            // Initialize the hardware connection manager

            // Keep loop ticking every 5 seconds
            let interval = Duration::from_secs(5);

            glib::timeout_add_local(interval, glib::clone!(
                #[weak]
                battery_bar,

                #[weak]
                label,

                #[upgrade_or]
                glib::ControlFlow::Break,

                move || {
                    if let Ok(mut batteries) = manager.batteries() {
                        if let Some(Ok(battery)) = batteries.next() {

                            let fraction = battery.state_of_charge().value;
                            let current_percentage = (fraction * 100.0).round() as u32;

                            let is_charging = matches!(
                                battery.state(),
                                State::Charging | State::Full
                            );

                            battery_bar.set_visible(true);
                            Self::update_battery_status(
                                &battery_bar,
                                &label,
                                current_percentage,
                                is_charging,
                            );
                        }
                    }

                    gtk::glib::ControlFlow::Continue
                }
            ));
            container.append(&battery_bar);
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

    fn update_battery_status(battery_box: &gtk::Box, label: &gtk::Label, percentage: u32, is_plugged_in: bool) {
        if is_plugged_in {
            label.set_label("Battery: AC Power");

            // Force the gradient to fill 100% using AC green
            battery_box.set_property("custom-css-properties", &format!("--bat-pct: 100%; --bat-color: #2ec27e;"));
        } else {
            label.set_label(&format!("Battery: {}%", percentage));

            let fill_color = if percentage <= 15 { "#e01b24" } else { "#3584e4" };

            // Pass the updated raw percentage and color state straight to the UI node
            battery_box.set_property(
                "custom-css-properties",
                &format!("--bat-pct: {}%; --bat-color: {};", percentage, fill_color)
            );
        }
    }

}

