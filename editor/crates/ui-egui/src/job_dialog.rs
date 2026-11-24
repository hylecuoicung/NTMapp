//! NTMapp: preview renders (Sequence ▸ Render In to Out, Render Effects In to Out, Render
//! Selection, Render Audio) and exports (Export, Quick Export, the export queue) show their
//! progress in a dialog in the middle of the window, and the editor cannot be used (no clicks
//! behind it, no keys) until the job ends or is cancelled. Upstream shows only the status bar's
//! small progress bar, and editing on while a job ran left people unsure whether it still worked.
//! Quitting meanwhile asks first; Quit stops the job, waits for it to stop, then goes on as a quit
//! does (the unsaved-changes prompt when there are any).
//!
//! Automation ids: `jobDialog.progress` (its label is the percent done and the time left),
//! `jobDialog.cancel`; while a quit is asked, `jobDialog.keepWorking` and `jobDialog.quit`.

use std::sync::atomic::Ordering;

use egui::{Frame, Margin, RichText};
use filmcraft_engine::Job;
use filmcraft_engine::export_tools::QueueStatus;
use serde_json::json;

use crate::FilmcraftApp;
use crate::panels::file_dialogs::{button, modal_frame};

/// How often the percent and the time left change: every frame, the estimate would flicker.
const TEXT_EVERY: f64 = 1.0;
/// How long Quit waits for a cancelled job to stop before it closes anyway.
const STOP_WAIT: f64 = 10.0;
/// Darker than egui's default, so the editor behind clearly cannot be used.
const BACKDROP: egui::Color32 = egui::Color32::from_black_alpha(140);

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Render,
    Export,
}

/// The job that holds the editor.
struct Busy {
    job: Job,
    kind: Kind,
    /// An export of the export queue, which Cancel stops with the exports waiting after it.
    queue: bool,
    /// Exports waiting in the queue to start after this one.
    waiting: usize,
}

/// Where a quit asked while a job runs has got to.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
enum Quit {
    #[default]
    None,
    /// Quit or Keep Working is asked.
    Asking,
    /// Quit was chosen: the job was cancelled at this time ([`FilmcraftApp::now`]); the window
    /// closes once it stops.
    Stopping(f64),
    /// The window was asked to close: that request goes through.
    Closing,
}

fn quit_id() -> egui::Id {
    egui::Id::unique("job-dialog-quit")
}

fn quit_state(ctx: &egui::Context) -> Quit {
    ctx.data(|d| d.get_temp(quit_id())).unwrap_or_default()
}

fn set_quit(ctx: &egui::Context, q: Quit) {
    ctx.data_mut(|d| d.insert_temp(quit_id(), q));
}

/// A preview render the engine runs as a job (`previews::render`, `previews::render_audio`).
fn is_preview_render(job: &Job) -> bool {
    job.label.starts_with("Rendering ") && job.label.contains("preview")
}

fn running(job: &&Job) -> bool {
    !job.progress.finished.load(Ordering::Relaxed)
}

fn busy(app: &FilmcraftApp) -> Option<Busy> {
    let s = &app.session;
    let q = &s.export_queue;
    // every export job, whichever way it started, goes through `spawn_export`, which notes it here
    let job = s.jobs.iter().rev().filter(running).find(|j| is_preview_render(j) || s.export_offline.contains_key(&j.id)).or_else(|| {
        // the export queue between two exports: the last one until it starts the next
        q.is_active().then(|| q.items.iter().filter_map(|i| i.job).max()).flatten().and_then(|id| s.jobs.iter().find(|j| j.id == id))
    })?;
    let kind = if is_preview_render(job) { Kind::Render } else { Kind::Export };
    let queue = kind == Kind::Export && q.items.iter().any(|i| i.job == Some(job.id));
    let waiting = if queue && q.running { q.items.iter().filter(|i| i.status == QueueStatus::Ready).count() } else { 0 };
    Some(Busy { job: job.clone(), kind, queue, waiting })
}

