// SPDX-FileCopyrightText: 2022  Emmanuele Bassi
// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::Cell;

use gtk::{gio, prelude::*};

use crate::{
    audio::{Controller, PlaybackState},
    i18n::i18n,
};

#[derive(Debug, Default)]
pub struct InhibitController {
    cookie: Cell<u32>,
}

impl InhibitController {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Controller for InhibitController {
    fn set_playback_state(&self, playback_state: &PlaybackState) {
        let app = gio::Application::default()
            .expect("Failed to retrieve application singleton")
            .downcast::<gtk::Application>()
            .unwrap();
        let win = app
            .active_window()
            .unwrap()
            .downcast::<gtk::Window>()
            .unwrap();

        if playback_state == &PlaybackState::Playing {
            if self.cookie.get() == 0 {
                let cookie = app.inhibit(
                    Some(&win),
                    gtk::ApplicationInhibitFlags::SUSPEND,
                    Some(&i18n("Playback in progress")),
                );
                self.cookie.set(cookie);

                debug!("Suspend inhibited");
            }
        } else {
            let cookie = self.cookie.take();
            if cookie != 0 {
                app.uninhibit(cookie);

                debug!("Suspend uninhibited");
            }
        }
    }

    fn set_song_artist(&self, _artist: &str) {}
    fn set_song_title(&self, _title: &str) {}
    fn set_song_album(&self, _title: &str) {}
}
