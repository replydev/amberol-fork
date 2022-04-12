// SPDX-FileCopyrightText: 2022  Emmanuele Bassi
// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::Cell;

use gtk::{gio, glib, prelude::*, subclass::prelude::*};

use crate::audio::{RepeatMode, Song};

mod imp {
    use glib::{ParamFlags, ParamSpec, ParamSpecEnum, ParamSpecObject, ParamSpecUInt, Value};
    use once_cell::sync::Lazy;

    use super::*;

    #[derive(Debug, Default)]
    pub struct Queue {
        pub store: gio::ListStore,
        pub repeat_mode: Cell<RepeatMode>,
        pub current_pos: Cell<Option<u32>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Queue {
        const NAME: &'static str = "AmberolQueue";
        type Type = super::Queue;
        type ParentType = glib::Object;

        fn new() -> Self {
            Self {
                store: gio::ListStore::new(Song::static_type()),
                repeat_mode: Cell::new(RepeatMode::default()),
                current_pos: Cell::new(None),
            }
        }
    }

    impl ObjectImpl for Queue {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecObject::new(
                        "current",
                        "",
                        "",
                        Song::static_type(),
                        ParamFlags::READABLE,
                    ),
                    ParamSpecEnum::new(
                        "repeat-mode",
                        "",
                        "",
                        RepeatMode::static_type(),
                        0,
                        ParamFlags::READABLE,
                    ),
                    ParamSpecUInt::new("n-songs", "", "", 0, u32::MAX, 0, ParamFlags::READABLE),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "current" => obj.current_song().to_value(),
                "repeat-mode" => self.repeat_mode.get().to_value(),
                "n-songs" => self.store.n_items().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub struct Queue(ObjectSubclass<imp::Queue>);
}

impl Default for Queue {
    fn default() -> Self {
        glib::Object::new::<Self>(&[]).expect("Failed to create Queue object")
    }
}

impl Queue {
    fn imp(&self) -> &imp::Queue {
        imp::Queue::from_instance(self)
    }

    pub fn n_songs(&self) -> u32 {
        self.imp().store.n_items()
    }

    pub fn model(&self) -> &gio::ListStore {
        self.imp().store.as_ref()
    }

    pub fn song_at(&self, pos: u32) -> Option<Song> {
        if let Some(song) = self.imp().store.item(pos) {
            return Some(song.downcast::<Song>().unwrap());
        }

        None
    }

    pub fn current_song(&self) -> Option<Song> {
        if let Some(pos) = self.imp().current_pos.get() {
            return self.song_at(pos);
        }

        None
    }

    pub fn current_song_index(&self) -> Option<u32> {
        self.imp().current_pos.get()
    }

    pub fn add_song(&self, song: &Song) {
        self.imp().store.append(song);
        self.notify("n-songs");
    }

    pub fn clear(&self) {
        self.imp().store.remove_all();
        self.notify("n-songs");
    }

    pub fn skip_song(&self, pos: u32) -> Option<Song> {
        self.imp().current_pos.replace(Some(pos));
        self.notify("current");
        self.song_at(pos)
    }

    pub fn previous_song(&self) -> Option<Song> {
        if let Some(current_pos) = self.imp().current_pos.get() {
            if current_pos > 0 {
                let prev = current_pos - 1;
                self.imp().current_pos.replace(Some(prev));
                self.notify("current");
                return self.song_at(current_pos - 1);
            }
        }

        None
    }

    pub fn next_song(&self) -> Option<Song> {
        let store = &self.imp().store;

        let n_songs = store.n_items();
        if n_songs == 0 {
            return None;
        }

        let repeat_mode = self.imp().repeat_mode.get();
        if let Some(current) = self.current_song_index() {
            let next: Option<u32> = match repeat_mode {
                RepeatMode::Consecutive if current < n_songs => Some(current + 1),
                RepeatMode::RepeatOne => Some(current),
                RepeatMode::RepeatAll if current < n_songs - 1 => Some(current + 1),
                RepeatMode::RepeatAll if current == n_songs - 1 => Some(0),
                _ => None,
            };

            if let Some(next) = next {
                self.imp().current_pos.replace(Some(next));
                self.notify("current");
                self.song_at(next)
            } else {
                None
            }
        } else {
            // Return the first song
            self.imp().current_pos.replace(Some(0));
            self.notify("current");
            self.song_at(0)
        }
    }

    pub fn repeat_mode(&self) -> RepeatMode {
        self.imp().repeat_mode.get()
    }

    pub fn set_repeat_mode(&self, repeat_mode: RepeatMode) {
        let old_mode = self.imp().repeat_mode.replace(repeat_mode);
        if old_mode != repeat_mode {
            self.notify("repeat-mode");
        }
    }

    pub fn is_first_song(&self) -> bool {
        if let Some(current_pos) = self.imp().current_pos.get() {
            return current_pos == 0;
        }

        false
    }

    pub fn is_last_song(&self) -> bool {
        if let Some(current_pos) = self.imp().current_pos.get() {
            return current_pos == self.imp().store.n_items() - 1;
        }

        false
    }
}
