// SPDX-FileCopyrightText: 2022  Emmanuele Bassi
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::audio::PlaybackState;

pub trait Controller {
    fn set_playback_state(&self, state: &PlaybackState);

    fn set_song_artist(&self, artist: &str);
    fn set_song_title(&self, title: &str);
    fn set_song_album(&self, album: &str);
}
