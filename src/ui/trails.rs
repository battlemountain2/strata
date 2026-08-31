// SPDX-License-Identifier: GPL-3.0-or-later

use std::rc::Rc;

use gtk::prelude::*;

use crate::{
    app::{Browser, Trails},
    model::Trail,
};

pub struct TrailSwitcher {
    button: gtk::MenuButton,
    title: gtk::Label,
    popover: gtk::Popover,
    list: gtk::Box,
    trails: Rc<Trails>,
    browser: Rc<Browser>,
}

impl TrailSwitcher {
    pub fn new(trails: Rc<Trails>, browser: Rc<Browser>) -> Rc<Self> {
        let title = gtk::Label::new(None);
        title.set_ellipsize(gtk::pango::EllipsizeMode::End);
        title.set_max_width_chars(18);
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 7);
        content.append(&crate::assets::text_icon(crate::assets::icons::ROWS, 16));
        content.append(&title);

        let button = gtk::MenuButton::builder()
            .child(&content)
            .tooltip_text("Switch Trails (Ctrl+Shift+T)")
            .build();
        button.add_css_class("trail-switcher-button");

        let list = gtk::Box::new(gtk::Orientation::Vertical, 4);
        list.add_css_class("trail-switcher-list");
        let popover = gtk::Popover::builder().child(&list).build();
        popover.add_css_class("trail-switcher-popover");
        button.set_popover(Some(&popover));

        let switcher = Rc::new(Self {
            button,
            title,
            popover,
            list,
            trails,
            browser,
        });
        switcher.refresh();
        switcher
    }

    pub fn widget(&self) -> gtk::MenuButton {
        self.button.clone()
    }

    pub fn popup(&self) {
        self.popover.popup();
    }

    fn refresh(self: &Rc<Self>) {
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        let trails = self.trails.all();
        let active = self.trails.active_id();
        self.title.set_text(
            trails
                .iter()
                .find(|trail| Some(&trail.id) == active.as_ref())
                .map(|trail| trail.name.as_str())
                .unwrap_or("Trails"),
        );

        let heading = gtk::Label::new(Some("TRAILS"));
        heading.set_xalign(0.0);
        heading.add_css_class("trail-switcher-heading");
        self.list.append(&heading);

        for trail in &trails {
            self.list
                .append(&self.trail_row(trail, trails.len(), active.as_ref()));
        }

        let create = gtk::Button::builder()
            .label("New Trail from here")
            .tooltip_text("Save the current browsing context as a new Trail")
            .build();
        create.set_child(Some(&labeled_icon(
            crate::assets::icons::PLUS,
            "New Trail from here",
        )));
        create.add_css_class("trail-create");
        let weak = Rc::downgrade(self);
        create.connect_clicked(move |_| {
            let Some(switcher) = weak.upgrade() else {
                return;
            };
            let Some(location) = switcher.browser.active_location() else {
                return;
            };
            let name = location.display_name();
            if let Err(error) = switcher.trails.create(name, location) {
                tracing::warn!(%error, "unable to create Trail");
            }
            switcher.refresh();
        });
        self.list.append(&create);
    }

    fn trail_row(
        self: &Rc<Self>,
        trail: &Trail,
        count: usize,
        active: Option<&crate::model::TrailId>,
    ) -> gtk::Box {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        row.add_css_class("trail-row");
        let is_active = active == Some(&trail.id);
        if is_active {
            row.add_css_class("active");
        }

        let select = gtk::Button::with_label(&trail.name);
        select.set_hexpand(true);
        select.set_halign(gtk::Align::Fill);
        select.add_css_class("trail-select");
        select.set_tooltip_text(
            trail
                .active_location()
                .map(|location| location.display_path())
                .as_deref(),
        );
        let id = trail.id.clone();
        let weak = Rc::downgrade(self);
        select.connect_clicked(move |_| {
            let Some(switcher) = weak.upgrade() else {
                return;
            };
            match switcher.trails.activate(&id) {
                Ok(Some(location)) => switcher.browser.navigate(location),
                Ok(None) => {}
                Err(error) => tracing::warn!(%error, "unable to activate Trail"),
            }
            switcher.popover.popdown();
            switcher.refresh();
        });

        let rename = gtk::Entry::builder()
            .text(&trail.name)
            .hexpand(true)
            .visible(false)
            .build();
        rename.add_css_class("trail-rename");
        let rename_id = trail.id.clone();
        let weak = Rc::downgrade(self);
        rename.connect_activate(move |entry| {
            let Some(switcher) = weak.upgrade() else {
                return;
            };
            if let Err(error) = switcher.trails.rename(&rename_id, entry.text()) {
                tracing::warn!(%error, "unable to rename Trail");
            }
            switcher.refresh();
        });
        let rename_keys = gtk::EventControllerKey::new();
        let weak = Rc::downgrade(self);
        rename_keys.connect_key_pressed(move |_, key, _, _| {
            if key != gtk::gdk::Key::Escape {
                return gtk::glib::Propagation::Proceed;
            }
            if let Some(switcher) = weak.upgrade() {
                switcher.refresh();
            }
            gtk::glib::Propagation::Stop
        });
        rename.add_controller(rename_keys);

        let edit = icon_button(crate::assets::icons::PENCIL, "Rename Trail");
        let shown_select = select.clone();
        let shown_rename = rename.clone();
        edit.connect_clicked(move |_| {
            shown_select.set_visible(false);
            shown_rename.set_visible(true);
            shown_rename.grab_focus();
            shown_rename.select_region(0, -1);
        });

        let pin = icon_button(
            crate::assets::icons::PIN,
            if trail.pinned {
                "Unpin Trail"
            } else {
                "Pin Trail"
            },
        );
        if trail.pinned {
            pin.add_css_class("active");
        }
        let pin_id = trail.id.clone();
        let weak = Rc::downgrade(self);
        pin.connect_clicked(move |_| {
            let Some(switcher) = weak.upgrade() else {
                return;
            };
            if let Err(error) = switcher.trails.toggle_pinned(&pin_id) {
                tracing::warn!(%error, "unable to update Trail pin");
            }
            switcher.refresh();
        });

        let close = icon_button(crate::assets::icons::X, "Close Trail");
        close.set_sensitive(count > 1);
        let close_id = trail.id.clone();
        let weak = Rc::downgrade(self);
        close.connect_clicked(move |_| {
            let Some(switcher) = weak.upgrade() else {
                return;
            };
            match switcher.trails.close(&close_id) {
                Ok(Some(location)) => switcher.browser.navigate(location),
                Ok(None) => {}
                Err(error) => tracing::warn!(%error, "unable to close Trail"),
            }
            switcher.refresh();
        });

        row.append(&select);
        row.append(&rename);
        row.append(&edit);
        row.append(&pin);
        row.append(&close);
        row
    }
}

fn icon_button(icon: &str, tooltip: &str) -> gtk::Button {
    let button = gtk::Button::builder().tooltip_text(tooltip).build();
    button.set_child(Some(&crate::assets::text_icon(icon, 14)));
    button.add_css_class("trail-row-action");
    button
}

fn labeled_icon(icon: &str, label: &str) -> gtk::Box {
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 7);
    content.append(&crate::assets::text_icon(icon, 14));
    let text = gtk::Label::new(Some(label));
    text.set_xalign(0.0);
    content.append(&text);
    content
}
