// SPDX-FileCopyrightText: 2022  Emmanuele Bassi
// SPDX-License-Identifier: GPL-3.0-or-later

mod application;
mod config;
mod drag_overlay;
mod i18n;
mod player;
mod queue_row;
mod song;
mod window;

use self::{application::AmberolApplication, window::Window};

#[macro_use]
extern crate log;

use std::env;

use config::{APPLICATION_ID, GETTEXT_PACKAGE, LOCALEDIR, PKGDATADIR, PROFILE};
use gettextrs::{bind_textdomain_codeset, bindtextdomain, setlocale, textdomain, LocaleCategory};
use gtk::{gio, glib, prelude::*};

fn main() {
    pretty_env_logger::init();

    // Set up gettext translations
    debug!("Setting up locale data");
    setlocale(LocaleCategory::LcAll, "");

    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to set the text domain encoding");
    textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    debug!("Setting up pulseaudio environment");
    env::set_var("PULSE_PROP_application.icon_name", &APPLICATION_ID);
    env::set_var("PULSE_PROP_application.metadata().name", "Amberol");

    debug!("Loading resources");
    let resources = gio::Resource::load(PKGDATADIR.to_owned() + "/amberol.gresource")
        .expect("Could not load resources");
    gio::resources_register(&resources);

    debug!("Setting up application (profile: {})", &PROFILE);
    glib::set_application_name("Amberol");
    glib::set_program_name(Some("amberol"));

    gtk::init().expect("Failed to initialize gtk");
    gst::init().expect("Failed to initialize gstreamer");
    adw::init();

    let app = AmberolApplication::new(&APPLICATION_ID, &gio::ApplicationFlags::empty());

    std::process::exit(app.run());
}
