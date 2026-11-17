//! This build's logo for the About dialog and Home: the mark (the app icon's artwork,
//! `assets/app-icon/`, the same for every brand) followed by the brand's product name as text.

use egui::emath::GuiRounding as _;

/// The name the wordmark spells.
pub const WORDMARK: &str = ntm_brand::editor::PRODUCT;

/// The mark as `packaging/icons.sh` renders it from the vector, smallest first: (side in pixels, PNG).
const MARKS: &[(u32, &[u8])] = &[
    (24, include_bytes!("../../../assets/app-icon/hicolor/24x24/apps/ai.storyteller.filmcraft.png")),
    (32, include_bytes!("../../../assets/app-icon/hicolor/32x32/apps/ai.storyteller.filmcraft.png")),
    (48, include_bytes!("../../../assets/app-icon/hicolor/48x48/apps/ai.storyteller.filmcraft.png")),
    (64, include_bytes!("../../../assets/app-icon/hicolor/64x64/apps/ai.storyteller.filmcraft.png")),
    (128, include_bytes!("../../../assets/app-icon/hicolor/128x128/apps/ai.storyteller.filmcraft.png")),
    (256, include_bytes!("../../../assets/app-icon/hicolor/256x256/apps/ai.storyteller.filmcraft.png")),
];

/// The mark's side as a multiple of the wordmark's text height.
const MARK_SCALE: f32 = 1.6;

/// The mark as a texture about `px` pixels square, made once per size. Drawing one large image
/// several times smaller would shimmer: this backend has no mipmaps. Sizes are rounded up to a
/// multiple of 8 and capped at the largest render, so zooming the UI cannot pile up textures without
/// limit. A size `packaging/icons.sh` rendered is used as is; any other is scaled down from the next
/// larger render.
fn mark(ctx: &egui::Context, px: u32) -> Option<egui::TextureHandle> {
    let px = px.clamp(8, 256).div_ceil(8) * 8;
    let id = egui::Id::unique(("ntmdigital-mark", px));
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(t);
    }
    let (_, png) = MARKS.iter().find(|(side, _)| *side >= px).or(MARKS.last())?;
    let mut img = image::load_from_memory_with_format(png, image::ImageFormat::Png).ok()?.to_rgba8();
    // Premultiply before resampling, or the transparent surround darkens the mark's edges.
    for p in img.pixels_mut() {
        let a = u16::from(p.0[3]);
        for c in p.0.iter_mut().take(3) {
            *c = u8::try_from((u16::from(*c) * a + 127) / 255).unwrap_or(u8::MAX);
        }
    }
    if img.width() != px || img.height() != px {
        img = image::imageops::resize(&img, px, px, image::imageops::FilterType::Triangle);
    }
    let color = egui::ColorImage::from_rgba_premultiplied([img.width() as usize, img.height() as usize], img.as_raw());
    let tex = ctx.load_texture(format!("ntmdigital-mark-{px}"), color, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, tex.clone()));
    Some(tex)
}

/// Draw the logo with its left edge at `left_center` (vertically centred): the mark, then the name
/// `height` points tall. Returns the drawn rect.
pub fn paint_wordmark(ui: &egui::Ui, left_center: egui::Pos2, height: f32, dark: bool) -> Option<egui::Rect> {
    let ink = if dark { egui::Color32::WHITE } else { egui::Color32::from_rgb(0x1a, 0x1a, 0x1a) };
    let side = (height * MARK_SCALE).round();
    let mut drawn: Option<egui::Rect> = None;
    let mut text_x = left_center.x;
    // A float-to-int cast saturates, and `mark` clamps the size.
    let px = (side * ui.ctx().pixels_per_point()).ceil() as u32;
    if let Some(tex) = mark(ui.ctx(), px) {
        // On whole pixels, so a texture drawn at its own size is not blurred by the sampler.
        let min = egui::pos2(left_center.x, left_center.y - side / 2.0).round_to_pixels(ui.ctx().pixels_per_point());
        let r = egui::Rect::from_min_size(min, egui::vec2(side, side));
        ui.painter().image(tex.id(), r, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
        text_x = r.max.x + (height * 0.4).round();
        drawn = Some(r);
    }
    let galley = ui.painter().layout_no_wrap(WORDMARK.to_string(), crate::theme::Tokens::semibold(height), ink);
    let r = egui::Rect::from_min_size(egui::pos2(text_x, left_center.y - galley.size().y / 2.0), galley.size());
    ui.painter().galley(r.min, galley, ink);
    Some(drawn.map_or(r, |m| m.union(r)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_render_of_the_mark_has_the_size_it_is_listed_under() {
        let mut last = 0;
        for (side, png) in MARKS {
            assert!(*side > last, "renders are listed smallest first");
            last = *side;
            let img = image::load_from_memory_with_format(png, image::ImageFormat::Png).unwrap().to_rgba8();
            assert_eq!((img.width(), img.height()), (*side, *side));
            // Transparent corners: the mark is a shape, not a tile.
            assert_eq!(img.get_pixel(0, 0).0[3], 0);
        }
    }

    #[test]
    fn mark_textures_come_in_a_bounded_set_of_sizes() {
        let ctx = egui::Context::default();
        for (asked, got) in [(0, 8), (1, 8), (24, 24), (25, 32), (48, 48), (255, 256), (100_000, 256)] {
            let tex = mark(&ctx, asked).unwrap();
            assert_eq!(tex.size(), [got, got], "asked for {asked} px");
        }
    }
}
