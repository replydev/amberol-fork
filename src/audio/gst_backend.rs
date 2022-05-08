// SPDX-FileCopyrightText: 2022  Emmanuele Bassi
// SPDX-License-Identifier: GPL-3.0-or-later

use glib::{clone, Sender};
use gtk::{glib, prelude::*};
use gtk_macros::send;

use crate::audio::{PlaybackAction, SeekDirection};

pub struct GstBackend {
    sender: Sender<PlaybackAction>,
    gst_player: gst_player::Player,
}

impl GstBackend {
    pub fn new(sender: Sender<PlaybackAction>) -> Self {
        let dispatcher = gst_player::PlayerGMainContextSignalDispatcher::new(None);
        let gst_player = gst_player::Player::new(
            None,
            Some(&dispatcher.upcast::<gst_player::PlayerSignalDispatcher>()),
        );
        gst_player.set_video_track_enabled(false);

        let mut config = gst_player.config();
        config.set_position_update_interval(250);
        gst_player.set_config(config).unwrap();

        let res = Self { sender, gst_player };

        res.setup_signals();

        res
    }

    fn setup_signals(&self) {
        self.gst_player.connect_warning(move |_, warn| {
            warn!("GStreamer warning: {}", warn);
        });

        self.gst_player
            .connect_end_of_stream(clone!(@strong self.sender as sender => move |_| {
                send!(sender, PlaybackAction::PlayNext);
            }));

        self.gst_player.connect_position_updated(
            clone!(@strong self.sender as sender => move |_, clock| {
                if let Some(clock) = clock {
                    send!(sender, PlaybackAction::UpdatePosition(clock.seconds()));
                }
            }),
        );

        self.gst_player.connect_volume_changed(
            clone!(@strong self.sender as sender => move |player| {
                let volume = gst_audio::StreamVolume::convert_volume(
                    gst_audio::StreamVolumeFormat::Linear,
                    gst_audio::StreamVolumeFormat::Cubic,
                    player.volume(),
                );
                send!(sender, PlaybackAction::VolumeChanged(volume));
            }),
        );
    }

    pub fn set_song_uri(&self, uri: Option<&str>) {
        // FIXME: https://gitlab.freedesktop.org/gstreamer/gstreamer/-/issues/1124
        if uri.is_some() {
            self.gst_player.set_uri(uri);
        }
    }

    pub fn seek(&self, position: u64, duration: u64, offset: u64, direction: SeekDirection) {
        let offset = gst::ClockTime::from_seconds(offset);
        let position = gst::ClockTime::from_seconds(position);
        let duration = gst::ClockTime::from_seconds(duration);

        let destination = match direction {
            SeekDirection::Backwards if position >= offset => position.checked_sub(offset),
            SeekDirection::Backwards if position < offset => Some(gst::ClockTime::from_seconds(0)),
            SeekDirection::Forward if !duration.is_zero() && position + offset <= duration => {
                position.checked_add(offset)
            }
            SeekDirection::Forward if !duration.is_zero() && position + offset > duration => {
                Some(duration)
            }
            _ => None,
        };

        if let Some(destination) = destination {
            self.gst_player.seek(destination);
        }
    }

    pub fn seek_position(&self, position: u64) {
        self.gst_player.seek(gst::ClockTime::from_seconds(position));
    }

    pub fn seek_start(&self) {
        self.gst_player.seek(gst::ClockTime::from_seconds(0));
    }

    pub fn play(&self) {
        self.gst_player.play();
    }

    pub fn pause(&self) {
        self.gst_player.pause();
    }

    pub fn stop(&self) {
        self.gst_player.stop();
    }

    pub fn set_volume(&self, volume: f64) {
        let linear_volume = gst_audio::StreamVolume::convert_volume(
            gst_audio::StreamVolumeFormat::Cubic,
            gst_audio::StreamVolumeFormat::Linear,
            volume,
        );
        debug!("Setting volume to: {}", &linear_volume);
        self.gst_player.set_volume(linear_volume);
    }
}
