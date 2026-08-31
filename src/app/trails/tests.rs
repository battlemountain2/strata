// SPDX-License-Identifier: GPL-3.0-or-later

use std::{cell::RefCell, io, rc::Rc};

use crate::services::{StoredTrails, TrailStore};

use super::*;

#[derive(Default)]
struct MemoryStore {
    value: RefCell<StoredTrails>,
}

impl TrailStore for MemoryStore {
    fn load(&self) -> io::Result<StoredTrails> {
        Ok(self.value.borrow().clone())
    }

    fn save(&self, trails: &StoredTrails) -> io::Result<()> {
        self.value.replace(trails.clone());
        Ok(())
    }
}

fn manager() -> (Rc<Trails>, Rc<MemoryStore>) {
    let store = Rc::new(MemoryStore::default());
    let trails = Trails::load(store.clone()).expect("load Trails");
    trails
        .ensure_default(Location::local("/home/example"))
        .expect("create default");
    (trails, store)
}

#[test]
fn create_activate_rename_pin_and_close_round_trip() {
    let (trails, store) = manager();
    let created = trails
        .create("Pictures", Location::local("/home/example/Pictures"))
        .expect("create Trail");
    assert_eq!(trails.active_id(), Some(created.clone()));

    assert!(trails.rename(&created, "Photos").expect("rename Trail"));
    assert!(trails.toggle_pinned(&created).expect("pin Trail"));
    assert_eq!(trails.all()[1].name, "Photos");
    assert!(trails.all()[1].pinned);

    let default = trails.all()[0].id.clone();
    let location = trails.activate(&default).expect("activate Trail");
    assert_eq!(location, Some(Location::local("/home/example")));
    assert!(trails.close(&created).expect("close Trail").is_none());
    assert_eq!(store.value.borrow().collection.trails.len(), 1);
}

#[test]
fn closing_the_active_trail_selects_its_neighbor() {
    let (trails, _) = manager();
    let created = trails
        .create("Temporary", Location::local("/tmp"))
        .expect("create Trail");

    let location = trails.close(&created).expect("close active Trail");

    assert_eq!(location, Some(Location::local("/home/example")));
    assert_eq!(trails.all().len(), 1);
}

#[test]
fn navigation_updates_only_the_active_trail() {
    let (trails, _) = manager();
    trails
        .create("Other", Location::local("/tmp"))
        .expect("create Trail");

    trails
        .update_active_location(Location::local("/tmp/nested"))
        .expect("update location");

    assert_eq!(
        trails.all()[0].active_location(),
        Some(&Location::local("/home/example"))
    );
    assert_eq!(
        trails.all()[1].active_location(),
        Some(&Location::local("/tmp/nested"))
    );
}

#[test]
fn the_last_trail_cannot_be_closed() {
    let (trails, _) = manager();
    let only = trails.active_id().expect("active Trail");

    assert!(trails.close(&only).expect("close Trail").is_none());
    assert_eq!(trails.all().len(), 1);
}
