//! Camera Raw's local adjustments: masks that confine a set of develop sliders to part of the
//! picture (the Masking panel: Linear Gradient and Radial Gradient).
//!
//! A mask holds a *shape* and a set of *adjustments* (the Basic, Presence and Effects sliders that
//! make sense locally). The develop pipeline runs the whole picture through the global sliders
//! first. Then, for each active mask in order, it develops a copy with that mask's sliders alone,
//! the same stages as the global develop, and mixes the copy over the picture by the mask's
//! coverage times its Amount. Masks therefore stack, and every slider means what it means in the
//! global panels.
//!
//! Shapes are stored in picture fractions (0 at the left or top edge, 1 at the right or bottom
//! edge), so a mask follows the picture when the preview proxy is smaller than the full image:
//!
//! * **Linear Gradient**: full effect at `start`, none at `end`, a straight ramp between, along
//!   the line joining them; everything beyond `start` is fully covered, beyond `end` not at all.
//! * **Radial Gradient**: an ellipse at `center` with half-axes `radius` (fractions of the width
//!   and height), turned by `angle` degrees clockwise; full effect inside, falling off to none at
//!   the edge over the outer `feather` percent of the radius. A feather of 0 is a hard edge, one
//!   pixel wide so it does not alias.
//!
//! `invert` swaps covered and uncovered. Everything is bounded: [`MAX_MASKS`] masks, positions
//! within a picture's width of the picture, finite numbers only (anything else counts as the
//! default or as neutral), so stored or scripted settings cannot cost unbounded work or produce
//! NaN pixels.

use serde::{Deserialize, Serialize};

use super::CameraRaw;
use crate::photo_util::par_rows;

/// Most masks one Camera Raw setting develops. Each costs one more develop pass over the picture.
pub const MAX_MASKS: usize = 16;

/// How far outside the picture (as a fraction of its size) a mask's points may sit.
const POSITION_MARGIN: f32 = 1.0;

/// The geometry of a [`LocalMask`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MaskShape {
    /// Full effect at `start`, none at `end`.
    #[default]
    Linear,
    /// An ellipse around `center`.
    Radial,
}

/// The sliders a mask applies, in the units of the global panels with the same names. All 0 is
/// neutral.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LocalAdjust {
    pub temperature: f32,
    pub tint: f32,
    /// Stops, −5…5.
    pub exposure: f32,
    pub contrast: f32,
    pub highlights: f32,
    pub shadows: f32,
    pub whites: f32,
    pub blacks: f32,
    pub texture: f32,
    pub clarity: f32,
    pub dehaze: f32,
    pub vibrance: f32,
    pub saturation: f32,
}

/// Replaces a non-finite number by `default`, and bounds the rest to `lo..=hi`.
fn bounded(v: f32, lo: f32, hi: f32, default: f32) -> f32 {
    if v.is_finite() { v.clamp(lo, hi) } else { default }
}

impl LocalAdjust {
    /// Whether every slider is at 0 (or not a number).
    pub fn is_neutral(&self) -> bool {
        self.values().iter().all(|v| !v.is_finite() || *v == 0.0)
    }

    fn values(&self) -> [f32; 13] {
        [
            self.temperature,
            self.tint,
            self.exposure,
            self.contrast,
            self.highlights,
            self.shadows,
            self.whites,
            self.blacks,
            self.texture,
            self.clarity,
            self.dehaze,
            self.vibrance,
            self.saturation,
        ]
    }

    /// The Camera Raw settings that develop only these sliders on a picture `base` describes:
    /// the document's encoding and the proxy's pixel scale carry over, everything else is default,
    /// and the sliders are bounded to their panels' ranges.
    pub fn to_camera_raw(&self, base: &CameraRaw) -> CameraRaw {
        let s = |v: f32| bounded(v, -100.0, 100.0, 0.0);
        CameraRaw {
            temperature: s(self.temperature),
            tint: s(self.tint),
            exposure: bounded(self.exposure, -5.0, 5.0, 0.0),
            contrast: s(self.contrast),
            highlights: s(self.highlights),
            shadows: s(self.shadows),
            whites: s(self.whites),
            blacks: s(self.blacks),
            texture: s(self.texture),
            clarity: s(self.clarity),
            dehaze: s(self.dehaze),
            vibrance: s(self.vibrance),
            saturation: s(self.saturation),
            pixel_scale: base.pixel_scale,
            encoding_gamma: base.encoding_gamma,
            ..CameraRaw::default()
        }
    }
}

