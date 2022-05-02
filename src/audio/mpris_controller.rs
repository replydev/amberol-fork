// SPDX-FileCopyrightText: 2022  Emmanuele Bassi
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{cell::RefCell, sync::Arc, time::Duration};

use glib::{clone, Sender};
use gtk::glib;
use mpris_player::{LoopStatus, Metadata, MprisPlayer, OrgMprisMediaPlayer2Player, PlaybackStatus};

use crate::{
    audio::{Controller, PlaybackAction, PlaybackState, RepeatMode, Song},
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
        mpris.set_can_seek(true);
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

        self.mpris
            .connect_loop_status(clone!(@strong self.sender as sender => move |status| {
                let mode = match status {
                    LoopStatus::None => RepeatMode::Consecutive,
                    LoopStatus::Track => RepeatMode::RepeatOne,
                    LoopStatus::Playlist => RepeatMode::RepeatAll,
                };
                send!(sender, PlaybackAction::Repeat(mode));
            }));

        self.mpris
            .connect_seek(clone!(@strong self.sender as sender => move |position| {
                let pos = Duration::from_micros(position as u64).as_secs();
                send!(sender, PlaybackAction::Seek(pos));
            }));
    }

    fn update_metadata(&self) {
        let mut metadata = Metadata::new();

        if let Some(song) = self.song.take() {
            metadata.artist = Some(vec![song.artist()]);
            metadata.title = Some(song.title());
            metadata.album = Some(song.album());

            let length = Duration::from_secs(song.duration()).as_micros() as i64;
            metadata.length = Some(length);

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

    fn set_position(&self, position: u64) {
        let msecs = Duration::from_secs(position).as_micros();
        self.mpris.set_position(msecs as i64);
    }

    fn set_repeat_mode(&self, repeat: RepeatMode) {
        match repeat {
            RepeatMode::Consecutive => self.mpris.set_loop_status(LoopStatus::None),
            RepeatMode::RepeatOne => self.mpris.set_loop_status(LoopStatus::Track),
            RepeatMode::RepeatAll => self.mpris.set_loop_status(LoopStatus::Playlist),
        }
    }
}
