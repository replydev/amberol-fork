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
        pub hover_position: Cell<Option<f64>>,
        // left and right channel peaks, normalised between 0 and 1
        pub peaks: RefCell<Option<Vec<PeakPair>>>,
        pub tick_id: RefCell<Option<gtk::TickCallbackId>>,
        pub last_frame_time: Cell<Option<i64>>,
        pub factor: Cell<Option<f64>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WaveformView {
        const NAME: &'static str = "AmberolWaveformView";
        type Type = super::WaveformView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("waveformview");
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
                gtk::Orientation::Horizontal => (256, 256, -1, -1),
                gtk::Orientation::Vertical => (48, 48, -1, -1),
                _ => (0, 0, -1, -1),
            }
        }

        fn snapshot(&self, widget: &Self::Type, snapshot: &gtk::Snapshot) {
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
            let cursor_color = if let Some(color) = style_context.lookup_color("background_color_2")
            {
                color
            } else {
                if let Some(color) = style_context.lookup_color("accent_color") {
                    color
                } else {
                    style_context.color()
                }
            };

            let bar_size = 2.0;
            let space_size = 2.0;
            let block_size = bar_size + space_size;
            let effective_width = (w as f64 - (2.0 * space_size)) / block_size;

            // Set up the Cairo node
            let cr = snapshot.append_cairo(&graphene::Rect::new(0.0, 0.0, w as f32, h as f32));

            if let Some(ref peaks) = *self.peaks.borrow() {
                // If we have more samples than pixels, then we chunk the
                // samples and we average each chunk
                let spacing;
                let chunk_per_pixel;
                if peaks.len() < w as usize {
                    chunk_per_pixel = 1;
                    spacing = block_size * (w as f64 / peaks.len() as f64);
                } else {
                    chunk_per_pixel = (peaks.len() as f64 / effective_width).round() as usize;
                    spacing = block_size;
                }

                let mut offset = spacing;

                // We have two cursors:
                //
                // 1. the state position, updated by the player
                // 2. the hover position, updated by the motion controller
                //
                // The hover position may be behind the state position, if we are
                // scrubbing backwards; or after the state position, if we are
                // scrubbing forward.
                //
                // The area between the state position and the hover position is
                // meant to be shown as a dimmed cursor color; the area between
                // the start of the waveform and the state position is meant to be
                // shown as a full cursor color; and the area between the hover
                // position and the end of the waveform is meant to be shown as a
                // current foreground color.
                let position = self.position.get();
                let mut cursor_pos: [f64; 2] =
                    [position * w as f64 + spacing, position * w as f64 + spacing];
                if let Some(hover) = self.hover_position.get() {
                    if hover >= position {
                        cursor_pos[1] = hover * w as f64 + spacing;
                    } else {
                        cursor_pos[0] = hover * w as f64 + spacing;
                    }
                }

                cr.set_line_cap(cairo::LineCap::Round);
                cr.set_line_width(bar_size);

                for chunk in peaks.chunks(chunk_per_pixel) {
                    // Average each chunk
                    let mut peak_avg = PeakPair::new(0.0, 0.0);
                    for p in 0..chunk.len() {
                        peak_avg.left += chunk[p].left;
                        peak_avg.right += chunk[p].right;
                    }
                    peak_avg /= chunk.len() as f64;

                    // Scale by half: left goes in the upper half of the
                    // available space, and right goes in the lower half
                    let mut left = peak_avg.left / 2.0;
                    let mut right = peak_avg.right / 2.0;

                    if let Some(factor) = self.factor.get() {
                        left *= factor;
                        right *= factor;
                    }

                    if offset < (cursor_pos[0] - spacing) {
                        cr.set_source_rgba(
                            cursor_color.red().into(),
                            cursor_color.green().into(),
                            cursor_color.blue().into(),
                            cursor_color.alpha().into(),
                        );
                    } else if offset < (cursor_pos[1] - spacing) {
                        cr.set_source_rgba(
                            cursor_color.red().into(),
                            cursor_color.green().into(),
                            cursor_color.blue().into(),
                            dimmed_color.alpha().into(),
                        );
                    } else {
                        cr.set_source_rgba(
                            color.red().into(),
                            color.green().into(),
                            color.blue().into(),
                            color.alpha().into(),
                        );
                    }

                    cr.move_to(offset, center_y + left * h as f64);
                    cr.line_to(offset, center_y - right * h as f64);
                    cr.stroke().expect("stroke");

                    offset += spacing as f64;
                }
            } else {
                cr.set_line_cap(cairo::LineCap::Butt);
                cr.set_line_width(bar_size);

                cr.move_to(space_size, center_y);
                cr.line_to(w as f64 - space_size, center_y);
                cr.set_source_rgba(
                    color.red().into(),
                    color.green().into(),
                    color.blue().into(),
                    color.alpha().into(),
                );
                cr.set_dash(&[2.0, 2.0], 0.0);
                cr.stroke().expect("midline stroke");
            }
        }
    }
}

glib::wrapper! {
    pub struct WaveformView(ObjectSubclass<imp::WaveformView>)
        @extends gtk::Widget;
}

fn ease_out_cubic(t: f64) -> f64 {
    let p = t - 1.0;
    p * p * p + 1.0
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
        let click_gesture = gtk::GestureClick::new();
        click_gesture.set_name("waveform-click");
        click_gesture.set_button(0);
        click_gesture.connect_pressed(
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
        self.add_controller(&click_gesture);

        let motion_gesture = gtk::EventControllerMotion::new();
        motion_gesture.set_name("waveform-motion");
        motion_gesture.connect_motion(clone!(@strong self as this => move |_, x, _| {
            let width = this.width() as f64;
            let position = x as f64 / width;
            this.imp().hover_position.replace(Some(position));
            this.queue_draw();
        }));
        motion_gesture.connect_leave(clone!(@strong self as this => move |_| {
            this.imp().hover_position.replace(None);
            this.queue_draw();
        }));
        self.add_controller(&motion_gesture);
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
        if let Some(tick_id) = self.imp().tick_id.replace(None) {
            tick_id.remove();
        }

        if let Some(peaks) = peaks {
            let peak_pairs = self.normalize_peaks(peaks);
            debug!("Peaks: {}", peak_pairs.len());
            self.imp().peaks.replace(Some(peak_pairs));

            if self.settings().is_gtk_enable_animations() {
                self.imp().factor.set(Some(0.0));
                self.imp().last_frame_time.set(None);

                self.add_tick_callback(clone!(@strong self as this => move |_, clock| {
                    let frame_time = clock.frame_time();
                    if let Some(last_frame_time) = this.imp().last_frame_time.get() {
                        if frame_time < last_frame_time {
                            warn!("Frame clock going backwards");
                            return glib::Continue(true);
                        }

                        let delta = ease_out_cubic((frame_time - last_frame_time) as f64 / 250_000.0);
                        this.imp().factor.replace(Some(delta));
                        this.queue_draw();
                        if delta >= 1.0 {
                            debug!("Animation complete");
                            this.imp().tick_id.replace(None);
                            return glib::Continue(false);
                        }
                    } else {
                        this.imp().last_frame_time.replace(Some(frame_time));
                    }

                    glib::Continue(true)
                }));
            }
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