/// Whether the dialog holds the keyboard: shortcuts must not edit the sequence under it.
pub(crate) fn blocks_input(app: &FilmcraftApp) -> bool {
    busy(app).is_some()
}

/// Stop the job: an export of the queue stops with the exports waiting after it.
fn cancel(app: &mut FilmcraftApp, b: &Busy) {
    if b.queue
        && let Err(e) = app.session.execute("export.queue.cancel", json!({}))
    {
        app.ui.status = e.to_string();
    }
    b.job.progress.cancel.store(true, Ordering::Relaxed);
}

/// Keep the keyboard from the editor while a job holds it. The dialog blocks clicks behind it,
/// but panels read keys themselves, not only through shortcuts, so key, text and clipboard events
/// go before any of them sees one. Escape answers Keep Working while a quit is asked, and
/// otherwise cancels a preview render (quick to start again, and the parts already rendered are
/// kept); it never cancels an export, which a stray key must not throw away.
pub(crate) fn hold_keyboard(app: &mut FilmcraftApp, ctx: &egui::Context, raw: &mut egui::RawInput) {
    let Some(b) = busy(app) else { return };
    let escape = raw.events.iter().any(|e| matches!(e, egui::Event::Key { key: egui::Key::Escape, pressed: true, .. }));
    raw.events.retain(|e| {
        !matches!(e, egui::Event::Key { .. } | egui::Event::Text(_) | egui::Event::Copy | egui::Event::Cut | egui::Event::Paste(_) | egui::Event::Ime(_))
    });
    if escape {
        match quit_state(ctx) {
            Quit::Asking => set_quit(ctx, Quit::None),
            Quit::None if b.kind == Kind::Render => cancel(app, &b),
            _ => {}
        }
    }
}

/// A request to close the window (the title bar's close button, Alt+F4, the OS) while a job holds
/// the editor: cancelled, and Quit or Keep Working is asked first. Returns whether the close must
/// be cancelled. A scripted quit (`app.quit` over the control channel) does not ask.
pub(crate) fn intercept_quit(app: &FilmcraftApp, ctx: &egui::Context) -> bool {
    if app.quit_confirmed {
        return false;
    }
    match quit_state(ctx) {
        Quit::Closing => {
            set_quit(ctx, Quit::None);
            false
        }
        Quit::Asking | Quit::Stopping(_) => true,
        Quit::None if busy(app).is_some() => {
            set_quit(ctx, Quit::Asking);
            true
        }
        Quit::None => false,
    }
}

fn close_now(ctx: &egui::Context) {
    set_quit(ctx, Quit::Closing);
    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
}

/// "40% · 0:12 left", made again once a second.
fn progress_text(ctx: &egui::Context, job: &Job, now: f64) -> String {
    let id = egui::Id::unique("job-dialog-text");
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
    let now = app.now();
    let state = quit_state(ctx);
    let Some(b) = busy(app) else {
        match state {
            // the job Quit cancelled has stopped
            Quit::Stopping(_) => close_now(ctx),
            // it ended on its own while Quit was asked: there is nothing left to ask about
            Quit::Asking => set_quit(ctx, Quit::None),
            Quit::None | Quit::Closing => {}
        }
        return;
    };
    if let Quit::Stopping(since) = state
        && now - since > STOP_WAIT
    {
        close_now(ctx);
    }
    // one dialog at a time: the question takes the progress's place until it is answered
    if state == Quit::Asking {
        quit_dialog(app, ctx, &b, now);
    } else {
        progress_dialog(app, ctx, &b, now);
    }
    ctx.request_repaint_after(std::time::Duration::from_millis(150));
}

