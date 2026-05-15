use gtk4::prelude::*;
use gtk4::{self as gtk, Orientation};
use gtk4::{gdk};
use gdk_pixbuf::PixbufLoader;
use std::cell::RefCell;
use std::rc::Rc;
use crate::ui::status_bar::StatusBar;

const ORBIT_LOGO: &[u8] = include_bytes!("../../assets/Logo.png");

#[derive(Clone)]
pub struct Header {
    container: gtk::Box,
    wifi_tab: gtk::Button,
    bluetooth_tab: gtk::Button,
    vpn_tab: gtk::Button,
    audio_tab: gtk::Button,
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

        let status_bar = StatusBar::new();
        container.append(status_bar.widget());

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
        
        let audio_tab = gtk::Button::builder()
            .label("Audio")
            .css_classes(["orbit-tab", "flat"])
            .hexpand(true)
            .build();

        tab_bar.append(&wifi_tab);
        tab_bar.append(&bluetooth_tab);
        tab_bar.append(&vpn_tab);
        tab_bar.append(&audio_tab);

        container.append(&title_row);
        container.append(&tab_bar);

        Self {
            container,
            wifi_tab,
            bluetooth_tab,
            vpn_tab,
            audio_tab,
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
    
    pub fn audio_tab(&self) -> &gtk::Button {
        &self.audio_tab
    }

    pub fn wired_button(&self) -> &gtk::Button {
        &self.wired_button
    }

    pub fn set_tab(&self, tab: &str) {
        self.wifi_tab.remove_css_class("active");
        self.bluetooth_tab.remove_css_class("active");
        self.vpn_tab.remove_css_class("active");
        self.audio_tab.remove_css_class("active");
        self.power_box.set_visible(false);
        self.wired_button.set_visible(false);

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
            }
            "vpn" => {
                self.vpn_tab.add_css_class("active");
            }
            "audio" => {
                self.audio_tab.add_css_class("active");
            }
            _ => {}
        }
    }
}