/// One local adjustment: where it applies and what it does there.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LocalMask {
    pub shape: MaskShape,
    /// Off keeps the mask and its sliders but leaves the picture alone.
    pub enabled: bool,
    /// How much of the mask's effect shows, 0…100 %.
    pub amount: f32,
    /// Swap covered and uncovered.
    pub invert: bool,
    /// Linear: where the effect is full, as picture fractions `[x, y]`.
    pub start: [f32; 2],
    /// Linear: where the effect has faded to nothing.
    pub end: [f32; 2],
    /// Radial: the centre.
    pub center: [f32; 2],
    /// Radial: the half-axes, as fractions of the width and the height.
    pub radius: [f32; 2],
    /// Radial: degrees clockwise.
    pub angle: f32,
    /// Radial: the percentage of the radius over which the edge fades, 0…100.
    pub feather: f32,
    /// The sliders this mask applies.
    #[serde(flatten)]
    pub adjust: LocalAdjust,
}

impl Default for LocalMask {
    fn default() -> Self {
        LocalMask {
            shape: MaskShape::Linear,
            enabled: true,
            amount: 100.0,
            invert: false,
            start: [0.5, 0.3],
            end: [0.5, 0.7],
            center: [0.5, 0.5],
            radius: [0.3, 0.3],
            angle: 0.0,
            feather: 50.0,
            adjust: LocalAdjust::default(),
        }
    }
}

impl LocalMask {
    /// Rejects a mask that new settings must not carry: a number that is not finite or lies outside
    /// its slider's range. Stored settings are not checked (see [`bounded`]): they still render.
    pub fn validate(&self) -> Result<(), String> {
        let within = |name: &str, v: f32, lo: f32, hi: f32| {
            if v.is_finite() && (lo..=hi).contains(&v) { Ok(()) } else { Err(format!("{name} must be a number from {lo} to {hi}")) }
        };
        let place = -POSITION_MARGIN..=1.0 + POSITION_MARGIN;
        for (name, p) in [("start", self.start), ("end", self.end), ("center", self.center)] {
            for v in p {
                within(name, v, *place.start(), *place.end())?;
            }
        }
        for v in self.radius {
            within("radius", v, 0.001, 2.0)?;
        }
        within("angle", self.angle, -360.0, 360.0)?;
        within("feather", self.feather, 0.0, 100.0)?;
        within("amount", self.amount, 0.0, 100.0)?;
        let a = &self.adjust;
        within("exposure", a.exposure, -5.0, 5.0)?;
        for (name, v) in [
            ("temperature", a.temperature),
            ("tint", a.tint),
            ("contrast", a.contrast),
            ("highlights", a.highlights),
            ("shadows", a.shadows),
            ("whites", a.whites),
            ("blacks", a.blacks),
            ("texture", a.texture),
            ("clarity", a.clarity),
            ("dehaze", a.dehaze),
            ("vibrance", a.vibrance),
            ("saturation", a.saturation),
        ] {
            within(name, v, -100.0, 100.0)?;
        }
        Ok(())
    }

    /// A new mask of `shape` with its default placement and no adjustments yet.
    pub fn new(shape: MaskShape) -> Self {
        LocalMask { shape, ..LocalMask::default() }
    }

    /// Whether developing with this mask changes nothing: switched off, no Amount or no sliders.
    pub fn is_neutral(&self) -> bool {
        !self.enabled || self.amount.is_nan() || self.amount <= 0.0 || self.adjust.is_neutral()
    }

