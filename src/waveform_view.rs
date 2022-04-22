// SPDX-FileCopyrightText: 2022  Emmanuele Bassi
// SPDX-License-Identifier: GPL-3.0-or-later

// Based on gnome-sound-recorder/src/waveform.js:
// - Copyright 2013 Meg Ford
// - Copyright 2022 Kavan Mevada
// Released under the terms of the LGPL 2.0 or later

use std::{
    cell::{Cell, RefCell},
    ops::DivAssign,
};

use adw::subclass::prelude::*;
use glib::clone;
use gtk::{cairo, glib, graphene, prelude::*, subclass::prelude::*};

#[derive(Debug, PartialEq)]
pub struct PeakPair {
    pub left: f64,
    pub right: f64,
}

impl PeakPair {
    pub fn new(left: f64, right: f64) -> Self {
        Self { left, right }
    }
}

impl DivAssign<f64> for PeakPair {
    fn div_assign(&mut self, rhs: f64) {
        self.left /= rhs;
        self.right /= rhs;
    }
}

mod imp {
    use glib::{subclass::Signal, ParamFlags, ParamSpec, ParamSpecDouble, Value};
    use once_cell::sync::Lazy;

    use super::*;

    #[derive(Debug, Default)]
    pub struct WaveformView {
        pub position: Cell<f64>,
        // left and right channel peaks, normalised between 0 and 1
        pub peaks: RefCell<Option<Vec<PeakPair>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WaveformView {
        const NAME: &'static str = "AmberolWaveformView";
        type Type = super::WaveformView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("waveformview");
        }

        fn instance_init(_obj: &glib::subclass::InitializingObject<Self>) {}

        fn new() -> Self {
            Self {
                position: Cell::new(0.0),
                peaks: RefCell::new(None),
            }
        }
    }

    impl ObjectImpl for WaveformView {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![ParamSpecDouble::new(
                    "position",
                    "",
                    "",
                    0.0,
                    1.0,
                    0.0,
                    ParamFlags::READWRITE,
                )]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(&self, _obj: &Self::Type, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "position" => self.position.replace(value.get::<f64>().unwrap()),
                _ => unimplemented!(),
            };
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "position" => self.position.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder(
                    "position-changed",
                    // The position
                    &[f64::static_type().into()],
                    <()>::static_type().into(),
                )
                .build()]
            });

            SIGNALS.as_ref()
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            obj.setup_gesture();
        }
    }

    impl WidgetImpl for WaveformView {
        fn request_mode(&self, _widget: &Self::Type) -> gtk::SizeRequestMode {
            gtk::SizeRequestMode::ConstantSize
        }

        fn measure(
            &self,
            _widget: &Self::Type,
            orientation: gtk::Orientation,
            _for_size: i32,
        ) -> (i32, i32, i32, i32) {
            match orientation {
                gtk::Orientation::Horizontal => (0, 0, -1, -1),
                gtk::Orientation::Vertical => (48, 48, -1, -1),
                _ => (0, 0, -1, -1),
            }
        }

        fn snapshot(&self, widget: &Self::Type, snapshot: &gtk::Snapshot) {
            if let Some(ref peaks) = *self.peaks.borrow() {
                if peaks.len() == 0 {
                    return;
                }

                let w = widget.width();
                let h = widget.height();
                if w == 0 || h == 0 {
                    return;
                }

                let center_y = h as f64 / 2.0;

                // Grab the colors
                let style_context = widget.style_context();
                let color = style_context.color();
                let dimmed_color = if let Some(color) = style_context.lookup_color("dimmed_color") {
                    color
                } else {
                    style_context.color()
                };
                let cursor_color = if let Some(color) = style_context.lookup_color("accent_color") {
                    color
                } else {
                    style_context.color()
                };

                // Set up the Cairo node
                let cr = snapshot.append_cairo(&graphene::Rect::new(0.0, 0.0, w as f32, h as f32));

                cr.set_line_cap(cairo::LineCap::Round);
                cr.set_line_width(1.0);

                // If we have more samples than pixels, then we chunk the
                // samples and we average each chunk
                let spacing;
                let chunk_per_pixel;
                if peaks.len() < w as usize {
                    chunk_per_pixel = 1;
                    spacing = 2.0 * (w as f64 / peaks.len() as f64);
                } else {
                    let effective_width = (w as f64 - 4.0) / 2.0;
                    chunk_per_pixel = (peaks.len() as f64 / effective_width).round() as usize;
                    spacing = 2.0;
                }

                let mut offset = spacing;
                let cursor_pos = self.position.get() * w as f64 + spacing;

                for chunk in peaks.chunks(chunk_per_pixel) {
                    // Average each chunk
                    let mut peak_avg = PeakPair::new(0.0, 0.0);
                    for p in 0..chunk.len() {
                        peak_avg.left += chunk[p].left;
                        peak_avg.right += chunk[p].right;
                    }
                    peak_avg /= chunk.len() as f64;

                    cr.set_line_cap(cairo::LineCap::Round);
                    cr.set_line_width(1.0);

                    // Scale by half: left goes in the upper half of the
                    // available space, and right goes in the lower half
                    let left = peak_avg.left / 3.0;
                    let right = peak_avg.right / 3.0;

                    // Dim the part that we have just played
                    if (offset - cursor_pos).abs() < spacing {
                        cr.set_source_rgba(
                            cursor_color.red().into(),
                            cursor_color.green().into(),
                            cursor_color.blue().into(),
                            cursor_color.alpha().into(),
                        );
                    } else if offset > cursor_pos {
                        cr.set_source_rgba(
                            color.red().into(),
                            color.green().into(),
                            color.blue().into(),
                            color.alpha().into(),
                        );
                    } else {
                        cr.set_source_rgba(
                            dimmed_color.red().into(),
                            dimmed_color.green().into(),
                            dimmed_color.blue().into(),
                            dimmed_color.alpha().into(),
                        );
                    }

                    cr.move_to(offset + 0.5, center_y + left * h as f64);
                    cr.line_to(offset + 0.5, center_y - right * h as f64);
                    cr.stroke().expect("stroke");

                    offset += spacing as f64;
                }
            }
        }
    }
}

