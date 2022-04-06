// SPDX-FileCopyrightText: 2022  Emmanuele Bassi
// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(dead_code)]

use gtk::{gdk, gio, glib};
use image::load_from_memory;
use palette_extract::{get_palette_with_options, MaxColors, PixelEncoding, PixelFilter, Quality};

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

pub fn load_dominant_color(buffer: &glib::Bytes) -> Option<gdk::RGBA> {
    if let Ok(image) = load_from_memory(buffer) {
        let colors: Vec<gdk::RGBA> = get_palette_with_options(
            image.as_bytes(),
            PixelEncoding::Rgb,
            Quality::new(1),
            MaxColors::new(2),
            PixelFilter::White,
        )
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

        debug!("Dominant colors: {:?}", colors);

        return Some(colors[0].clone());
    }

    None
}