    /// The mask's coverage of a `w` × `h` picture, pixel by pixel.
    pub fn field(&self, w: usize, h: usize) -> Field {
        Field::new(self, w, h)
    }
}

/// A mask's geometry prepared for one picture size: coverage at any pixel in 0…1.
#[derive(Clone, Copy, Debug)]
pub enum Field {
    Linear {
        origin: [f32; 2],
        along: [f32; 2],
        inverse_length_squared: f32,
        invert: bool,
    },
    Radial {
        center: [f32; 2],
        axes: [f32; 2],
        cos: f32,
        sin: f32,
        inner: f32,
        invert: bool,
    },
    /// A degenerate shape (the two ends of a gradient on the same spot): covers nothing.
    Empty,
}

impl Field {
    fn new(m: &LocalMask, w: usize, h: usize) -> Field {
        let (wf, hf) = (w as f32, h as f32);
        let at = |p: [f32; 2]| {
            let b = |v: f32| bounded(v, -POSITION_MARGIN, 1.0 + POSITION_MARGIN, 0.5);
            [b(p[0]) * wf, b(p[1]) * hf]
        };
        match m.shape {
            MaskShape::Linear => {
                let (s, e) = (at(m.start), at(m.end));
                let along = [e[0] - s[0], e[1] - s[1]];
                let length_squared = along[0] * along[0] + along[1] * along[1];
                // Under a pixel long there is no ramp to speak of.
                if length_squared < 1.0 {
                    return Field::Empty;
                }
                Field::Linear { origin: s, along, inverse_length_squared: 1.0 / length_squared, invert: m.invert }
            }
            MaskShape::Radial => {
                let c = at(m.center);
                let half = |v: f32, size: f32| bounded(v, 0.001, 2.0, 0.3) * size;
                let axes = [half(m.radius[0], wf).max(1.0), half(m.radius[1], hf).max(1.0)];
                let (sin, cos) = bounded(m.angle, -360.0, 360.0, 0.0).to_radians().sin_cos();
                // The fade ends at the edge; a hard edge still gets about a pixel of ramp.
                let feather = bounded(m.feather, 0.0, 100.0, 50.0) / 100.0;
                let inner = (1.0 - feather).min(1.0 - 1.0 / axes[0].min(axes[1]));
                Field::Radial { center: c, axes, cos, sin, inner, invert: m.invert }
            }
        }
    }

    /// Coverage at the centre of pixel (`x`, `y`): 1 where the mask's effect is full, 0 where
    /// there is none.
    pub fn at(&self, x: usize, y: usize) -> f32 {
        let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
        match *self {
            Field::Empty => 0.0,
            Field::Linear { origin, along, inverse_length_squared, invert } => {
                let t = ((px - origin[0]) * along[0] + (py - origin[1]) * along[1]) * inverse_length_squared;
                let k = (1.0 - t).clamp(0.0, 1.0);
                if invert { 1.0 - k } else { k }
            }
            Field::Radial { center, axes, cos, sin, inner, invert } => {
                let (dx, dy) = (px - center[0], py - center[1]);
                let u = (dx * cos + dy * sin) / axes[0];
                let v = (-dx * sin + dy * cos) / axes[1];
                let d = (u * u + v * v).sqrt();
                let t = ((d - inner) / (1.0 - inner)).clamp(0.0, 1.0);
                let k = 1.0 - t * t * (3.0 - 2.0 * t);
                if invert { 1.0 - k } else { k }
            }
        }
    }
}

