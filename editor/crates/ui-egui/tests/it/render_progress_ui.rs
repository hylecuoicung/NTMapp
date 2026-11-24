//! NTMapp: a preview render (Sequence ▸ Render In to Out, Render Selection, …) shows its progress
//! in a dialog in the middle of the window and holds the editor, clicks and keyboard shortcuts
//! alike, until it ends or is cancelled. An export keeps to the status bar.

use std::sync::atomic::Ordering;
use std::sync::mpsc::{Sender, channel};

use egui::{Rect, pos2};
use egui_kittest::Harness;
use filmcraft_engine::{Job, Session};
use filmcraft_ui_egui::FilmcraftApp;
use filmcraft_ui_egui::control::ControlRequest;
use filmcraft_ui_egui::state::Tool;
use serde_json::{Value, json};

struct Driver {
    harness: Harness<'static, FilmcraftApp>,
    tx: Sender<ControlRequest>,
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
        let mut d = Driver { harness, tx, snapshots };
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
                img.save(dir.join(format!("render-progress-{name}.png"))).unwrap();
            }
            Err(e) => eprintln!("snapshot {name} skipped: {e}"),
        }
    }

    fn app(&self) -> &FilmcraftApp {
        self.harness.state()
    }

    /// A running job as the engine makes one, `done` of 200 units.
    fn start_job(&mut self, id: u64, label: &str, done: u64) -> std::sync::Arc<filmcraft_export::Progress> {
        let job = Job { id, label: label.into(), progress: Default::default(), result: Default::default() };
        job.progress.total.store(200, Ordering::Relaxed);
        job.progress.done.store(done, Ordering::Relaxed);
        let progress = job.progress.clone();
        self.harness.state_mut().session.jobs.push(job);
        self.frames(3);
        progress
    }
}

#[test]
fn a_preview_render_holds_the_editor_in_a_dialog_until_it_ends() {
    let mut d = Driver::new();
    let progress = d.start_job(901, "Rendering 2 preview segments", 80);
    *progress.status.lock().unwrap() = "Rendering segment 1 of 2".into();
    d.frames(2);
    d.snapshot("rendering");
    let (bar, label) = d.element("render.progress");
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
    let r = d.call("ui.click", json!({"id": "render.cancel"}));
    assert_eq!(r["ok"], true, "{r}");
    d.frames(3);
    assert!(progress.cancel.load(Ordering::Relaxed));
    assert!(d.ids("render.cancel").is_empty(), "Cancelling… replaces the button");
    d.snapshot("cancelling");
    assert_eq!(d.ids("render.progress"), ["render.progress"], "the dialog stays until the job stops");
    // the job ends: the dialog goes and the editor answers again
    progress.finished.store(true, Ordering::Relaxed);
    d.frames(3);
    assert!(d.ids("render.").is_empty());
    d.key("C");
    assert_eq!(d.app().ui.tool, Tool::Razor);
}

#[test]
fn escape_cancels_the_render() {
    let mut d = Driver::new();
    let progress = d.start_job(902, "Rendering audio previews", 10);
    assert_eq!(d.ids("render.cancel"), ["render.cancel"]);
    d.key("Escape");
    assert!(progress.cancel.load(Ordering::Relaxed));
}

#[test]
fn an_export_keeps_to_the_status_bar() {
    let mut d = Driver::new();
    let progress = d.start_job(903, "Exporting", 50);
    assert!(d.ids("render.").is_empty());
    assert_eq!(d.ids("status.job.cancel"), ["status.job.cancel"]);
    d.key("C");
    assert_eq!(d.app().ui.tool, Tool::Razor, "editing goes on during an export");
    assert!(!progress.cancel.load(Ordering::Relaxed));
}
