// SPDX-FileCopyrightText: 2022  Emmanuele Bassi
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{cell::RefCell, sync::Arc};

use glib::{clone, Sender};
use gtk::glib;
use mpris_player::{Metadata, MprisPlayer, OrgMprisMediaPlayer2Player, PlaybackStatus};

use crate::{
    audio::{Controller, PlaybackAction, PlaybackState, Song},
    config::APPLICATION_ID,
};

pub struct MprisController {
    sender: Sender<PlaybackAction>,
    mpris: Arc<MprisPlayer>,

    song: RefCell<Option<Song>>,
}

impl MprisController {
    pub fn new(sender: Sender<PlaybackAction>) -> Self {
        let mpris = MprisPlayer::new(
            APPLICATION_ID.to_string(),
            "Amberol".to_string(),
            APPLICATION_ID.to_string(),
        );

        mpris.set_can_raise(true);
        mpris.set_can_play(false);
        mpris.set_can_pause(true);
        mpris.set_can_seek(false);
        mpris.set_can_go_next(true);
        mpris.set_can_go_previous(true);
        mpris.set_can_set_fullscreen(false);

        let res = Self {
            sender,
            mpris,
            song: RefCell::new(None),
        };

        res.setup_signals();

        res
    }

    fn setup_signals(&self) {
        self.mpris.connect_play_pause(
            clone!(@weak self.mpris as mpris, @strong self.sender as sender => move || {
                match mpris.get_playback_status().unwrap().as_ref() {
                    "Paused" => send!(sender, PlaybackAction::Play),
                    "Stopped" => send!(sender, PlaybackAction::Play),
                    _ => send!(sender, PlaybackAction::Pause),
                };
            }),
        );

        self.mpris
            .connect_play(clone!(@strong self.sender as sender => move || {
                send!(sender, PlaybackAction::Play);
            }));

        self.mpris
            .connect_stop(clone!(@strong self.sender as sender => move || {
                send!(sender, PlaybackAction::Stop);
            }));

        self.mpris
            .connect_pause(clone!(@strong self.sender as sender => move || {
                send!(sender, PlaybackAction::Pause);
            }));

        self.mpris
            .connect_previous(clone!(@strong self.sender as sender => move || {
                send!(sender, PlaybackAction::SkipPrevious);
            }));

        self.mpris
            .connect_next(clone!(@strong self.sender as sender => move || {
                send!(sender, PlaybackAction::SkipNext);
            }));

        self.mpris
            .connect_raise(clone!(@strong self.sender as sender => move || {
                send!(sender, PlaybackAction::Raise);
            }));
    }

    fn update_metadata(&self) {
        let mut metadata = Metadata::new();

        if let Some(song) = self.song.take() {
            metadata.artist = Some(vec![song.artist()]);
            metadata.title = Some(song.title());
            metadata.album = Some(song.album());

            // MPRIS should really support passing a bytes buffer for
            // the cover art, instead of requiring this ridiculous
            // charade
            if let Some(uuid) = song.uuid() {
                let mut cache = glib::user_cache_dir();
                cache.push("amberol");
                cache.push("covers");
                cache.push(format!("{}.png", uuid));

                if let Ok(uri) = glib::filename_to_uri(&cache, None) {
                    metadata.art_url = Some(uri.as_str().to_string());
                }
            }

            self.song.replace(Some(song));
        }

        self.mpris.set_metadata(metadata);
    }
}

impl Controller for MprisController {
    fn set_playback_state(&self, state: &PlaybackState) {
        self.mpris.set_can_play(true);

        match state {
            PlaybackState::Playing => self.mpris.set_playback_status(PlaybackStatus::Playing),
            _ => self.mpris.set_playback_status(PlaybackStatus::Stopped),
        };
    }

    fn set_song(&self, song: &Song) {
        self.song.replace(Some(song.clone()));
        self.update_metadata();
    }
}
