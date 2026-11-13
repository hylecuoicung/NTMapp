//! Camera Raw's Local Adjustments: the panel section that lists the masks with their shape and
//! sliders, and the handles drawn on the preview to place them (see
//! [`photocraft_algo::camera_raw::masks`] for what a mask is and how it develops).

use egui::{Color32, Pos2, Rect as ERect, Sense, Shape, Stroke, pos2, vec2};
use photocraft_algo::camera_raw::CameraRaw;
use photocraft_algo::camera_raw::masks::{LocalMask, MAX_MASKS, MaskShape};

use super::controls::row;
use crate::theme::Tokens;
use crate::widgets;

/// How close (points) to a handle a press grabs it.
const HANDLE_RADIUS: f32 = 7.0;

/// The panel's name for `shape`.
fn shape_name(shape: MaskShape) -> &'static str {
    match shape {
        MaskShape::Linear => "Linear Gradient",
        MaskShape::Radial => "Radial Gradient",
    }
}

/// The list's names: "Linear Gradient 1", "Radial Gradient 1", "Linear Gradient 2", …
fn names(masks: &[LocalMask]) -> Vec<String> {
    let (mut linear, mut radial) = (0, 0);
    masks
        .iter()
        .map(|m| {
            let n = match m.shape {
                MaskShape::Linear => &mut linear,
                MaskShape::Radial => &mut radial,
            };
            *n += 1;
            format!("{} {}", crate::i18n::tr_ctx(crate::i18n::current(), "cameraRaw", shape_name(m.shape)), n)
        })
        .collect()
}

/// A percentage slider over a fraction stored as `0..=1`; `scale` maps the stored value to the
/// shown one (2 turns a half-axis into a width).
fn percent(ui: &mut egui::Ui, dirty: &mut bool, label: &str, v: &mut f32, scale: f32, range: std::ops::RangeInclusive<f32>) {
    let mut shown = *v * 100.0 * scale;
    let before = shown;
    row(ui, dirty, label, &mut shown, range, None);
    if shown != before {
        *v = shown / (100.0 * scale);
    }
}

