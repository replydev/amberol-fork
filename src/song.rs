// SPDX-FileCopyrightText: 2022  Emmanuele Bassi
// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::{Cell, RefCell};

use glib::{
    ParamFlags, ParamSpec, ParamSpecBoolean, ParamSpecObject, ParamSpecString, ParamSpecUInt, Value,
};
use gtk::{gdk, gio, glib, prelude::*, subclass::prelude::*};
use lofty::Accessor;
use once_cell::sync::Lazy;

use crate::{i18n::i18n, utils};

#[derive(Debug, Clone)]
pub struct SongData {
    artist: Option<String>,
    title: Option<String>,
    album: Option<String>,
    cover_texture: Option<gdk::Texture>,
    cover_color: Option<gdk::RGBA>,
    duration: u64,
    file: gio::File,
}

impl SongData {
    pub fn artist(&self) -> Option<&str> {
        self.artist.as_ref().map(|s| s.as_str())
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|s| s.as_str())
    }

    pub fn album(&self) -> Option<&str> {
        self.album.as_ref().map(|s| s.as_str())
    }

    pub fn duration(&self) -> u64 {
        self.duration
    }

    pub fn cover_texture(&self) -> Option<&gdk::Texture> {
        self.cover_texture.as_ref()
    }

    pub fn cover_color(&self) -> Option<&gdk::RGBA> {
        self.cover_color.as_ref()
    }

    pub fn from_uri(uri: &str) -> Self {
        let file = gio::File::for_uri(uri);
        let path = file.path().expect("Unable to find file");

        let tagged_file = match lofty::read_from_path(&path, true) {
            Ok(f) => f,
            Err(e) => {
                warn!("Unable to open file {:?}: {}", path, e);
                return SongData::default();
            }
        };

        let mut artist = None;
        let mut title = None;
        let mut album = None;
        let mut cover_art = None;
        if let Some(tag) = tagged_file.primary_tag() {
            artist = tag.artist().map(|s| s.to_string());
            title = tag.title().map(|s| s.to_string());
            album = tag.album().map(|s| s.to_string());
            for picture in tag.pictures() {
                cover_art = match picture.mime_type() {
                    lofty::MimeType::Png => Some(glib::Bytes::from(picture.data())),
                    lofty::MimeType::Jpeg => Some(glib::Bytes::from(picture.data())),
                    lofty::MimeType::Tiff => Some(glib::Bytes::from(picture.data())),
                    _ => None,
                };
                // Stop at the first cover we find
                if cover_art.is_some() {
                    break;
                }
            }
        } else {
            warn!("Unable to load tags for {}", uri);
        };

        let mut cover_texture = None;
        let mut cover_color = None;
        if let Some(ref cover_art) = cover_art {
            cover_texture = utils::load_cover_texture(&cover_art);
            cover_color = utils::load_dominant_color(&cover_art);
        }

        let duration = tagged_file.properties().duration().as_secs();

        SongData {
            artist,
            title,
            album,
            cover_texture,
            cover_color,
            duration,
            file,
        }
    }

    pub fn uri(&self) -> String {
        self.file.uri().to_string()
    }
}

impl Default for SongData {
    fn default() -> Self {
        SongData {
            artist: Some("Invalid Artist".to_string()),
            title: Some("Invalid Title".to_string()),
            album: Some("Invalid Album".to_string()),
            cover_texture: None,
            cover_color: None,
            duration: 0,
            file: gio::File::for_path("/does-not-exist"),
        }
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct Song {
        pub data: RefCell<SongData>,
        pub playing: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Song {
        const NAME: &'static str = "AmberolSong";
        type Type = super::Song;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for Song {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecString::new(
                        "uri",
                        "",
                        "",
                        None,
                        ParamFlags::READWRITE | ParamFlags::CONSTRUCT_ONLY,
                    ),
                    ParamSpecString::new("artist", "", "", None, ParamFlags::READABLE),
                    ParamSpecString::new("title", "", "", None, ParamFlags::READABLE),
                    ParamSpecString::new("album", "", "", None, ParamFlags::READABLE),
                    ParamSpecUInt::new("duration", "", "", 0, u32::MAX, 0, ParamFlags::READABLE),
                    ParamSpecObject::new(
                        "cover",
                        "",
                        "",
                        gdk::Texture::static_type(),
                        ParamFlags::READABLE,
                    ),
                    ParamSpecBoolean::new("playing", "", "", false, ParamFlags::READWRITE),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _obj: &Self::Type, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "uri" => {
                    if let Ok(p) = value.get::<&str>() {
                        self.data.replace(SongData::from_uri(p));
                        _obj.notify("artist");
                        _obj.notify("title");
                        _obj.notify("album");
                        _obj.notify("duration");
                    }
                }
                "playing" => {
                    let p = value.get::<bool>().expect("Value must be a boolean");
                    self.playing.set(p);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "artist" => self.data.borrow().artist().to_value(),
                "title" => self.data.borrow().title().to_value(),
                "album" => self.data.borrow().album().to_value(),
                "duration" => self.data.borrow().duration().to_value(),
                "uri" => self.data.borrow().uri().to_value(),
                "cover" => self.data.borrow().cover_texture().to_value(),
                "playing" => self.playing.get().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub struct Song(ObjectSubclass<imp::Song>);
}

impl Song {
    pub fn new(uri: &str) -> Self {
        glib::Object::new::<Self>(&[("uri", &uri)]).expect("Failed to create Song object")
    }

    pub fn empty() -> Self {
        glib::Object::new::<Self>(&[]).expect("Failed to create an empty Song object")
    }

    pub fn equals(&self, other: &Self) -> bool {
        self.uri() == other.uri()
    }

    fn imp(&self) -> &imp::Song {
        imp::Song::from_instance(self)
    }

    pub fn uri(&self) -> String {
        self.imp().data.borrow().uri()
    }

    pub fn artist(&self) -> String {
        match self.imp().data.borrow().artist() {
            Some(artist) => return artist.to_string(),
            None => return i18n("Unknown artist").to_string(),
        }
    }

    pub fn title(&self) -> String {
        match self.imp().data.borrow().title() {
            Some(title) => return title.to_string(),
            None => return i18n("Unknown title").to_string(),
        }
    }

    pub fn album(&self) -> String {
        match self.imp().data.borrow().album() {
            Some(album) => return album.to_string(),
            None => return i18n("Unknown album").to_string(),
        }
    }

    pub fn cover_texture(&self) -> Option<gdk::Texture> {
        match self.imp().data.borrow().cover_texture() {
            Some(texture) => Some(texture.clone()),
            None => None,
        }
    }

    pub fn cover_color(&self) -> Option<gdk::RGBA> {
        match self.imp().data.borrow().cover_color() {
            Some(color) => Some(color.clone()),
            None => None,
        }
    }

    pub fn duration(&self) -> u64 {
        self.imp().data.borrow().duration()
    }

    pub fn playing(&self) -> bool {
        self.imp().playing.get()
    }

    pub fn set_playing(&self, playing: bool) {
        let was_playing = self.imp().playing.replace(playing);
        if was_playing != playing {
            self.notify("playing");
        }
    }
}

impl Default for Song {
    fn default() -> Self {
        Self::empty()
    }
}
