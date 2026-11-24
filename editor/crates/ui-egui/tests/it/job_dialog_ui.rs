//! NTMapp: a preview render (Sequence ▸ Render In to Out, Render Selection, …) or an export shows
//! its progress in a dialog in the middle of the window and holds the editor, clicks and keyboard
//! shortcuts alike, until it ends or is cancelled; closing the window meanwhile asks first. Other
//! jobs keep to the status bar.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Sender, channel};

use egui::{Rect, ViewportCommand, ViewportEvent, ViewportId, pos2};
use egui_kittest::Harness;
use filmcraft_engine::export_tools::{QueueItem, QueueStatus};
use filmcraft_engine::{Job, Session};
use filmcraft_export::Progress;
use filmcraft_ui_egui::FilmcraftApp;
use filmcraft_ui_egui::control::ControlRequest;
use filmcraft_ui_egui::state::Tool;
use serde_json::{Value, json};

struct Driver {
    harness: Harness<'static, FilmcraftApp>,
    tx: Sender<ControlRequest>,
    /// The commands the app sent the main window.
    window: Vec<ViewportCommand>,
    /// `FILMCRAFT_UI_SNAPSHOT_DIR`: where to save pictures of the window (GPU needed).
    snapshots: Option<std::path::PathBuf>,
}

impl Driver {
    fn new() -> Self {
        let mut s = Session::default();
        s.execute("file.openDemoProject", json!({})).unwrap();
        let (tx, rx) = channel();
        let app = FilmcraftApp::new(s).with_control(rx);
        let snapshots = std::env::var_os("FILMCRAFT_UI_SNAPSHOT_DIR").map(std::path::PathBuf::from);
        let mut b = Harness::builder().with_size(egui::vec2(1600.0, 980.0)).with_step_dt(1.0 / 60.0).with_max_steps(10_000);
        if snapshots.is_some() {
            b = b.wgpu();
        }
        let harness = b.build_eframe(move |_cc| app);
        let mut d = Driver { harness, tx, window: Vec::new(), snapshots };
        d.frames(4);
        d
    }

    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            let ctx = self.harness.ctx.clone();
            let mut raw = std::mem::take(self.harness.input_mut());
            eframe::App::raw_input_hook(self.harness.state_mut(), &ctx, &mut raw);
            *self.harness.input_mut() = raw;
            self.harness.step();
            if let Some(v) = self.harness.output().viewport_output.get(&ViewportId::ROOT) {
                self.window.extend(v.commands.iter().cloned());
            }
        }
    }

    /// The control channel's whole reply (`ok` and `result` or `error`).
    fn call(&mut self, method: &str, params: Value) -> Value {
        let (req, reply) = ControlRequest::new(method, params.clone());
        self.tx.send(req).unwrap();
        for _ in 0..600 {
            self.frames(1);
            if let Ok(v) = reply.try_recv() {
                return v;
            }
        }
        panic!("no reply to {method} {params}");
    }

    fn key(&mut self, key: &str) {
        let r = self.call("ui.key", json!({"key": key}));
        assert_eq!(r["ok"], true, "{r}");
        self.frames(3);
    }

    fn click(&mut self, id: &str) {
        let r = self.call("ui.click", json!({"id": id}));
        assert_eq!(r["ok"], true, "{id}: {r}");
        self.frames(3);
    }

    fn ids(&mut self, prefix: &str) -> Vec<String> {
        let v = self.call("ui.elements", json!({"prefix": prefix}));
        v["result"].as_array().map(|a| a.iter().filter_map(|e| e["id"].as_str().map(str::to_string)).collect()).unwrap_or_default()
    }

    /// The rect and label of the automation element `id`.
    fn element(&mut self, id: &str) -> (Rect, String) {
        let v = self.call("ui.elements", json!({"prefix": id}));
        let e = v["result"].as_array().unwrap().iter().find(|e| e["id"] == id).cloned().unwrap_or_else(|| panic!("no {id} in {v}"));
        let r: Vec<f32> = e["rect"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap() as f32).collect();
        (Rect::from_min_size(pos2(r[0], r[1]), egui::vec2(r[2], r[3])), e["label"].as_str().unwrap().to_string())
    }

    fn snapshot(&mut self, name: &str) {
        let Some(dir) = self.snapshots.clone() else { return };
        self.frames(2);
        match self.harness.render() {
            Ok(img) => {
                std::fs::create_dir_all(&dir).unwrap();
                img.save(dir.join(format!("job-dialog-{name}.png"))).unwrap();
            }
            Err(e) => eprintln!("snapshot {name} skipped: {e}"),
        }
    }

    fn app(&self) -> &FilmcraftApp {
        self.harness.state()
    }

    fn app_mut(&mut self) -> &mut FilmcraftApp {
        self.harness.state_mut()
    }

    /// A running job as the engine makes one, `done` of 200 units; an export when `export`, as
    /// `spawn_export` notes each of its jobs.
    fn start_job(&mut self, id: u64, label: &str, done: u64, export: bool) -> Arc<Progress> {
        let job = Job { id, label: label.into(), progress: Default::default(), result: Default::default() };
        job.progress.total.store(200, Ordering::Relaxed);
        job.progress.done.store(done, Ordering::Relaxed);
        let progress = job.progress.clone();
        let s = &mut self.app_mut().session;
        s.jobs.push(job);
        if export {
            s.export_offline.insert(id, Default::default());
        }
        self.frames(3);
        progress
    }

    /// The title bar's close button.
    fn close_window(&mut self) {
        self.window.clear();
        self.harness.input_mut().viewports.entry(ViewportId::ROOT).or_default().events.push(ViewportEvent::Close);
        self.frames(3);
    }

    fn sent(&self, command: &ViewportCommand) -> bool {
        self.window.contains(command)
    }
}