/// The section's body: add buttons, the list, and the selected mask's shape and sliders.
/// `selected` is the mask being edited (and shown on the preview).
pub(super) fn section_body(ui: &mut egui::Ui, p: &mut CameraRaw, selected: &mut Option<usize>, dirty: &mut bool) {
    let tr = |s: &'static str| crate::i18n::tr_ctx(crate::i18n::current(), "cameraRaw", s);
    ui.horizontal_wrapped(|ui| {
        for shape in [MaskShape::Linear, MaskShape::Radial] {
            let room = p.masks.len() < MAX_MASKS;
            if ui.add_enabled(room, egui::Button::new(tr(shape_name(shape)))).on_hover_text(tl!("Add a mask to the picture")).clicked() {
                p.masks.push(LocalMask::new(shape));
                *selected = Some(p.masks.len() - 1);
                *dirty = true;
            }
        }
    });
    if p.masks.is_empty() {
        let t = Tokens::get(ui.ctx());
        ui.label(egui::RichText::new(tl!("Add a gradient, then drag its handles on the picture.")).color(t.text_faint).size(11.0));
        *selected = None;
        return;
    }
    if selected.is_none_or(|i| i >= p.masks.len()) {
        *selected = Some(0);
    }
    let labels = names(&p.masks);
    let mut remove = None;
    for (i, label) in labels.iter().enumerate() {
        ui.horizontal(|ui| {
            if ui.selectable_label(*selected == Some(i), label).clicked() {
                *selected = Some(i);
            }
            if let Some(m) = p.masks.get_mut(i)
                && widgets::checkbox(ui, &mut m.enabled, tl!("On")).changed()
            {
                *dirty = true;
            }
            if ui.small_button(tl!("Delete")).clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        p.masks.remove(i);
        *selected = if p.masks.is_empty() { None } else { Some(i.min(p.masks.len() - 1)) };
        *dirty = true;
        return;
    }
    let Some(m) = selected.and_then(|i| p.masks.get_mut(i)) else { return };
    widgets::section_label(ui, tr("Mask"));
    row(ui, dirty, "Amount", &mut m.amount, 0.0..=100.0, None);
    if widgets::checkbox(ui, &mut m.invert, tl!("Invert")).changed() {
        *dirty = true;
    }
    match m.shape {
        MaskShape::Linear => {
            percent(ui, dirty, "Start X", &mut m.start[0], 1.0, 0.0..=100.0);
            percent(ui, dirty, "Start Y", &mut m.start[1], 1.0, 0.0..=100.0);
            percent(ui, dirty, "End X", &mut m.end[0], 1.0, 0.0..=100.0);
            percent(ui, dirty, "End Y", &mut m.end[1], 1.0, 0.0..=100.0);
        }
        MaskShape::Radial => {
            percent(ui, dirty, "Center X", &mut m.center[0], 1.0, 0.0..=100.0);
            percent(ui, dirty, "Center Y", &mut m.center[1], 1.0, 0.0..=100.0);
            percent(ui, dirty, "Width", &mut m.radius[0], 2.0, 1.0..=200.0);
            percent(ui, dirty, "Height", &mut m.radius[1], 2.0, 1.0..=200.0);
            row(ui, dirty, "Angle", &mut m.angle, -180.0..=180.0, None);
            row(ui, dirty, "Feather", &mut m.feather, 0.0..=100.0, None);
        }
    }
    widgets::section_label(ui, tr("Adjustments"));
    let a = &mut m.adjust;
    row(ui, dirty, "Exposure", &mut a.exposure, -5.0..=5.0, None);
    row(ui, dirty, "Contrast", &mut a.contrast, -100.0..=100.0, None);
    row(ui, dirty, "Highlights", &mut a.highlights, -100.0..=100.0, None);
    row(ui, dirty, "Shadows", &mut a.shadows, -100.0..=100.0, None);
    row(ui, dirty, "Whites", &mut a.whites, -100.0..=100.0, None);
    row(ui, dirty, "Blacks", &mut a.blacks, -100.0..=100.0, None);
    row(ui, dirty, "Temperature", &mut a.temperature, -100.0..=100.0, None);
    row(ui, dirty, "Tint", &mut a.tint, -100.0..=100.0, None);
    row(ui, dirty, "Vibrance", &mut a.vibrance, -100.0..=100.0, None);
    row(ui, dirty, "Saturation", &mut a.saturation, -100.0..=100.0, None);
    row(ui, dirty, "Texture", &mut a.texture, -100.0..=100.0, None);
    row(ui, dirty, "Clarity", &mut a.clarity, -100.0..=100.0, None);
    row(ui, dirty, "Dehaze", &mut a.dehaze, -100.0..=100.0, None);
}

/// Picture fractions to screen points, and back, for a picture shown in `image`.
fn to_screen(image: ERect, f: [f32; 2]) -> Pos2 {
    pos2(image.left() + f[0] * image.width(), image.top() + f[1] * image.height())
}

fn to_fraction(image: ERect, p: Pos2) -> [f32; 2] {
    let bounded = |v: f32| if v.is_finite() { v.clamp(-1.0, 2.0) } else { 0.5 };
    [bounded((p.x - image.left()) / image.width().max(1.0)), bounded((p.y - image.top()) / image.height().max(1.0))]
}

/// A draggable round handle at `at`. Returns the pointer position while it is being dragged.
fn handle(ui: &mut egui::Ui, id: egui::Id, at: Pos2, filled: bool, color: Color32) -> Option<Pos2> {
    let hit = ERect::from_center_size(at, vec2(HANDLE_RADIUS * 2.0 + 4.0, HANDLE_RADIUS * 2.0 + 4.0));
    let response = ui.interact(hit, id, Sense::drag()).on_hover_cursor(egui::CursorIcon::Grab);
    let painter = ui.painter();
    painter.circle_stroke(at, HANDLE_RADIUS + 1.0, Stroke::new(2.0, Color32::BLACK));
    if filled {
        painter.circle_filled(at, HANDLE_RADIUS, color);
    } else {
        painter.circle_stroke(at, HANDLE_RADIUS, Stroke::new(2.0, color));
    }
    if response.dragged() { response.interact_pointer_pos() } else { None }
}

/// A line drawn light over dark so it reads on any picture.
fn guide(ui: &egui::Ui, points: Vec<Pos2>, color: Color32, closed: bool) {
    let painter = ui.painter();
    let (shadow, line) = (Stroke::new(3.0, Color32::from_black_alpha(160)), Stroke::new(1.5, color));
    if closed {
        painter.add(Shape::closed_line(points.clone(), shadow));
        painter.add(Shape::closed_line(points, line));
    } else {
        painter.add(Shape::line(points.clone(), shadow));
        painter.add(Shape::line(points, line));
    }
}

/// Draws the selected mask over the preview (`image` is where the picture is shown) and lets its
/// handles be dragged. Returns whether a drag changed the mask.
pub(super) fn overlay(ui: &mut egui::Ui, p: &mut CameraRaw, selected: Option<usize>, image: ERect, t: &Tokens) -> bool {
    let Some(index) = selected else { return false };
    let Some(m) = p.masks.get_mut(index) else { return false };
    if !m.enabled || image.width() < 8.0 || image.height() < 8.0 {
        return false;
    }
    let color = if t.pro { Color32::from_rgb(255, 214, 90) } else { t.accent };
    let id = egui::Id::unique(("camera-raw-mask-handle", index));
    let mut changed = false;
    match m.shape {
        MaskShape::Linear => {
            let (s, e) = (to_screen(image, m.start), to_screen(image, m.end));
            let along = e - s;
            if along.length() >= 1.0 {
                // The full-effect and no-effect lines run across the whole preview.
                let across = vec2(-along.y, along.x).normalized() * (image.width() + image.height());
                guide(ui, vec![s - across, s + across], color, false);
                guide(ui, vec![e - across, e + across], color.gamma_multiply(0.55), false);
            }
            guide(ui, vec![s, e], color, false);
            if let Some(q) = handle(ui, id.with("start"), s, true, color) {
                m.start = to_fraction(image, q);
                changed = true;
            }
            if let Some(q) = handle(ui, id.with("end"), e, false, color) {
                m.end = to_fraction(image, q);
                changed = true;
            }
        }
        MaskShape::Radial => {
            let c = to_screen(image, m.center);
            let (sin, cos) = m.angle.clamp(-360.0, 360.0).to_radians().sin_cos();
            let axes = vec2(m.radius[0].clamp(0.001, 2.0) * image.width(), m.radius[1].clamp(0.001, 2.0) * image.height());
            let at = |u: f32, v: f32| pos2(c.x + u * axes.x * cos - v * axes.y * sin, c.y + u * axes.x * sin + v * axes.y * cos);
            let outline: Vec<Pos2> = (0..96)
                .map(|k| {
                    let a = k as f32 / 96.0 * std::f32::consts::TAU;
                    at(a.cos(), a.sin())
                })
                .collect();
            guide(ui, outline, color, true);
            let feather = 1.0 - m.feather.clamp(0.0, 100.0) / 100.0;
            if feather > 0.02 && feather < 0.98 {
                let inner: Vec<Pos2> = (0..96)
                    .map(|k| {
                        let a = k as f32 / 96.0 * std::f32::consts::TAU;
                        at(a.cos() * feather, a.sin() * feather)
                    })
                    .collect();
                guide(ui, inner, color.gamma_multiply(0.5), true);
            }
            if let Some(q) = handle(ui, id.with("center"), c, true, color) {
                m.center = to_fraction(image, q);
                changed = true;
            }
            // The width and height handles sit on the ellipse's axes and drag along them.
            let along_x = vec2(cos, sin);
            let along_y = vec2(-sin, cos);
            if let Some(q) = handle(ui, id.with("width"), at(1.0, 0.0), false, color) {
                m.radius[0] = ((q - c).dot(along_x) / image.width()).clamp(0.01, 2.0);
                changed = true;
            }
            if let Some(q) = handle(ui, id.with("height"), at(0.0, 1.0), false, color) {
                m.radius[1] = ((q - c).dot(along_y) / image.height()).clamp(0.01, 2.0);
                changed = true;
            }
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_are_named_by_their_kind_and_order() {
        let masks = [LocalMask::new(MaskShape::Linear), LocalMask::new(MaskShape::Radial), LocalMask::new(MaskShape::Linear)];
        assert_eq!(names(&masks), ["Linear Gradient 1", "Radial Gradient 1", "Linear Gradient 2"]);
    }

    #[test]
    fn picture_fractions_and_screen_points_round_trip() {
        let image = ERect::from_min_size(pos2(100.0, 50.0), vec2(400.0, 200.0));
        let p = to_screen(image, [0.25, 0.75]);
        assert_eq!(p, pos2(200.0, 200.0));
        assert_eq!(to_fraction(image, p), [0.25, 0.75]);
        assert_eq!(to_fraction(image, pos2(f32::NAN, f32::INFINITY)), [0.5, 0.5], "not a number: the middle");
        // far outside the picture the fraction stays within what a mask may hold
        assert_eq!(to_fraction(image, pos2(1.0e6, -1.0e6)), [2.0, -1.0]);
    }
}
