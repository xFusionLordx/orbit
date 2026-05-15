use gtk4::prelude::*;
use gtk4::{self as gtk, Orientation};
use std::cell::RefCell;
use std::rc::Rc;

use crate::dbus::audio_manager::AudioDevice;

#[derive(Clone)]
pub struct Audio {
    container: gtk::Box,
    output_selector: gtk::DropDown,
    output_slider: gtk::Scale,
    output_mute_btn: gtk::Button,
    on_default_changed: Rc<RefCell<Option<Rc<dyn Fn(String)>>>>,
    on_volume_changed: Rc<RefCell<Option<Rc<dyn Fn(String, f64)>>>>,
    on_mute_toggled: Rc<RefCell<Option<Rc<dyn Fn(String, bool)>>>>,
}

impl Audio {
    pub fn new() -> Self {
        let container = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .vexpand(true)
            .hexpand(true)
            .spacing(16)
            .margin_start(16)
            .margin_end(16)
            .margin_top(16)
            .margin_bottom(16)
            .build();

        // ======================================================
        // 1. OUTPUT CONTROL GROUP
        // ======================================================
        let out_title = gtk::Label::builder()
            .label("OUTPUT AUDIO CHANNEL")
            .css_classes(["orbit-section-header"])
            .halign(gtk::Align::Start)
            .build();
        container.append(&out_title);

        let output_selector = gtk::DropDown::builder().hexpand(true).build();
        container.append(&output_selector);

        let out_controls = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .build();

        let output_mute_btn = gtk::Button::builder().valign(gtk::Align::Center).build();
        out_controls.append(&output_mute_btn);
        container.append(&out_controls);

        let output_slider = gtk::Scale::builder()
            .orientation(Orientation::Horizontal)
            .adjustment(&gtk::Adjustment::new(50.0, 0.0, 100.0, 1.0, 10.0, 0.0))
            .hexpand(true)
            .build();
        out_controls.append(&output_slider);

        // Separator layout bar
        container.append(&gtk::Separator::new(Orientation::Horizontal));

        let list = Self {
            container,
            output_selector,
            output_slider,
            output_mute_btn,
            on_default_changed: Rc::new(RefCell::new(None)),
            on_volume_changed: Rc::new(RefCell::new(None)),
            on_mute_toggled: Rc::new(RefCell::new(None)),
        };

        list.setup_interaction_handlers();
        list
    }

    pub fn set_audio_devices(&self, devices: Vec<AudioDevice>, active_id: Option<&str>) {

        let out_strings = gtk::StringList::new(&[]);

        let outputs: Vec<AudioDevice> = devices.iter().filter(|d| d.is_output).cloned().collect();

        // 1. Rebuild Output Channel Model
        let mut active_out_idx = 0;
        let mut active_out_dev: Option<AudioDevice> = None;
        for (idx, dev) in outputs.iter().enumerate() {
            out_strings.append(&dev.name);
            println!("{}", dev.name);
            if let Some(target_id) = active_id {
                if dev.id == target_id {
                    active_out_idx = idx as u32;
                    active_out_dev = Some(dev.clone());
                }
            }
        }
        self.output_selector.set_model(Some(&out_strings));
        if !outputs.is_empty() {
            self.output_selector.set_selected(active_out_idx);
            // Default to first output device if active_id match was not provided
            let current_out = active_out_dev.unwrap_or_else(|| outputs[0].clone());
            self.update_control_state(&self.output_slider, &self.output_mute_btn, &current_out);
        }
    }

    fn update_control_state(&self, scale: &gtk::Scale, mute_btn: &gtk::Button, device: &AudioDevice) {
        // 1. Fetch standard adjustment node properties
        scale.set_value(device.volume * 100.0);
        scale.set_sensitive(!device.is_muted);

        // 2. Set structural icon states
        let icon = if device.is_muted {
            "audio-volume-muted-symbolic"
        } else {
            "audio-volume-high-symbolic"
        };
        mute_btn.set_icon_name(icon);

        if device.is_muted {
            mute_btn.set_css_classes(&["orbit-button", "primary", "flat"]);
        } else {
            mute_btn.set_css_classes(&["orbit-button", "flat"]);
        }
    }


    fn setup_interaction_handlers(&self) {
        let cb_routing = self.on_default_changed.clone();
        let cb_vol = self.on_volume_changed.clone();
        let cb_mute = self.on_mute_toggled.clone();

        // ======================================================
        // 1. DROPDOWN SELECTION ROUTING LISTENER
        // ======================================================
        let cb = cb_routing.clone();
        self.output_selector.connect_selected_item_notify(move |drop| {
            let selected_idx = drop.selected();
            if selected_idx != gtk::INVALID_LIST_POSITION {
                // Fetch the model, cast it back to a StringList, and extract the string value directly
                if let Some(model) = drop.model() {
                    if let Some(string_list) = model.downcast_ref::<gtk::StringList>() {
                        if let Some(device_name) = string_list.string(selected_idx) {
                            if let Some(ref run) = *cb.borrow() {
                                // Pass the exact human-readable device name text down to the glue layer
                                run(device_name.to_string());
                            }
                        }
                    }
                }
            }
        });

        // ======================================================
        // 2. SLIDER SCALE INTERACTION SIGNAL LISTENER
        // ======================================================
        let cb_v = cb_vol.clone();
        let drop_out_vol = self.output_selector.clone();
        self.output_slider.connect_value_changed(move |scale| {
            let selected_idx = drop_out_vol.selected();
            if selected_idx != gtk::INVALID_LIST_POSITION {
                if let Some(model) = drop_out_vol.model() {
                    if let Some(string_list) = model.downcast_ref::<gtk::StringList>() {
                        if let Some(device_name) = string_list.string(selected_idx) {
                            if let Some(ref run) = *cb_v.borrow() {
                                // Pass the string name and slider fractional scale (0.0 to 1.0)
                                run(device_name.to_string(), scale.value() / 100.0);
                            }
                        }
                    }
                }
            }
        });

        // ======================================================
        // 3. MUTE BUTTON CLICK SIGNAL LISTENER
        // ======================================================
        let cb_m = cb_mute.clone();
        let drop_out_mute = self.output_selector.clone();
        let mute_btn_clone = self.output_mute_btn.clone();

        self.output_mute_btn.connect_clicked(move |_| {
            let selected_idx = drop_out_mute.selected();
            if selected_idx != gtk::INVALID_LIST_POSITION {
                if let Some(model) = drop_out_mute.model() {
                    if let Some(string_list) = model.downcast_ref::<gtk::StringList>() {
                        if let Some(device_name) = string_list.string(selected_idx) {
                            if let Some(ref run) = *cb_m.borrow() {
                                // Detect active mute state by checking if the button currently has the primary color class assignment
                                let is_currently_muted = mute_btn_clone.has_css_class("primary");

                                // Send the name and the inverse of current state to execute the toggle action
                                run(device_name.to_string(), !is_currently_muted);
                            }
                        }
                    }
                }
            }
        });
    }

    pub fn widget(&self) -> &gtk::Box { &self.container }

    pub fn set_on_default_changed<F: Fn(String) + 'static>(&self, callback: F) { *self.on_default_changed.borrow_mut() = Some(Rc::new(callback)); }
    pub fn set_on_volume_changed<F: Fn(String, f64) + 'static>(&self, callback: F) { *self.on_volume_changed.borrow_mut() = Some(Rc::new(callback)); }
    pub fn set_on_mute_toggled<F: Fn(String, bool) + 'static>(&self, callback: F) { *self.on_mute_toggled.borrow_mut() = Some(Rc::new(callback)); }
}
