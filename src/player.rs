// SPDX-FileCopyrightText: 2022  Emmanuele Bassi
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    cell::{Cell, RefCell},
    ops::Deref,
    rc::Rc,
    sync::Arc,
};

use fragile::Fragile;
use glib::{
    clone, ParamFlags, ParamSpec, ParamSpecBoolean, ParamSpecEnum, ParamSpecObject,
    ParamSpecString, ParamSpecUInt, ParamSpecUInt64,
};
use gtk::{gdk, gio, glib, prelude::*, subclass::prelude::*};
use mpris_player::{Metadata, MprisPlayer, OrgMprisMediaPlayer2Player, PlaybackStatus};
use once_cell::sync::Lazy;

use crate::{config::APPLICATION_ID, song::Song, utils::format_time};

#[derive(Clone, Copy, Debug, glib::Enum)]
#[enum_type(name = "AmberolRepeatMode")]
pub enum RepeatMode {
    Consecutive,
    RepeatAll,
    RepeatOne,
}

#[derive(Debug)]
enum SeekDirection {
    Forward,
    Backwards,
}

impl Default for RepeatMode {
    fn default() -> Self {
        RepeatMode::Consecutive
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct PlayerState {
        pub playing: Cell<bool>,
        pub current_song: Cell<u32>,
        pub n_songs: Cell<u32>,
        pub duration: Cell<u64>,
        pub position: Cell<u64>,
        pub repeat: Cell<RepeatMode>,
        pub song: RefCell<Option<Song>>,
        pub current_cover: RefCell<Option<gdk::Texture>>,
        pub queue: gio::ListStore,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlayerState {
        const NAME: &'static str = "AmberolPlayerState";
        type Type = super::PlayerState;
        type ParentType = glib::Object;

        fn new() -> Self {
            Self {
                playing: Cell::new(false),
                current_song: Cell::new(0),
                n_songs: Cell::new(0),
                duration: Cell::new(0),
                position: Cell::new(0),
                repeat: Cell::new(RepeatMode::Consecutive),
                song: RefCell::new(None),
                current_cover: RefCell::new(None),
                queue: gio::ListStore::new(Song::static_type()),
            }
        }
    }

    impl ObjectImpl for PlayerState {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecBoolean::new("playing", "", "", false, ParamFlags::READABLE),
                    ParamSpecUInt::new(
                        "current-song",
                        "",
                        "",
                        0,
                        u32::MAX,
                        0,
                        ParamFlags::READABLE,
                    ),
                    ParamSpecUInt::new("n-songs", "", "", 0, u32::MAX, 0, ParamFlags::READABLE),
                    ParamSpecUInt64::new("duration", "", "", 0, u64::MAX, 0, ParamFlags::READABLE),
                    ParamSpecUInt64::new("position", "", "", 0, u64::MAX, 0, ParamFlags::READABLE),
                    ParamSpecString::new("current-title", "", "", None, ParamFlags::READABLE),
                    ParamSpecString::new("current-artist", "", "", None, ParamFlags::READABLE),
                    ParamSpecString::new("current-album", "", "", None, ParamFlags::READABLE),
                    ParamSpecString::new("current-time", "", "", None, ParamFlags::READABLE),
                    ParamSpecObject::new(
                        "current-cover",
                        "",
                        "",
                        gdk::Texture::static_type(),
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
                ]
            });
            PROPERTIES.as_ref()
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "playing" => self.playing.get().to_value(),
                "current-song" => self.current_song.get().to_value(),
                "n-songs" => _obj.n_songs().to_value(),
                "duration" => _obj.duration().to_value(),
                "position" => self.position.get().to_value(),
                "current-title" => _obj.current_title().to_value(),
                "current-artist" => _obj.current_artist().to_value(),
                "current-album" => _obj.current_album().to_value(),
                "current-time" => _obj.current_time().to_value(),
                "current-cover" => self.current_cover.borrow().to_value(),
                "repeat-mode" => _obj.repeat_mode().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

// PlayerState is a GObject that we can use to bind to
// widgets and other objects; it contains the current
// state of the audio player: song metadata, playback
// position and duration, etc.
glib::wrapper! {
    pub struct PlayerState(ObjectSubclass<imp::PlayerState>);
}

impl PlayerState {
    fn imp(&self) -> &imp::PlayerState {
        imp::PlayerState::from_instance(self)
    }

    pub fn queue(&self) -> &gio::ListStore {
        self.imp().queue.as_ref()
    }

    pub fn playing(&self) -> bool {
        self.imp().playing.get()
    }

    pub fn current_song(&self) -> u32 {
        self.imp().current_song.get()
    }

    pub fn n_songs(&self) -> u32 {
        self.imp().queue.n_items()
    }

    pub fn position(&self) -> u64 {
        self.imp().position.get()
    }

    pub fn current_artist(&self) -> Option<String> {
        let res = match &*self.imp().song.borrow() {
            Some(s) => Some(s.artist()),
            None => None,
        };

        res
    }

    pub fn current_title(&self) -> Option<String> {
        let res = match &*self.imp().song.borrow() {
            Some(s) => Some(s.title()),
            None => None,
        };

        res
    }

    pub fn current_album(&self) -> Option<String> {
        let res = match &*self.imp().song.borrow() {
            Some(s) => Some(s.album()),
            None => None,
        };

        res
    }

    pub fn duration(&self) -> u64 {
        match &*self.imp().song.borrow() {
            Some(s) => s.duration(),
            None => 0,
        }
    }

    pub fn current_time(&self) -> String {
        format_time(self.position(), self.duration())
    }

    pub fn song(&self) -> Option<Song> {
        match &*self.imp().song.borrow() {
            Some(s) => Some(s.clone()),
            None => None,
        }
    }

    pub fn song_at(&self, pos: u32) -> Song {
        self.imp()
            .queue
            .item(pos)
            .unwrap()
            .downcast::<Song>()
            .unwrap()
    }

    fn set_song(&self, song: Option<Song>) {
        self.imp().song.replace(song);
        self.notify("current-artist");
        self.notify("current-title");
        self.notify("current-album");
        self.notify("duration");
        self.notify("position");
    }

    fn set_playing(&self, playing: bool) {
        self.imp().playing.set(playing);
        self.notify("playing");
    }

    fn set_current_song(&self, pos: u32) {
        self.imp().current_song.set(pos);
        self.notify("current-song");
    }

    fn set_position(&self, position: u64) {
        self.imp().position.set(position);
        self.notify("position");
        self.notify("current-time");
    }

    fn set_current_cover(&self, buffer: Option<glib::Bytes>) {
        let imp = self.imp();
        if let Some(buffer) = buffer {
            let texture = match gdk::Texture::from_bytes(&buffer) {
                Ok(t) => Some(t),
                Err(_) => None,
            };

            match texture {
                Some(image) => imp.current_cover.replace(Some(image)),
                None => imp.current_cover.replace(None),
            };
        } else {
            warn!("No cover art found for current song");
            imp.current_cover.replace(None);
        }
        self.notify("current-cover");
    }

    fn toggle_repeat(&self) {
        let repeat_mode = self.imp().repeat.get();
        let new_repeat_mode = match repeat_mode {
            RepeatMode::Consecutive => RepeatMode::RepeatAll,
            RepeatMode::RepeatAll => RepeatMode::RepeatOne,
            RepeatMode::RepeatOne => RepeatMode::Consecutive,
        };
        self.imp().repeat.set(new_repeat_mode);
        self.notify("repeat-mode");
    }

    pub fn repeat_mode(&self) -> RepeatMode {
        self.imp().repeat.get()
    }
}

impl Default for PlayerState {
    fn default() -> Self {
        glib::Object::new::<Self>(&[]).expect("Unable to create PlayerState instance")
    }
}

// AudioPlayer is our main interface for the audio playback; it
// handles the songs queue, the player state object, the GStreamer
// player API, and MPRIS
#[derive(Debug)]
pub struct AudioPlayer {
    gst_player: gst_player::Player,
    mpris_player: Arc<MprisPlayer>,
    state: PlayerState,
}

impl Default for AudioPlayer {
    fn default() -> Self {
        let dispatcher = gst_player::PlayerGMainContextSignalDispatcher::new(None);
        let gst_player = gst_player::Player::new(
            None,
            Some(&dispatcher.upcast::<gst_player::PlayerSignalDispatcher>()),
        );
        gst_player.set_video_track_enabled(false);

        let mpris_player = MprisPlayer::new(
            APPLICATION_ID.to_string(),
            "Amberol".to_string(),
            APPLICATION_ID.to_string(),
        );
        mpris_player.set_can_raise(true);
        mpris_player.set_can_play(true);
        mpris_player.set_can_pause(true);
        mpris_player.set_can_seek(false);
        mpris_player.set_can_set_fullscreen(false);

        let mut config = gst_player.config();
        config.set_position_update_interval(250);
        gst_player.set_config(config).unwrap();

        AudioPlayer {
            gst_player,
            mpris_player,
            state: PlayerState::default(),
        }
    }
}

impl AudioPlayer {
    pub fn queue_song(&self, song: &Song) {
        let default_song = Song::default();
        if song.equals(&default_song) {
            warn!("Invalid song data, skipping...");
            return;
        }

        self.state.queue().append(song);
        if self.state.queue().n_items() == 1 {
            self.init_current_song();
        }
        self.state.notify("n-songs");
    }

    pub fn queue(&self) -> &gio::ListStore {
        self.state.queue()
    }

    pub fn queue_clear(&self) {
        self.state.queue().remove_all();
        self.reset_current_song();
        self.state.notify("n-songs");
    }

    pub fn current_song(&self) -> u32 {
        match self.state.song() {
            None => return 0,
            Some(song) => {
                let n_songs = self.state.n_songs();
                for pos in 0..n_songs {
                    let other = self.state.song_at(pos);
                    if song.equals(&other) {
                        return pos;
                    }
                }
            }
        }

        0
    }

    pub fn reset_current_song(&self) {
        self.state.set_song(None);
        self.state.set_current_song(0);
        self.state.set_current_cover(None);
        self.state.set_position(0);

        // FIXME: https://gitlab.freedesktop.org/gstreamer/gstreamer/-/issues/1124
        // self.gst_player.set_uri(None);

        self.mpris_player.set_can_go_previous(false);
        self.mpris_player.set_can_go_next(false);
        self.mpris_player.set_can_play(false);
    }

    pub fn init_current_song(&self) {
        let current_pos = self.state.current_song();
        let current_song = self.state.song_at(current_pos);
        let n_songs = self.state.n_songs();

        // Do not replace the song; we can toggle between play and pause states
        // without changing the rest of the UI
        if self
            .gst_player
            .uri()
            .map_or(true, |s| s != current_song.uri().as_str())
        {
            self.state.set_song(Some(current_song.clone()));
            self.state.set_position(0);
            self.state.set_current_cover(current_song.cover_art());

            self.gst_player.set_uri(Some(&current_song.uri()));
        }

        self.mpris_player.set_can_play(true);
        self.mpris_player.set_can_go_next(current_pos < n_songs - 1);
        self.mpris_player.set_can_go_previous(current_pos > 0);

        let mut metadata = Metadata::new();
        metadata.artist = Some(vec![current_song.artist()]);
        metadata.title = Some(current_song.title());
        self.mpris_player.set_metadata(metadata);

        for pos in 0..self.state.n_songs() {
            let s = self.state.song_at(pos);
            if pos == current_pos {
                s.set_playing(true);
            } else {
                s.set_playing(false);
            }
        }
    }

    pub fn play(&self) {
        if self.state.n_songs() == 0 {
            debug!("Empty songs queue");
            return;
        }

        if !self.state.playing() {
            debug!("Playing {} song", self.current_song());
            self.state.set_playing(true);
            self.init_current_song();

            self.gst_player.play();

            self.mpris_player.set_can_pause(true);
            self.mpris_player
                .set_playback_status(PlaybackStatus::Playing);
        }
    }

    pub fn pause(&self) {
        if self.state.playing() {
            debug!("Paused {} song", self.current_song());
            self.state.set_playing(false);

            self.gst_player.pause();

            self.mpris_player.set_can_pause(false);
            self.mpris_player
                .set_playback_status(PlaybackStatus::Paused);
        }
    }

    pub fn stop(&self) {
        if self.state.playing() {
            debug!("Stopped");
            self.state.set_playing(false);
            self.reset_current_song();

            self.gst_player.stop();

            self.mpris_player.set_can_play(true);
            self.mpris_player.set_can_pause(false);
            self.mpris_player
                .set_playback_status(PlaybackStatus::Stopped);
        }
    }

    pub fn skip_next(&self) {
        let was_playing = self.state.playing();
        let current_pos = self.state.current_song();
        if current_pos == self.state.n_songs() - 1 {
            debug!("Reached the end of the queue");
            self.stop();
            return;
        } else {
            debug!(
                "Skipping to the next song in the queue: {}",
                current_pos + 1
            );
            if was_playing {
                self.pause();
            }
            self.state.set_current_song(current_pos + 1);
            if was_playing {
                self.play();
            } else {
                self.init_current_song();
            }
        }
    }

    pub fn skip_previous(&self) {
        let was_playing = self.state.playing();
        let current_pos = self.state.current_song();
        // If we are more than a backward seek step, then
        // we rewind to the start of the song instead
        if self.state.position() >= 10 {
            let offset = gst::ClockTime::from_seconds(self.state.position() + 1);
            self.seek(offset, SeekDirection::Backwards);
            return;
        }
        if current_pos == 0 {
            debug!("Reached the start of the queue");
            return;
        } else {
            debug!(
                "Skipping to the previous song in the queue: {}",
                current_pos - 1
            );
            if was_playing {
                self.pause();
            }
            self.state.set_current_song(current_pos - 1);
            if was_playing {
                self.play();
            } else {
                self.init_current_song();
            }
        }
    }

    pub fn skip_to(&self, pos: u32) {
        if pos >= self.state.n_songs() {
            debug!("Out of bounds");
            return;
        }

        let was_playing = self.state.playing();
        let current_pos = self.state.current_song();
        if was_playing {
            self.pause();
        }
        if pos != current_pos {
            self.state.set_current_song(pos);
        } else {
            self.state.set_position(0);
        }
        if was_playing {
            self.play();
        } else {
            self.init_current_song();
        }
    }

    fn seek(&self, offset: gst::ClockTime, direction: SeekDirection) {
        let position = gst::ClockTime::from_seconds(self.state.position());
        let duration = gst::ClockTime::from_seconds(self.state.duration());
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

    pub fn seek_forward(&self) {
        self.seek(gst::ClockTime::from_seconds(10), SeekDirection::Forward);
    }

    pub fn seek_backwards(&self) {
        self.seek(gst::ClockTime::from_seconds(10), SeekDirection::Backwards);
    }

    pub fn state(&self) -> &PlayerState {
        &self.state
    }

    pub fn advance_position(&self, clock: gst::ClockTime) {
        self.state.set_position(clock.seconds());
    }

    pub fn toggle_queue_repeat(&self) {
        self.state.toggle_repeat();
    }
}

// AudioPlayerWrapper is a reference counted wrapper around
// AudioPlayer that we can send around to GStreamer signals

#[derive(Debug, Clone)]
pub struct AudioPlayerWrapper(pub Rc<RefCell<AudioPlayer>>);

impl Default for AudioPlayerWrapper {
    fn default() -> Self {
        AudioPlayerWrapper(Rc::new(RefCell::new(AudioPlayer::default())))
    }
}

impl Deref for AudioPlayerWrapper {
    type Target = Rc<RefCell<AudioPlayer>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AudioPlayerWrapper {
    // We don't have mutable users at the moment
    // pub(crate) fn borrow_mut(&self) -> RefMut<'_, AudioPlayer> {
    //    self.0.borrow_mut()
    //}

    pub(crate) fn new() -> Self {
        let p = AudioPlayerWrapper::default();
        p.init();
        p
    }

    // This is the core logic to handle GstPlayer
    fn init(&self) {
        let player = self.borrow();

        player.gst_player.connect_warning(move |_, warn| {
            warn!("GStreamer warning: {}", warn);
        });

        // We use Fragile here because the GstPlayer signals happen
        // in the GTK main context, so we don't need to go too deep
        // into the woods of thread safety
        let weak = Fragile::new(Rc::downgrade(self));

        player
            .gst_player
            .connect_end_of_stream(clone!(@strong weak => move |_| {
                debug!("Stream ended");
                if let Some(p) = weak.get().upgrade() {
                    let audio_player = p.borrow();
                    let current = audio_player.state().current_song();
                    let n_songs = audio_player.state().n_songs();

                    // Determine what to do next depending on the repeat mode
                    // and whether we reached the end of the queue
                    match audio_player.state().repeat_mode() {
                        RepeatMode::Consecutive => {
                            if current < n_songs - 1 {
                                audio_player.skip_next();
                            } else {
                                audio_player.stop();
                            }
                        },
                        RepeatMode::RepeatAll => {
                            if current < n_songs - 1 {
                                audio_player.skip_next();
                            } else {
                                audio_player.skip_to(0);
                            }
                        },
                        RepeatMode::RepeatOne => {
                            audio_player.pause();
                            audio_player.advance_position(gst::ClockTime::from_seconds(0));
                            audio_player.play();
                        },
                    };
                }
            }));
        player
            .gst_player
            .connect_position_updated(clone!(@strong weak => move |_, clock| {
                if let Some(audio_player) = weak.get().upgrade() {
                    if let Some(clock) = clock {
                        audio_player.borrow().advance_position(clock);
                    }
                }
            }));

        player
            .mpris_player
            .connect_play_pause(clone!(@strong weak => move || {
                debug!("MPRIS play/pause");
                if let Some(audio_player) = weak.get().upgrade() {
                    let p = audio_player.borrow();
                    match p.mpris_player.get_playback_status().unwrap().as_ref() {
                        "Paused" => p.play(),
                        "Stopped" => p.play(),
                        _ => p.pause(),
                    }
                }
            }));
        player
            .mpris_player
            .connect_play(clone!(@strong weak => move || {
                debug!("MPRIS play");
                if let Some(audio_player) = weak.get().upgrade() {
                    audio_player.borrow().play();
                }
            }));
        player
            .mpris_player
            .connect_pause(clone!(@strong weak => move || {
                debug!("MPRIS pause");
                if let Some(audio_player) = weak.get().upgrade() {
                    audio_player.borrow().pause();
                }
            }));
        player
            .mpris_player
            .connect_stop(clone!(@strong weak => move || {
                debug!("MPRIS stop");
                if let Some(audio_player) = weak.get().upgrade() {
                    audio_player.borrow().stop();
                }
            }));
        player
            .mpris_player
            .connect_previous(clone!(@strong weak => move || {
                debug!("MPRIS previous");
                if let Some(audio_player) = weak.get().upgrade() {
                    audio_player.borrow().skip_previous();
                }
            }));
        player
            .mpris_player
            .connect_next(clone!(@strong weak => move || {
                debug!("MPRIS next");
                if let Some(audio_player) = weak.get().upgrade() {
                    audio_player.borrow().skip_next();
                }
            }));
        player.mpris_player.connect_raise(move || {
            let app = gio::Application::default()
                .expect("Failed to retrieve application singleton")
                .downcast::<gtk::Application>()
                .unwrap();
            let win = app
                .active_window()
                .unwrap()
                .downcast::<gtk::Window>()
                .unwrap();
            win.present();
        });
    }
}
