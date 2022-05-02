// SPDX-FileCopyrightText: 2022  Emmanuele Bassi
// SPDX-License-Identifier: GPL-3.0-or-later

use adw::subclass::prelude::*;
use glib::clone;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};

use crate::{
    config::{APPLICATION_ID, VERSION},
    i18n::i18n,
    utils, Window,
};

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct Application {}

    #[glib::object_subclass]
    impl ObjectSubclass for Application {
        const NAME: &'static str = "AmberolApplication";
        type Type = super::Application;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for Application {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            obj.setup_gactions();

            obj.set_accels_for_action("app.quit", &["<primary>q"]);

            obj.set_accels_for_action("queue.add-song", &["s"]);
            obj.set_accels_for_action("queue.add-folder", &["a"]);
            obj.set_accels_for_action("queue.clear", &["<primary>L"]);
            obj.set_accels_for_action("win.previous", &["b"]);
            obj.set_accels_for_action("win.next", &["n"]);
            obj.set_accels_for_action("win.play", &["p"]);
        }
    }

    impl ApplicationImpl for Application {
        fn startup(&self, application: &Self::Type) {
            self.parent_startup(application);

            gtk::Window::set_default_icon_name(APPLICATION_ID);
        }

        fn activate(&self, application: &Self::Type) {
            debug!("Application::activate");

            let window = if let Some(window) = application.active_window() {
                window
            } else {
                let window = Window::new(application);
                window.upcast()
            };

            window.present();
        }

        fn open(&self, application: &Self::Type, files: &[gio::File], _hint: &str) {
            debug!("Application::open");

            let window = if let Some(window) = application.active_window() {
                window
            } else {
                let window = Window::new(application);
                window.upcast()
            };

            for f in files {
                window.downcast_ref::<Window>().unwrap().open_file(f);
            }

            window.present();
        }
    }

    impl GtkApplicationImpl for Application {}
    impl AdwApplicationImpl for Application {}
}

glib::wrapper! {
    pub struct Application(ObjectSubclass<imp::Application>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl Default for Application {
    fn default() -> Self {
        glib::Object::new(&[
            ("application-id", &APPLICATION_ID),
            ("flags", &gio::ApplicationFlags::HANDLES_OPEN),
            // We don't change the resource path depending on the
            // profile, so we need to specify the base path ourselves
            ("resource-base-path", &"/io/bassi/Amberol"),
        ])
        .expect("Failed to create Application")
    }
}

impl Application {
    pub fn new() -> Self {
        Self::default()
    }

    fn setup_gactions(&self) {
        action!(
            self,
            "quit",
            clone!(@weak self as app => move |_, _| {
                app.quit();
            })
        );

        action!(
            self,
            "about",
            clone!(@weak self as app => move |_, _| {
                app.show_about();
            })
        );

        let settings = utils::settings_manager();
        let enable_recoloring = settings.boolean("enable-recoloring");
        stateful_action!(
            self,
            "enable-recoloring",
            enable_recoloring,
            clone!(@weak self as app => move |action, _| {
                let state = action.state().unwrap();
                let action_state: bool = state.get().unwrap();
                let enable_recoloring = !action_state;
                action.set_state(&enable_recoloring.to_variant());

                let settings = utils::settings_manager();
                settings.set_boolean("enable-recoloring", enable_recoloring).expect("Unable to store setting");
            })
        );
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
        let dialog = gtk::AboutDialog::builder()
            .transient_for(&window)
            .modal(true)
            .logo_icon_name(APPLICATION_ID)
            .program_name("Amberol")
            .comments(&i18n("Plays music and nothing else"))
            .version(VERSION)
            .authors(vec!["Emmanuele Bassi".into()])
            .copyright("© 2022 Emmanuele Bassi")
            .license_type(gtk::License::Gpl30)
            // Translators: Replace "translator-credits" with your names, one name per line
            .translator_credits(&i18n("translator-credits"))
            .build();

        dialog.present();
    }
}