fn progress_dialog(app: &mut FilmcraftApp, ctx: &egui::Context, b: &Busy, now: f64) {
    let t = app.tokens;
    let progress = b.job.progress.clone();
    let text = progress_text(ctx, &b.job, now);
    let status = progress.status.lock().map(|s| s.clone()).unwrap_or_default();
    let cancelling = progress.cancel.load(Ordering::Relaxed);
    let (title, note) = match b.kind {
        Kind::Render => (tl!("Rendering"), tl!("Editing is paused until the render ends. Cancel keeps the parts already rendered.")),
        Kind::Export if b.queue => {
            (tl!("Exporting"), tl!("Editing is paused until the export ends. Cancel stops this export and the exports waiting after it."))
        }
        Kind::Export => (tl!("Exporting"), tl!("Editing is paused until the export ends.")),
    };
    let mut cancel_clicked = false;
    egui::Modal::new(egui::Id::unique("job-dialog")).frame(modal_frame(&t)).backdrop_color(BACKDROP).show(ctx, |ui| {
        ui.set_width(460.0);
        Frame::new().inner_margin(Margin { left: 28, right: 28, top: 24, bottom: 20 }).show(ui, |ui| {
            ui.label(RichText::new(title).size(18.0).strong().color(t.text));
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);
            ui.label(RichText::new(crate::i18n::t(&b.job.label)).size(13.5).color(t.text));
            if !status.is_empty() {
                ui.label(RichText::new(crate::i18n::t(&status)).size(12.0).color(t.text_dim));
            }
            if b.waiting > 0 {
                ui.label(RichText::new(tlf!("{n} more waiting in the export queue", n = b.waiting)).size(12.0).color(t.text_dim));
            }
            ui.add_space(10.0);
            let fraction = progress.fraction().clamp(0.0, 1.0);
            let bar = ui.add(egui::ProgressBar::new(fraction).desired_width(ui.available_width()).fill(t.accent).text(text.as_str()));
            app.auto.add("jobDialog.progress", bar.rect, &text);
            ui.add_space(8.0);
            ui.label(RichText::new(note).size(12.0).color(t.text_dim));
            ui.add_space(16.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if cancelling {
                    ui.label(RichText::new(tl!("Cancelling…")).size(13.0).color(t.text_dim));
                } else if button(app, ui, "jobDialog.cancel", tl!("Cancel"), false) {
                    cancel_clicked = true;
                }
            });
        });
    });
    // (a click beside the dialog does nothing; Escape is handled in `hold_keyboard`)
    if cancel_clicked {
        cancel(app, b);
    }
}

fn quit_dialog(app: &mut FilmcraftApp, ctx: &egui::Context, b: &Busy, now: f64) {
    let t = app.tokens;
    let (title, body) = match b.kind {
        Kind::Render => (tl!("Quit while rendering?"), tl!("Previews are still rendering. Quitting stops the render; the parts already rendered are kept.")),
        Kind::Export if b.queue => (
            tl!("Quit while exporting?"),
            tl!("An export is still running. Quitting stops it, leaves the file it was writing unfinished, and cancels the exports waiting in the queue."),
        ),
        Kind::Export => (tl!("Quit while exporting?"), tl!("An export is still running. Quitting stops it and leaves the file it was writing unfinished.")),
    };
    let mut answer = None;
    egui::Modal::new(egui::Id::unique("job-dialog-quit-modal")).frame(modal_frame(&t)).backdrop_color(BACKDROP).show(ctx, |ui| {
        ui.set_width(460.0);
        Frame::new().inner_margin(Margin { left: 28, right: 28, top: 24, bottom: 20 }).show(ui, |ui| {
            ui.label(RichText::new(title).size(18.0).strong().color(t.text));
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);
            ui.label(RichText::new(body).size(13.5).color(t.text));
            ui.add_space(18.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if button(app, ui, "jobDialog.keepWorking", tl!("Keep Working"), true) {
                    answer = Some(false);
                }
                if button(app, ui, "jobDialog.quit", tl!("Quit"), false) {
                    answer = Some(true);
                }
            });
        });
    });
    match answer {
        Some(true) => {
            cancel(app, b);
            set_quit(ctx, Quit::Stopping(now));
        }
        Some(false) => set_quit(ctx, Quit::None),
        None => {}
    }
}
