// SPDX-FileCopyrightText: 2022  Emmanuele Bassi
// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(dead_code)]

use color_thief::{get_palette, ColorFormat};
use gtk::{gdk, gio, glib, prelude::*};

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

pub fn is_color_dark(color: &gdk::RGBA) -> bool {
    let lum = color.red() * 0.2126 + color.green() * 0.7152 + color.blue() * 0.072;

    lum < 0.5
}

pub fn load_cover_texture(buffer: &glib::Bytes) -> Option<gdk::Texture> {
    let texture = match gdk::Texture::from_bytes(buffer) {
        Ok(t) => Some(t),
        Err(_) => None,
    };

    texture
}

pub fn load_palette(texture: &gdk::Texture) -> Option<Vec<gdk::RGBA>> {
    let mut buf: Vec<u8> = Vec::new();
    buf.resize(texture.height() as usize * texture.width() as usize * 4, 0);
    texture.download(&mut buf, 4 * texture.width() as usize);

    if let Ok(palette) = get_palette(&buf, ColorFormat::Rgba, 5, 4) {
        let colors: Vec<gdk::RGBA> = palette
            .iter()
            .map(|c| {
                gdk::RGBA::new(
                    c.r as f32 / 255.0,
                    c.g as f32 / 255.0,
                    c.b as f32 / 255.0,
                    1.0,
                )
            })
            .collect();

        return Some(colors);
    }

    None
}

pub fn load_dominant_color(texture: &gdk::Texture) -> Option<gdk::RGBA> {
    if let Some(palette) = load_palette(texture) {
        return Some(palette[0]);
    }

    None
}