#[test]
fn a_preview_render_holds_the_editor_in_a_dialog_until_it_ends() {
    let mut d = Driver::new();
    let progress = d.start_job(901, "Rendering 2 preview segments", 80, false);
    *progress.status.lock().unwrap() = "Rendering segment 1 of 2".into();
    d.frames(2);
    d.snapshot("rendering");
    let (bar, label) = d.element("jobDialog.progress");
    assert!(label.starts_with("40%"), "{label}");
    assert!((bar.center().x - 800.0).abs() < 60.0 && (bar.center().y - 490.0).abs() < 160.0, "the dialog is in the middle of the window: {bar:?}");
    // keyboard shortcuts wait: C does not take the Razor
    d.key("C");
    assert_eq!(d.app().ui.tool, Tool::Selection);
    // and a click behind the dialog does nothing
    d.call("ui.click", json!({"id": "tools.Razor"}));
    d.frames(3);
    assert_eq!(d.app().ui.tool, Tool::Selection);
    // Cancel asks the job to stop and says it is stopping
    d.click("jobDialog.cancel");
    assert!(progress.cancel.load(Ordering::Relaxed));
    assert!(d.ids("jobDialog.cancel").is_empty(), "Cancelling… replaces the button");
    d.snapshot("cancelling");
    assert_eq!(d.ids("jobDialog.progress"), ["jobDialog.progress"], "the dialog stays until the job stops");
    // the job ends: the dialog goes and the editor answers again
    progress.finished.store(true, Ordering::Relaxed);
    d.frames(3);
    assert!(d.ids("jobDialog.").is_empty());
    d.key("C");
    assert_eq!(d.app().ui.tool, Tool::Razor);
}

#[test]
fn escape_cancels_a_render_but_not_an_export() {
    let mut d = Driver::new();
    let render = d.start_job(902, "Rendering audio previews", 10, false);
    d.key("Escape");
    assert!(render.cancel.load(Ordering::Relaxed));
    render.finished.store(true, Ordering::Relaxed);
    let export = d.start_job(903, "Export cut.mp4", 10, true);
    d.key("Escape");
    assert!(!export.cancel.load(Ordering::Relaxed), "a stray Escape must not throw an export away");
    assert_eq!(d.ids("jobDialog.cancel"), ["jobDialog.cancel"]);
}