glib::wrapper! {
    pub struct WaveformView(ObjectSubclass<imp::WaveformView>)
        @extends gtk::Widget;
}

impl Default for WaveformView {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create WaveformView")
    }
}

impl WaveformView {
    pub fn new() -> Self {
        Self::default()
    }

    fn setup_gesture(&self) {
        let gesture = gtk::GestureClick::new();
        gesture.set_name("waveform-click");
        gesture.set_button(0);
        gesture.connect_pressed(
            clone!(@strong self as this => move |gesture, n_press, x, _| {
                if n_press == 1 {
                    gesture.set_state(gtk::EventSequenceState::Claimed);
                    let width = this.width();
                    let position = x as f64 / width as f64;
                    debug!("Button press at {} (width: {}, position: {})", x, width, position);
                    this.emit_by_name::<()>("position-changed", &[&position]);
                }
            }),
        );
        self.add_controller(&gesture);
    }

    fn normalize_peaks(&self, peaks: Vec<(f64, f64)>) -> Vec<PeakPair> {
        let left_channel: Vec<f64> = peaks.iter().map(|p| p.0).collect();
        let right_channel: Vec<f64> = peaks.iter().map(|p| p.1).collect();

        let max_left: f64 = left_channel
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let max_right: f64 = right_channel
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);

        let normalized: Vec<PeakPair> = peaks
            .iter()
            .map(|p| PeakPair::new(p.0 / max_left, p.1 / max_right))
            .collect();

        normalized
    }

    pub fn set_peaks(&self, peaks: Option<Vec<(f64, f64)>>) {
        if let Some(peaks) = peaks {
            let peak_pairs = self.normalize_peaks(peaks);
            debug!("Peaks: {}", peak_pairs.len());
            self.imp().peaks.replace(Some(peak_pairs));
        } else {
            self.imp().peaks.replace(None);
        }
        self.queue_draw();
    }

    pub fn set_position(&self, position: f64) {
        self.imp().position.replace(position.clamp(0.0, 1.0));
        self.queue_draw();
    }
}
