// SPDX-FileCopyrightText: 2022  Emmanuele Bassi
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    cell::{Cell, RefCell},
    time::Instant,
};

use glib::{
    ParamFlags, ParamSpec, ParamSpecBoolean, ParamSpecObject, ParamSpecString, ParamSpecUInt, Value,
};
use gtk::{gdk, gio, glib, prelude::*, subclass::prelude::*};
use lofty::Accessor;
use once_cell::sync::Lazy;
use sha2::{Digest, Sha256};

use crate::{i18n::i18n, utils};

#[derive(Debug, Clone)]
pub struct SongData {
    artist: Option<String>,
    title: Option<String>,
    album: Option<String>,
    cover_texture: Option<gdk::Texture>,
    cover_palette: Option<Vec<gdk::RGBA>>,
    uuid: Option<String>,
    duration: u64,
    file: gio::File,
}

impl SongData {
    pub fn artist(&self) -> Option<&str> {
        self.artist.as_deref()
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn album(&self) -> Option<&str> {
        self.album.as_deref()
    }

    pub fn uuid(&self) -> Option<&str> {
        self.uuid.as_deref()
    }

    pub fn duration(&self) -> u64 {
        self.duration
    }

    pub fn cover_texture(&self) -> Option<&gdk::Texture> {
        self.cover_texture.as_ref()
    }

    pub fn cover_palette(&self) -> Option<&Vec<gdk::RGBA>> {
        self.cover_palette.as_ref()
    }

    pub fn from_uri(uri: &str) -> Self {
        let now = Instant::now();

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
            if let Some(picture) = tag.get_picture_type(lofty::PictureType::CoverFront) {
                cover_art = match picture.mime_type() {
                    lofty::MimeType::Png => Some(glib::Bytes::from(picture.data())),
                    lofty::MimeType::Jpeg => Some(glib::Bytes::from(picture.data())),
                    lofty::MimeType::Tiff => Some(glib::Bytes::from(picture.data())),
                    _ => None,
                };
            } else {
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
            }
        } else {
            warn!("Unable to load primary tag for: {}", uri);
            for tag in tagged_file.tags() {
                debug!("Found tag: {:?}", tag.tag_type());
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

                if artist.is_some() && title.is_some() {
                    break;
                }
            }
        };

        let uuid = if let Some(basename) = file.basename() {
            let mut hasher = Sha256::new();

            hasher.update(basename.to_str().unwrap());

            // Compute the checksum using the data we have
            // at load time; at worst, we are going to use
            // the basename of the file, in case the song
            // is missing all metadata
            if let Some(basename) = file.basename() {
                hasher.update(basename.to_str().unwrap());
            }
            if let Some(ref artist) = artist {
                hasher.update(&artist);
            }
            if let Some(ref title) = title {
                hasher.update(&title);
            }
            if let Some(ref album) = album {
                hasher.update(&album);
            }

            Some(format!("{:x}", hasher.finalize()))
        } else {
            None
        };

        // The pixel buffer for the cover art
        let cover_pixbuf = if let Some(ref cover_art) = cover_art {
            utils::load_cover_texture(cover_art)
        } else {
            None
        };

        if let Some(ref pixbuf) = cover_pixbuf {
            if let Some(ref uuid) = uuid {
                // This is not great; the only reason why we have to do this
                // is because MPRIS is a bad specification, and requires us
                // to save the cover art in order to pass a URL to it.
                let mut cache = glib::user_cache_dir();
                cache.push("amberol");
                cache.push("covers");
                glib::mkdir_with_parents(&cache, 0o755);

                cache.push(format!("{}.png", uuid));
                let file = gio::File::for_path(&cache);
                match file.create(gio::FileCreateFlags::NONE, gio::Cancellable::NONE) {
                    Ok(stream) => {
                        debug!("Creating cover data cache at {:?}", &cache);
                        pixbuf.save_to_streamv_async(
                            &stream,
                            "png",
                            &[("tEXt::Software", "amberol")],
                            gio::Cancellable::NONE,
                            move |res| {
                                match res {
                                    Err(e) => warn!("Unable to cache cover data: {}", e),
                                    _ => debug!("Cached cover data: {:?}", &cache),
                                };
                            },
                        );
                    }
                    Err(e) => {
                        if let Some(file_error) = e.kind::<glib::FileError>() {
                            match file_error {
                                glib::FileError::Exist => (),
                                _ => warn!("Unable to create file: {}", e),
                            };
                        }
                    }
                };
            } else {
                warn!("No UUID available")
            }
        }

        // The texture we draw on screen
        let cover_texture = cover_pixbuf.as_ref().map(gdk::Texture::for_pixbuf);

        // The color palette we use for styling the UI
        let cover_palette = if let Some(ref pixbuf) = cover_pixbuf {
            utils::load_palette(pixbuf)
        } else {
            None
        };

        let properties = lofty::AudioFile::properties(&tagged_file);
        let duration = properties.duration().as_secs();

        debug!("Song loading time: {} ms", now.elapsed().as_millis());

        SongData {
            artist,
            title,
            album,
            cover_texture,
            cover_palette,
            uuid,
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
            cover_palette: None,
            uuid: None,
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

        fn set_property(&self, obj: &Self::Type, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "uri" => {
                    if let Ok(p) = value.get::<&str>() {
                        self.data.replace(SongData::from_uri(p));
                        obj.notify("artist");
                        obj.notify("title");
                        obj.notify("album");
                        obj.notify("duration");
                        obj.notify("cover");
                    }
                }
                "playing" => {
                    let p = value.get::<bool>().expect("Value must be a boolean");
                    self.playing.set(p);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "artist" => obj.artist().to_value(),
                "title" => obj.title().to_value(),
                "album" => obj.album().to_value(),
                "duration" => obj.duration().to_value(),
                "uri" => obj.uri().to_value(),
                "cover" => obj.cover_texture().to_value(),
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
        if self.uuid().is_some() && other.uuid().is_some() {
            self.uuid() == other.uuid()
        } else {
            self.uri() == other.uri()
        }
    }

    pub fn uri(&self) -> String {
        self.imp().data.borrow().uri()
    }

    pub fn artist(&self) -> String {
        match self.imp().data.borrow().artist() {
            Some(artist) => artist.to_string(),
            None => i18n("Unknown artist"),
        }
    }

    pub fn title(&self) -> String {
        match self.imp().data.borrow().title() {
            Some(title) => title.to_string(),
            None => i18n("Unknown title"),
        }
    }

    pub fn album(&self) -> String {
        match self.imp().data.borrow().album() {
            Some(album) => album.to_string(),
            None => i18n("Unknown album"),
        }
    }

    pub fn cover_texture(&self) -> Option<gdk::Texture> {
        self.imp().data.borrow().cover_texture().cloned()
    }

    pub fn cover_color(&self) -> Option<gdk::RGBA> {
        self.imp().data.borrow().cover_palette().map(|p| p[0])
    }

    pub fn cover_palette(&self) -> Option<Vec<gdk::RGBA>> {
        self.imp().data.borrow().cover_palette().cloned()
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

    pub fn uuid(&self) -> Option<String> {
        self.imp().data.borrow().uuid().map(|s| s.to_string())
    }
}

impl Default for Song {
    fn default() -> Self {
        Self::empty()
    }
}