#[test]
fn an_export_holds_the_editor_until_it_ends_or_is_cancelled() {
    let mut d = Driver::new();
    let progress = d.start_job(904, "Quick Export cut.mp4", 150, true);
    d.snapshot("exporting");
    let (_, label) = d.element("jobDialog.progress");
    assert!(label.starts_with("75%"), "{label}");
    d.key("C");
    assert_eq!(d.app().ui.tool, Tool::Selection, "no editing during an export");
    d.click("jobDialog.cancel");
    assert!(progress.cancel.load(Ordering::Relaxed));
    progress.finished.store(true, Ordering::Relaxed);
    d.frames(3);
    assert!(d.ids("jobDialog.").is_empty());
}

#[test]
fn cancel_stops_the_queued_exports_too() {
    let mut d = Driver::new();
    let progress = d.start_job(905, "Queue: Main → a.mp4", 20, true);
    let s = &mut d.app_mut().session;
    let (sequence, project) = (s.state.active_sequence.unwrap(), s.project.clone());
    let item = |id, status, job| QueueItem {
        id,
        sequence,
        sequence_name: "Main".into(),
        preset: String::new(),
        settings: Default::default(),
        status,
        job,
        error: None,
        project: project.clone(),
    };
    s.export_queue.items = vec![item(1, QueueStatus::Encoding, Some(905)), item(2, QueueStatus::Ready, None), item(3, QueueStatus::Ready, None)];
    s.export_queue.running = true;
    d.frames(3);
    d.snapshot("queue");
    d.click("jobDialog.cancel");
    assert!(progress.cancel.load(Ordering::Relaxed));
    let q = &d.app().session.export_queue;
    assert!(!q.running);
    assert_eq!(q.items.iter().map(|i| i.status).collect::<Vec<_>>(), [QueueStatus::Encoding, QueueStatus::Cancelled, QueueStatus::Cancelled]);
}

#[test]
fn other_jobs_keep_to_the_status_bar() {
    let mut d = Driver::new();
    let progress = d.start_job(906, "Proxies", 50, false);
    assert!(d.ids("jobDialog.").is_empty());
    assert_eq!(d.ids("status.job.cancel"), ["status.job.cancel"]);
    d.key("C");
    assert_eq!(d.app().ui.tool, Tool::Razor, "editing goes on");
    assert!(!progress.cancel.load(Ordering::Relaxed));
}

#[test]
fn closing_the_window_during_a_job_asks_first() {
    let mut d = Driver::new();
    let progress = d.start_job(907, "Export cut.mp4", 60, true);
    d.close_window();
    assert!(d.sent(&ViewportCommand::CancelClose), "{:?}", d.window);
    assert_eq!(d.ids("jobDialog.quit"), ["jobDialog.quit"]);
    d.snapshot("quit");
    // Keep Working: the export goes on
    d.click("jobDialog.keepWorking");
    assert!(d.ids("jobDialog.quit").is_empty());
    assert!(!progress.cancel.load(Ordering::Relaxed));
    // Escape is Keep Working too, never Cancel
    d.close_window();
    d.key("Escape");
    assert!(d.ids("jobDialog.quit").is_empty());
    assert!(!progress.cancel.load(Ordering::Relaxed));
    // Quit: the export is stopped, and the window closes once it has
    d.close_window();
    d.click("jobDialog.quit");
    assert!(progress.cancel.load(Ordering::Relaxed));
    assert!(!d.sent(&ViewportCommand::Close), "not before the export stops");
    progress.finished.store(true, Ordering::Relaxed);
    d.frames(3);
    assert!(d.sent(&ViewportCommand::Close), "{:?}", d.window);
    // that close goes through
    d.close_window();
    assert!(!d.sent(&ViewportCommand::CancelClose), "{:?}", d.window);
}

#[test]
fn a_job_that_ends_while_quit_is_asked_leaves_nothing_to_answer() {
    let mut d = Driver::new();
    let progress = d.start_job(908, "Rendering 1 preview segment", 60, false);
    d.close_window();
    assert_eq!(d.ids("jobDialog.quit"), ["jobDialog.quit"]);
    progress.finished.store(true, Ordering::Relaxed);
    d.frames(3);
    assert!(d.ids("jobDialog.").is_empty());
    // closing now closes (no unsaved changes)
    d.close_window();
    assert!(!d.sent(&ViewportCommand::CancelClose), "{:?}", d.window);
}
