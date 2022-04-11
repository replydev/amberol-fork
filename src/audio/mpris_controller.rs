// SPDX-FileCopyrightText: 2022  Emmanuele Bassi
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{cell::Cell, sync::Arc};

use glib::{clone, Sender};
use gtk::glib;
use mpris_player::{Metadata, MprisPlayer, OrgMprisMediaPlayer2Player, PlaybackStatus};

use crate::{
    audio::{Controller, PlaybackAction, PlaybackState},
    config::APPLICATION_ID,
};

pub struct MprisController {
    sender: Sender<PlaybackAction>,
    mpris: Arc<MprisPlayer>,

    song_title: Cell<Option<String>>,
    song_artist: Cell<Option<String>>,
    song_album: Cell<Option<String>>,
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
            song_title: Cell::new(None),
            song_artist: Cell::new(None),
            song_album: Cell::new(None),
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

        let artist = self.song_artist.take();
        let title = self.song_title.take();
        let album = self.song_album.take();

        if let Some(artist) = artist.clone() {
            metadata.artist = Some(vec![artist]);
        }

        if let Some(title) = title.clone() {
            metadata.title = Some(title);
        }

        if let Some(album) = album.clone() {
            metadata.album = Some(album);
        }

        self.song_artist.set(artist);
        self.song_title.set(title);
        self.song_album.set(album);

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

    fn set_song_artist(&self, artist: &str) {
        self.song_artist.set(Some(artist.to_string()));
        self.update_metadata();
    }

    fn set_song_title(&self, title: &str) {
        self.song_title.set(Some(title.to_string()));
        self.update_metadata();
    }

    fn set_song_album(&self, album: &str) {
        self.song_album.set(Some(album.to_string()));
        self.update_metadata();
    }
}
