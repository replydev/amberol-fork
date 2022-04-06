// SPDX-FileCopyrightText: 2022  Emmanuele Bassi
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::gio;

use crate::config::APPLICATION_ID;

pub fn settings_manager() -> gio::Settings {
    // We ship a single schema for both default and development profiles
    let app_id = APPLICATION_ID.trim_end_matches(".Devel");
    gio::Settings::new(app_id)
}

pub fn format_time(seconds: u64, total: u64) -> String {
    let min = seconds / 60;
    let total_min = total / 60;
    format!(
        "{}:{:02} / {}:{:02}",
        min,
        seconds % 60,
        total_min,
        total % 60
    )
}
