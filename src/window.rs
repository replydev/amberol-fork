// SPDX-FileCopyrightText: 2022  Emmanuele Bassi
// SPDX-License-Identifier: GPL-3.0-or-later

use adw::subclass::prelude::*;
use glib::clone;
use gtk::{gdk, gio, glib, prelude::*, subclass::prelude::*, CompositeTemplate};

use crate::{
    config::APPLICATION_ID,
    drag_overlay::DragOverlay,
    i18n::{i18n, ni18n_f},
    player::{AudioPlayerWrapper, RepeatMode},
    queue_row::QueueRow,
    song::Song,
    utils,
};

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/bassi/Amberol/window.ui")]
    pub struct Window {
        // Template widgets
        #[template_child]
        pub playlist_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub previous_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub rewind_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub play_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub forward_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub next_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub repeat_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub song_title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub song_artist_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub song_album_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub song_time_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub queue_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub queue_view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub album_image: TemplateChild<gtk::Picture>,
        #[template_child]
        pub queue_length_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub drag_overlay: TemplateChild<DragOverlay>,

        pub player: AudioPlayerWrapper,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "AmberolWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action("win.play", None, move |win, _, _| {
                debug!("Window::win.play()");
                let player = win.imp().player.borrow();
                player.toggle_play_pause();
            });
            klass.install_action("win.previous", None, move |win, _, _| {
                debug!("Window::win.previous()");
                let player = win.imp().player.borrow();
                player.skip_previous();
            });
            klass.install_action("win.next", None, move |win, _, _| {
                debug!("Window::win.next()");
                let player = win.imp().player.borrow();
                player.skip_next();
            });
            klass.install_action("win.seek-backwards", None, move |win, _, _| {
                let player = win.imp().player.borrow();
                player.seek_backwards();
            });
            klass.install_action("win.seek-forward", None, move |win, _, _| {
                let player = win.imp().player.borrow();
                player.seek_forward();
            });
            klass.install_action("queue.repeat-mode", None, move |win, _, _| {
                debug!("Window::queue.repeat()");
                let player = win.imp().player.borrow();
                player.toggle_queue_repeat();
            });
            klass.install_action("queue.add-song", None, move |win, _, _| {
                debug!("Window::win.add-song()");
                win.add_song();
            });
            klass.install_action("queue.add-folder", None, move |win, _, _| {
                debug!("Window::win.add-folder()");
                win.add_folder();
            });
            klass.install_action("queue.clear", None, move |win, _, _| {
                debug!("Window::queue.clear()");
                win.clear_queue();
            });
            klass.install_action("queue.show", None, move |win, _, _| {
                debug!("Window::queue.show()");
                win.toggle_queue();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }

        fn new() -> Self {
            Self {
                playlist_button: TemplateChild::default(),
                previous_button: TemplateChild::default(),
                rewind_button: TemplateChild::default(),
                play_button: TemplateChild::default(),
                forward_button: TemplateChild::default(),
                next_button: TemplateChild::default(),
                repeat_button: TemplateChild::default(),
                menu_button: TemplateChild::default(),
                song_title_label: TemplateChild::default(),
                song_artist_label: TemplateChild::default(),
                song_album_label: TemplateChild::default(),
                song_time_label: TemplateChild::default(),
                queue_revealer: TemplateChild::default(),
                queue_view: TemplateChild::default(),
                album_image: TemplateChild::default(),
                queue_length_label: TemplateChild::default(),
                drag_overlay: TemplateChild::default(),
                player: AudioPlayerWrapper::new(),
            }
        }
    }

    impl ObjectImpl for Window {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            if APPLICATION_ID.ends_with("Devel") {
                obj.add_css_class("devel");
            }

            obj.connect_signals();
            obj.init_actions();
            obj.bind_state();
            obj.setup_queue();
            obj.setup_drop_target();
            // FIXME: https://gitlab.gnome.org/GNOME/gtk/-/issues/4136
            // obj.restore_window_state();
        }
    }

    impl WidgetImpl for Window {}
    impl WindowImpl for Window {}
    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl Window {
    pub fn new<P: glib::IsA<gtk::Application>>(application: &P) -> Self {
        glib::Object::new(&[("application", application)]).expect("Failed to create Window")
    }

    fn imp(&self) -> &imp::Window {
        imp::Window::from_instance(self)
    }

    // fn restore_window_state(&self) {
    //     let settings = utils::settings_manager();
    //     let width = settings.int("window-width");
    //     let height = settings.int("window-height");
    //     self.set_default_size(width, height);
    // }

    fn clear_queue(&self) {
        self.imp().player.borrow().queue_clear();
    }

    fn toggle_queue(&self) {
        let visible = self.imp().queue_revealer.reveals_child();
        self.imp().queue_revealer.set_reveal_child(!visible);
        let width = self.default_size().0;
        self.set_default_size(width, -1);
    }

    fn add_song(&self) {
        let app = gio::Application::default()
            .expect("Failed to retrieve application singleton")
            .downcast::<gtk::Application>()
            .unwrap();
        let win = app
            .active_window()
            .unwrap()
            .downcast::<gtk::Window>()
            .unwrap();
        let dialog = gtk::FileChooserNative::builder()
            .accept_label(&i18n("_Add Song"))
            .cancel_label(&i18n("_Cancel"))
            .modal(true)
            .title("Open File")
            .action(gtk::FileChooserAction::Open)
            .transient_for(&win)
            .build();

        let filter = gtk::FileFilter::new();
        gtk::FileFilter::set_name(&filter, Some(&i18n("Audio files")));
        filter.add_mime_type("audio/*");
        dialog.add_filter(&filter);

        dialog.connect_response(
            clone!(@strong dialog, @weak self as win => move |_, response| {
                if response == gtk::ResponseType::Accept {
                    win.add_files_to_queue(&dialog.files());
                }
            }),
        );
        dialog.show();
    }

    fn add_folder(&self) {
        let app = gio::Application::default()
            .expect("Failed to retrieve application singleton")
            .downcast::<gtk::Application>()
            .unwrap();
        let win = app
            .active_window()
            .unwrap()
            .downcast::<gtk::Window>()
            .unwrap();
        let dialog = gtk::FileChooserNative::builder()
            .accept_label(&i18n("_Add Folder"))
            .cancel_label(&i18n("_Cancel"))
            .modal(true)
            .title("Open Folder")
            .action(gtk::FileChooserAction::SelectFolder)
            .transient_for(&win)
            .build();

        dialog.connect_response(
            clone!(@strong dialog, @weak self as win => move |_, response| {
                if response == gtk::ResponseType::Accept {
                    win.add_folders_to_queue(&dialog.files());
                }
            }),
        );
        dialog.show();
    }

    fn add_folders_to_queue(&self, folders: &gio::ListModel) {
        for pos in 0..folders.n_items() {
            let folder = folders.item(pos).unwrap().downcast::<gio::File>().unwrap();
            debug!("Adding the contents of {} to the queue", folder.uri());

            let player = self.imp().player.borrow();

            let mut enumerator = folder
                .enumerate_children(
                    "standard::*",
                    gio::FileQueryInfoFlags::NONE,
                    None::<&gio::Cancellable>,
                )
                .expect("Unable to enumerate");

            let mut files = Vec::new();
            while let Some(info) = enumerator.next().and_then(|s| s.ok()) {
                if info.file_type() != gio::FileType::Regular {
                    continue;
                }

                if let Some(content_type) = info.content_type() {
                    if gio::content_type_is_a(&content_type, "audio/*") {
                        let child = enumerator.child(&info);
                        debug!("Adding {} to the queue", child.uri());
                        files.push(child.clone());
                    }
                }
            }

            // gio::FileEnumerator has no guaranteed order, so we should
            // rely on the basename being formatted in a way that gives us an
            // implicit order; if anything, this will queue songs in the same
            // order in which they appear in the directory when browsing its
            // contents
            files.sort_by(|a, b| {
                a.basename()
                    .unwrap()
                    .partial_cmp(&b.basename().unwrap())
                    .unwrap()
            });
            let songs: Vec<Song> = files.iter().map(|f| Song::new(f.uri().as_str())).collect();
            for s in songs {
                player.queue_song(&s);
            }
        }
    }

    fn add_files_to_queue(&self, files: &gio::ListModel) {
        for pos in 0..files.n_items() {
            let file = files.item(pos).unwrap().downcast::<gio::File>().unwrap();
            debug!("Adding {} to the queue", file.uri());
            self.add_file_to_queue(&file);
        }
    }

    pub fn add_file_to_queue(&self, file: &gio::File) {
        let song = Song::new(file.uri().as_str());
        self.imp().player.borrow().queue_song(&song);
    }

    fn bind_state(&self) {
        let imp = self.imp();
        let player = imp.player.borrow();
        let state = player.state();

        state
            .bind_property("current-title", &imp.song_title_label.get(), "label")
            .flags(glib::BindingFlags::DEFAULT)
            .build();
        state
            .bind_property("current-artist", &imp.song_artist_label.get(), "label")
            .flags(glib::BindingFlags::DEFAULT)
            .build();
        state
            .bind_property("current-album", &imp.song_album_label.get(), "label")
            .flags(glib::BindingFlags::DEFAULT)
            .build();
        state
            .bind_property("current-time", &imp.song_time_label.get(), "label")
            .flags(glib::BindingFlags::DEFAULT)
            .build();
        state
            .bind_property("current-cover", &imp.album_image.get(), "paintable")
            .flags(glib::BindingFlags::DEFAULT)
            .build();
    }

    fn connect_signals(&self) {
        let imp = self.imp();
        let player = imp.player.borrow();

        player.state().connect_notify_local(
            Some("current-song"),
            clone!(@weak self as win => move |state, _| {
                let current = state.current_song();
                let n_songs = state.n_songs();
                if n_songs == 0 {
                    win.action_set_enabled("win.previous", false);
                    win.action_set_enabled("win.next", false);
                } else {
                    win.action_set_enabled("win.previous", true);
                    win.action_set_enabled("win.next", current < n_songs - 1);
                }

                win.update_playlist_time();
            }),
        );

        player.state().connect_notify_local(
            Some("n-songs"),
            clone!(@weak self as win => move |state, _| {
                let n_songs = state.n_songs();
                win.action_set_enabled("win.play", n_songs != 0);
                win.action_set_enabled("win.pause", n_songs != 0);
                win.action_set_enabled("win.seek-backwards", n_songs != 0);
                win.action_set_enabled("win.seek-forward", n_songs != 0);

                let current = state.current_song();
                if n_songs > 0 {
                    win.action_set_enabled("win.previous", true);
                    win.action_set_enabled("win.next", current < n_songs - 1);
                }

                win.update_playlist_time();
            }),
        );

        player.state().connect_notify_local(
            Some("playing"),
            clone!(@weak self as win => move |state, _| {
                let imp = win.imp();
                if state.playing() {
                    imp.play_button.set_icon_name("media-playback-pause-symbolic");
                } else {
                    imp.play_button.set_icon_name("media-playback-start-symbolic");
                }
            }),
        );

        player.state().connect_notify_local(
            Some("repeat-mode"),
            clone!(@weak self as win => move |state, _| {
                let imp = win.imp();
                match state.repeat_mode() {
                    RepeatMode::Consecutive => {
                        imp.repeat_button.set_icon_name("media-playlist-consecutive-symbolic");
                    },
                    RepeatMode::RepeatAll => {
                        imp.repeat_button.set_icon_name("media-playlist-repeat-symbolic");
                    },
                    RepeatMode::RepeatOne => {
                        imp.repeat_button.set_icon_name("media-playlist-repeat-song-symbolic");
                    },
                };
            }),
        );

        self.connect_close_request(move |window| {
            debug!("Saving window state");
            let width = window.default_size().0;
            let height = window.default_size().1;

            let settings = utils::settings_manager();
            settings
                .set_int("window-width", width)
                .expect("Unable to store window-width");
            settings
                .set_int("window-height", height)
                .expect("Unable to stop window-height");

            glib::signal::Inhibit(false)
        });
    }

    // The initial state of the playback actions
    fn init_actions(&self) {
        self.action_set_enabled("win.play", false);
        self.action_set_enabled("win.pause", false);
        self.action_set_enabled("win.previous", false);
        self.action_set_enabled("win.next", false);
        self.action_set_enabled("win.seek-backwards", false);
        self.action_set_enabled("win.seek-forward", false);
    }

    fn setup_queue(&self) {
        let imp = self.imp();

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let row = QueueRow::new();
            list_item.set_child(Some(&row));

            list_item
                .property_expression("item")
                .chain_property::<Song>("artist")
                .bind(&row, "song-artist", gtk::Widget::NONE);
            list_item
                .property_expression("item")
                .chain_property::<Song>("title")
                .bind(&row, "song-title", gtk::Widget::NONE);
            list_item
                .property_expression("item")
                .chain_property::<Song>("playing")
                .bind(&row, "playing", gtk::Widget::NONE);
        });
        imp.queue_view
            .set_factory(Some(&factory.upcast::<gtk::ListItemFactory>()));

        let player = imp.player.borrow();
        let selection = gtk::SingleSelection::new(Some(player.queue()));
        selection.set_can_unselect(false);
        selection.set_selected(gtk::INVALID_LIST_POSITION);
        imp.queue_view
            .set_model(Some(&selection.upcast::<gtk::SelectionModel>()));
        imp.queue_view
            .connect_activate(clone!(@weak self as win => move |_, pos| {
                let player = win.imp().player.borrow();
                player.skip_to(pos);
                if !player.state().playing() {
                    player.play();
                }
            }));
    }

    fn setup_drop_target(&self) {
        let drop_target = gtk::DropTarget::builder()
            .name("file-drop-target")
            .actions(gdk::DragAction::COPY)
            .formats(&gdk::ContentFormats::for_type(gio::File::static_type()))
            .build();

        drop_target.connect_drop(
            clone!(@weak self as win => @default-return false, move |_, value, _, _| {
                if let Ok(file) = value.get::<gio::File>() {
                    if !file.query_exists(gio::Cancellable::NONE) {
                        debug!("Received {} but cannot access it", file.uri());
                        return false;
                    }
                    debug!("Creating Song for {}", file.uri());
                    let song = Song::new(file.uri().as_str());
                    if song.equals(&Song::default()) {
                        return false;
                    }
                    win.imp().player.borrow().queue_song(&song);
                    return true;
                }
                false
            }),
        );

        self.imp().drag_overlay.set_drop_target(&drop_target);
    }

    fn update_playlist_time(&self) {
        let player = self.imp().player.borrow();
        let state = player.state();
        let n_songs = state.n_songs();
        let current = state.current_song();

        let mut remaining_time = 0;
        for pos in 0..n_songs {
            let song = state.song_at(pos);
            if pos >= current {
                remaining_time += song.duration();
            }
        }

        let title = format!("<b>{}</b>", &i18n("Playlist"));
        let remaining_min = (remaining_time / 60) as u32;
        let remaining_str = &ni18n_f(
            // Translators: The first '{}' is the word "Playlist";
            // the second '{}' is the number of minutes remaining
            // in the playlist
            "{} ({} minute remaining)",
            "{} ({} minutes remaining)",
            remaining_min,
            &[&title, &remaining_min.to_string()],
        );

        self.imp().queue_length_label.set_label(remaining_str);
    }
}
