//! NTMapp: Sequence ▸ Render In to Out, Render Effects In to Out, Render Selection and Render Audio
//! show their progress in a dialog in the middle of the window, and the sequence cannot be edited
//! (no clicks behind it, no keyboard shortcuts) until the render ends or is cancelled. Upstream
//! shows only the status bar's small progress bar, and editing on while previews rendered left
//! people unsure whether the render still worked. Exports keep to the status bar and Export mode.
//!
//! Automation ids: `render.progress` (its label is the percent done and the time left),
//! `render.cancel`.

use std::sync::atomic::Ordering;

use egui::{Frame, Margin, RichText};

use crate::FilmcraftApp;
use crate::panels::file_dialogs::{button, modal_frame};

/// How often the percent and the time left change: every frame, the estimate would flicker.
const TEXT_EVERY: f64 = 1.0;

/// A preview render the engine runs as a job (`previews::render`, `previews::render_audio`).
fn is_preview_render(job: &filmcraft_engine::Job) -> bool {
    job.label.starts_with("Rendering ") && job.label.contains("preview")
}

/// The preview render still running, if any.
fn running(app: &FilmcraftApp) -> Option<filmcraft_engine::Job> {
    app.session.jobs.iter().rev().find(|j| is_preview_render(j) && !j.progress.finished.load(Ordering::Relaxed)).cloned()
}

/// Whether the render dialog holds the keyboard: shortcuts must not edit the sequence under it.
pub(crate) fn blocks_input(app: &FilmcraftApp) -> bool {
    running(app).is_some()
}

/// Keep the keyboard from the editor while previews render. The dialog blocks clicks behind it,
/// but panels read keys themselves, not only through shortcuts, so key, text and clipboard events
/// go before any of them sees one. Escape cancels the render, as the dialog's Cancel does.
pub(crate) fn hold_keyboard(app: &FilmcraftApp, raw: &mut egui::RawInput) {
    let Some(job) = running(app) else { return };
    raw.events.retain(|e| {
        if matches!(e, egui::Event::Key { key: egui::Key::Escape, pressed: true, .. }) {
            job.progress.cancel.store(true, Ordering::Relaxed);
        }
        !matches!(e, egui::Event::Key { .. } | egui::Event::Text(_) | egui::Event::Copy | egui::Event::Cut | egui::Event::Paste(_) | egui::Event::Ime(_))
    });
}

/// "40% · 0:12 left", made again once a second.
fn progress_text(ctx: &egui::Context, job: &filmcraft_engine::Job, now: f64) -> String {
    let id = egui::Id::unique("render-progress-text");
    // asked every frame though shown once a second: each call is also a reading of the job's speed
    let eta = job.progress.eta();
    let kept: Option<(u64, f64, String)> = ctx.data(|d| d.get_temp(id));
    if let Some((job_id, at, text)) = kept
        && job_id == job.id
        && (at..at + TEXT_EVERY).contains(&now)
    {
        return text;
    }
    let percent = (job.progress.fraction().clamp(0.0, 1.0) * 100.0).floor();
    let text = format!("{percent:.0}%{}", eta.map(crate::panels::left_text).unwrap_or_default());
    ctx.data_mut(|d| d.insert_temp(id, (job.id, now, text.clone())));
    text
}

pub fn show(app: &mut FilmcraftApp, ctx: &egui::Context) {
    let Some(job) = running(app) else { return };
    let t = app.tokens;
    let progress = job.progress.clone();
    let text = progress_text(ctx, &job, app.now());
    let status = progress.status.lock().map(|s| s.clone()).unwrap_or_default();
    let cancelling = progress.cancel.load(Ordering::Relaxed);
    let mut cancel = false;
    egui::Modal::new(egui::Id::unique("render-progress")).frame(modal_frame(&t)).backdrop_color(egui::Color32::from_black_alpha(140)).show(ctx, |ui| {
        ui.set_width(460.0);
        Frame::new().inner_margin(Margin { left: 28, right: 28, top: 24, bottom: 20 }).show(ui, |ui| {
            ui.label(RichText::new(tl!("Rendering")).size(18.0).strong().color(t.text));
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);
            ui.label(RichText::new(crate::i18n::t(&job.label)).size(13.5).color(t.text));
            if !status.is_empty() {
                ui.label(RichText::new(crate::i18n::t(&status)).size(12.0).color(t.text_dim));
            }
            ui.add_space(10.0);
            let fraction = progress.fraction().clamp(0.0, 1.0);
            let bar = ui.add(egui::ProgressBar::new(fraction).desired_width(ui.available_width()).fill(t.accent).text(text.as_str()));
            app.auto.add("render.progress", bar.rect, &text);
            ui.add_space(8.0);
            ui.label(RichText::new(tl!("Editing is paused until the render ends. Cancel keeps the parts already rendered.")).size(12.0).color(t.text_dim));
            ui.add_space(16.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if cancelling {
                    ui.label(RichText::new(tl!("Cancelling…")).size(13.0).color(t.text_dim));
                } else if button(app, ui, "render.cancel", tl!("Cancel"), false) {
                    cancel = true;
                }
            });
        });
    });
    // (Escape cancels in `hold_keyboard`; a click beside the dialog does nothing)
    if cancel {
        progress.cancel.store(true, Ordering::Relaxed);
    }
    ctx.request_repaint_after(std::time::Duration::from_millis(150));
}