/// Develops the masks of `p` over `px` (`w` × `h`, straight RGBA, already globally developed).
/// Alpha is never touched. `float` is the document's depth, as for the global develop.
pub(super) fn apply(px: &mut [[f32; 4]], w: usize, h: usize, p: &CameraRaw, float: bool) {
    if w == 0 || px.len() != w.saturating_mul(h) {
        return;
    }
    for m in p.masks.iter().take(MAX_MASKS) {
        if m.is_neutral() {
            continue;
        }
        let local = m.adjust.to_camera_raw(p);
        if local.is_identity() {
            continue;
        }
        let mut developed = px.to_vec();
        super::develop(&mut developed, w, h, &local, float);
        let field = m.field(w, h);
        let amount = bounded(m.amount, 0.0, 100.0, 0.0) / 100.0;
        par_rows(px, w, 1, |y, row| {
            for (x, q) in row.iter_mut().enumerate() {
                let k = field.at(x, y) * amount;
                if k <= 0.0 {
                    continue;
                }
                let d = developed[y * w + x];
                for c in 0..3 {
                    q[c] += (d[c] - q[c]) * k;
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_raw::develop;

    fn grey(w: usize, h: usize, v: f32) -> Vec<[f32; 4]> {
        vec![[v, v, v, 1.0]; w * h]
    }

    fn exposure_mask(shape: MaskShape, stops: f32) -> LocalMask {
        LocalMask { adjust: LocalAdjust { exposure: stops, ..Default::default() }, ..LocalMask::new(shape) }
    }

    #[test]
    fn a_linear_gradient_ramps_from_full_to_nothing() {
        let m = LocalMask { start: [0.0, 0.25], end: [0.0, 0.75], ..LocalMask::new(MaskShape::Linear) };
        let f = m.field(10, 100);
        assert_eq!(f.at(5, 0), 1.0, "before the start line: full");
        assert_eq!(f.at(5, 99), 0.0, "past the end line: none");
        let mid = f.at(5, 49);
        assert!((mid - 0.5).abs() < 0.03, "halfway between the lines: {mid}");
        let (a, b, c) = (f.at(0, 30), f.at(0, 50), f.at(0, 70));
        assert!(a > b && b > c, "monotonic ramp: {a} {b} {c}");
        // the ramp has no dependence on x for a vertical gradient
        assert_eq!(f.at(0, 50), f.at(9, 50));
    }

    #[test]
    fn invert_swaps_covered_and_uncovered() {
        for shape in [MaskShape::Linear, MaskShape::Radial] {
            let plain = LocalMask::new(shape);
            let flipped = LocalMask { invert: true, ..plain.clone() };
            let (a, b) = (plain.field(40, 30), flipped.field(40, 30));
            for (x, y) in [(0, 0), (20, 15), (39, 29), (10, 25)] {
                assert!((a.at(x, y) + b.at(x, y) - 1.0).abs() < 1e-5, "{shape:?} at {x},{y}");
            }
        }
    }

    #[test]
    fn a_radial_gradient_covers_its_ellipse_and_fades_over_the_feather() {
        let m = LocalMask { center: [0.5, 0.5], radius: [0.4, 0.2], angle: 0.0, feather: 50.0, ..LocalMask::new(MaskShape::Radial) };
        let f = m.field(200, 100);
        assert_eq!(f.at(100, 50), 1.0, "centre");
        assert_eq!(f.at(0, 0), 0.0, "corner is outside");
        // the horizontal half-axis is 0.4 * 200 = 80 px, the vertical one 0.2 * 100 = 20 px
        assert_eq!(f.at(100 + 38, 50), 1.0, "inside the inner half of the radius: still full");
        assert_eq!(f.at(100 + 85, 50), 0.0, "beyond the edge");
        let edge = f.at(100 + 70, 50);
        assert!(edge > 0.0 && edge < 1.0, "inside the feather band: {edge}");
        assert_eq!(f.at(100, 50 + 25), 0.0, "the vertical axis is shorter");
    }

    #[test]
    fn a_radial_gradient_turns_with_its_angle() {
        let base = LocalMask { center: [0.5, 0.5], radius: [0.4, 0.1], angle: 0.0, feather: 0.0, ..LocalMask::new(MaskShape::Radial) };
        let turned = LocalMask { angle: 90.0, ..base.clone() };
        let (a, b) = (base.field(100, 100), turned.field(100, 100));
        assert_eq!(a.at(80, 50), 1.0, "flat ellipse reaches sideways");
        assert_eq!(a.at(50, 80), 0.0);
        assert_eq!(b.at(80, 50), 0.0, "turned a quarter, it reaches up and down");
        assert_eq!(b.at(50, 80), 1.0);
    }

    #[test]
    fn a_hard_edge_is_about_a_pixel_wide() {
        let m = LocalMask { center: [0.5, 0.5], radius: [0.25, 0.25], feather: 0.0, ..LocalMask::new(MaskShape::Radial) };
        let f = m.field(100, 100);
        let row: Vec<f32> = (0..100).map(|x| f.at(x, 50)).collect();
        assert!(row.iter().all(|v| (0.0..=1.0).contains(v)));
        assert_eq!(row[50], 1.0);
        assert_eq!(row[95], 0.0);
        assert!(row.iter().filter(|v| **v > 0.001 && **v < 0.999).count() <= 4, "{row:?}");
    }

    #[test]
    fn degenerate_and_hostile_shapes_cover_nothing_or_stay_finite() {
        let flat = LocalMask { start: [0.3, 0.3], end: [0.3, 0.3], ..LocalMask::new(MaskShape::Linear) };
        assert_eq!(flat.field(50, 50).at(10, 10), 0.0);
        let hostile = LocalMask {
            center: [f32::NAN, f32::INFINITY],
            radius: [f32::NAN, -5.0],
            angle: f32::INFINITY,
            feather: f32::NAN,
            ..LocalMask::new(MaskShape::Radial)
        };
        let f = hostile.field(40, 40);
        for y in 0..40 {
            for x in 0..40 {
                let k = f.at(x, y);
                assert!(k.is_finite() && (0.0..=1.0).contains(&k), "{x},{y}: {k}");
            }
        }
        // a 1 × 1 picture and an empty one must not divide by zero
        assert!(LocalMask::new(MaskShape::Radial).field(1, 1).at(0, 0).is_finite());
        let mut none: Vec<[f32; 4]> = Vec::new();
        apply(&mut none, 0, 0, &CameraRaw { masks: vec![exposure_mask(MaskShape::Linear, 1.0)], ..Default::default() }, false);
    }

    #[test]
    fn neutral_masks_are_an_identity_and_cost_no_pass() {
        let neutral = [
            LocalMask::new(MaskShape::Linear),
            LocalMask { enabled: false, ..exposure_mask(MaskShape::Radial, 1.0) },
            LocalMask { amount: 0.0, ..exposure_mask(MaskShape::Radial, 1.0) },
            LocalMask { amount: f32::NAN, ..exposure_mask(MaskShape::Radial, 1.0) },
        ];
        for m in neutral {
            assert!(m.is_neutral());
            let p = CameraRaw { masks: vec![m], ..Default::default() };
            assert!(p.is_identity());
            let mut px = grey(8, 8, 0.4);
            let before = px.clone();
            develop(&mut px, 8, 8, &p, false);
            assert_eq!(px, before);
        }
        assert!(!CameraRaw { masks: vec![exposure_mask(MaskShape::Linear, 0.5)], ..Default::default() }.is_identity());
    }

    #[test]
    fn a_mask_adjusts_only_where_it_covers() {
        let (w, h) = (64, 64);
        // brighten the top of the picture only
        let m = LocalMask { start: [0.5, 0.2], end: [0.5, 0.6], ..exposure_mask(MaskShape::Linear, 1.0) };
        let p = CameraRaw { masks: vec![m], ..Default::default() };
        let mut px = grey(w, h, 0.3);
        develop(&mut px, w, h, &p, false);
        assert!(px[5 * w + 32][0] > 0.4, "top is brighter: {}", px[5 * w + 32][0]);
        assert_eq!(px[60 * w + 32], [0.3, 0.3, 0.3, 1.0], "bottom is untouched");
        assert!(px.iter().all(|q| q[3] == 1.0), "alpha untouched");
        // the columns of a vertical gradient agree
        assert_eq!(px[5 * w], px[5 * w + 63]);
    }

    #[test]
    fn amount_scales_the_effect_and_masks_stack() {
        let (w, h) = (32, 32);
        let full = exposure_mask(MaskShape::Radial, 1.0);
        let half = LocalMask { amount: 50.0, ..full.clone() };
        let run = |masks: Vec<LocalMask>| {
            let mut px = grey(w, h, 0.25);
            develop(&mut px, w, h, &CameraRaw { masks, ..Default::default() }, false);
            px[16 * w + 16][0]
        };
        let (none, one, half_one, two) = (0.25, run(vec![full.clone()]), run(vec![half]), run(vec![full.clone(), full]));
        assert!(one > half_one && half_one > none, "{none} {half_one} {one}");
        assert!((half_one - (none + one) / 2.0).abs() < 1e-3, "50 % is halfway: {half_one}");
        assert!(two > one, "a second mask adds to the first: {one} {two}");
    }

    #[test]
    fn masks_follow_the_global_sliders_and_leave_colour_alone_outside() {
        let (w, h) = (16, 16);
        let m = LocalMask { invert: true, center: [0.5, 0.5], radius: [0.2, 0.2], feather: 0.0, ..exposure_mask(MaskShape::Radial, -1.0) };
        let p = CameraRaw { exposure: 0.5, masks: vec![m], ..Default::default() };
        let mut with = vec![[0.5, 0.4, 0.2, 0.7]; w * h];
        let mut without = with.clone();
        develop(&mut with, w, h, &p, false);
        develop(&mut without, w, h, &CameraRaw { exposure: 0.5, ..Default::default() }, false);
        assert_eq!(with[8 * w + 8], without[8 * w + 8], "the middle is outside the inverted mask");
        assert!(with[0][0] < without[0][0], "the corner is darkened by the local −1 stop");
        assert!(with.iter().all(|q| q[3] == 0.7));
    }

    #[test]
    fn at_most_max_masks_are_developed_and_settings_survive_json() {
        let many = CameraRaw { masks: vec![exposure_mask(MaskShape::Radial, 0.1); MAX_MASKS + 10], ..Default::default() };
        let capped = CameraRaw { masks: vec![exposure_mask(MaskShape::Radial, 0.1); MAX_MASKS], ..Default::default() };
        let (mut a, mut b) = (grey(8, 8, 0.2), grey(8, 8, 0.2));
        develop(&mut a, 8, 8, &many, false);
        develop(&mut b, 8, 8, &capped, false);
        assert_eq!(a, b);

        let p = CameraRaw { masks: vec![LocalMask { shape: MaskShape::Radial, angle: 33.0, ..exposure_mask(MaskShape::Radial, 0.5) }], ..Default::default() };
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["masks"][0]["shape"], "radial");
        assert_eq!(json["masks"][0]["exposure"], 0.5, "the sliders sit beside the shape");
        assert_eq!(serde_json::from_value::<CameraRaw>(json).unwrap(), p);
        // an old setting without masks, and a mask with only a shape, both read
        assert!(serde_json::from_value::<CameraRaw>(serde_json::json!({"exposure": 1.0})).unwrap().masks.is_empty());
        let m: LocalMask = serde_json::from_value(serde_json::json!({"shape": "radial"})).unwrap();
        assert_eq!((m.enabled, m.amount, m.radius), (true, 100.0, [0.3, 0.3]));
    }

    #[test]
    fn local_sliders_are_bounded() {
        let wild = LocalAdjust { exposure: 1e9, contrast: f32::NAN, saturation: -1e9, ..Default::default() };
        let c = wild.to_camera_raw(&CameraRaw { pixel_scale: 0.5, encoding_gamma: 1.8, ..Default::default() });
        assert_eq!((c.exposure, c.contrast, c.saturation, c.pixel_scale, c.encoding_gamma), (5.0, 0.0, -100.0, 0.5, 1.8));
        assert!(c.masks.is_empty());
    }
}
