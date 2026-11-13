//! Home (header "Import", the screen the app starts on): the New Project page. The project's name
//! and the folder it is created in, with Create; Open Project; the recent projects; the community
//! link. Create writes the project file at once, so the project has a place on disk from the
//! start and Save needs no dialog.

use egui::{Align2, Color32, Rect, Sense, pos2, vec2};
use serde_json::{Value, json};

use crate::FilmcraftApp;
use crate::theme::Tokens;

/// What the New Project form holds between frames (kept with the UI state).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NewProjectDraft {
    pub name: String,
    /// The folder the project file is created in; `None` means [`default_location`].
    pub location: Option<String>,
}

impl Default for NewProjectDraft {
    fn default() -> Self {
        Self { name: "Untitled".into(), location: None }
    }
}

/// Where new projects go unless another folder is chosen: the Documents folder, else the home
/// folder; empty when neither is known (the project then starts unsaved, as File ▸ New does).
pub fn default_location() -> String {
    let Some(home) = filmcraft_engine::media_browser::std_home_dir() else { return String::new() };
    let docs = std::path::Path::new(&home).join("Documents");
    if docs.is_dir() { docs.to_string_lossy().into_owned() } else { home }
}

/// The project's name as typed, made safe as a file name: trimmed, the characters no file system
/// takes replaced by `-`, and `Untitled` when nothing is left.
pub fn file_name(typed: &str) -> String {
    let name: String =
        typed.trim().chars().map(|c| if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() { '-' } else { c }).collect();
    let name = name.trim_matches(|c: char| c == '.' || c.is_whitespace()).to_string();
    if name.is_empty() { "Untitled".into() } else { name }
}

/// The file a project named `name` is created as in `location`.
pub fn project_path(location: &str, name: &str) -> std::path::PathBuf {
    std::path::Path::new(location).join(format!("{name}.fcproj"))
}

const FORM_W: f32 = 440.0;
const FIELD_H: f32 = 32.0;

pub fn show(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    ui.painter().rect_filled(rect, t.radius, t.panel_bg);
    let ctx = ui.ctx().clone();
    let left = rect.min.x + 32.0;
    let form_w = FORM_W.min(rect.width() - 64.0).max(200.0);
    ui.painter().text(pos2(left, rect.min.y + 44.0), Align2::LEFT_CENTER, tl!("New project"), Tokens::semibold(26.0), t.text);
    ui.painter().text(
        pos2(left, rect.min.y + 76.0),
        Align2::LEFT_CENTER,
        tl!("Name the project and choose where to keep it, then Create. Drop media anywhere in the window once it is open."),
        Tokens::ui(13.0),
        t.text_dim,
    );

    // Project name
    let mut y = rect.min.y + 116.0;
    ui.painter().text(pos2(left, y), Align2::LEFT_CENTER, tl!("Project name"), Tokens::semibold(13.0), t.text);
    let name_r = Rect::from_min_size(pos2(left, y + 14.0), vec2(form_w, FIELD_H));
    field(ui, name_r, &t);
    let mut child = ui.new_child(
        egui::UiBuilder::new().max_rect(name_r.shrink2(vec2(10.0, 4.0))).layout(egui::Layout::left_to_right(egui::Align::Center)).id_salt("new-project-name"),
    );
    let r = child.add(egui::TextEdit::singleline(&mut app.ui.new_project.name).frame(egui::Frame::NONE).desired_width(f32::INFINITY).font(Tokens::ui(14.0)));
    let r = crate::widgets::accessible_name(r, "Project name");
    crate::widgets::select_all_on_focus(&child, &r, &app.ui.new_project.name);
    let enter = r.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter));
    app.auto.add("newProject.name", name_r, &app.ui.new_project.name);
    y += 14.0 + FIELD_H + 22.0;

    // Project location
    ui.painter().text(pos2(left, y), Align2::LEFT_CENTER, tl!("Project location"), Tokens::semibold(13.0), t.text);
    let location = app.ui.new_project.location.clone().unwrap_or_else(default_location);
    let browse_w = 110.0;
    let loc_r = Rect::from_min_size(pos2(left, y + 14.0), vec2(form_w - browse_w - 8.0, FIELD_H));
    field(ui, loc_r, &t);
    let shown: String = if location.is_empty() { tl!("Not saved until you choose Save").to_string() } else { location.clone() };
    ui.painter().with_clip_rect(loc_r.shrink2(vec2(10.0, 0.0))).text(
        pos2(loc_r.min.x + 10.0, loc_r.center().y),
        Align2::LEFT_CENTER,
        &shown,
        Tokens::ui(13.0),
        if location.is_empty() { t.text_dim } else { t.text },
    );
    ui.interact(loc_r, egui::Id::unique("new-project-location"), Sense::hover()).on_hover_text(&shown);
    app.auto.add("newProject.location", loc_r, &location);
    let browse_r = Rect::from_min_size(pos2(loc_r.max.x + 8.0, loc_r.min.y), vec2(browse_w, FIELD_H));
    if button(app, ui, browse_r, "newProject.browse", tl!("Browse…"), false) {
        let at = Some(location.clone()).filter(|l| !l.is_empty());
        let _ = app.pick(crate::native_dialogs::FileDialog::Folder { at }, |app, paths| {
            if let Some(p) = paths.into_iter().next() {
                app.ui.new_project.location = Some(p);
            }
            Ok(Value::Null)
        });
    }
    y += 14.0 + FIELD_H + 28.0;

    // Create, Open Project
    let create_r = Rect::from_min_size(pos2(left, y), vec2(150.0, 36.0));
    let open_r = Rect::from_min_size(pos2(create_r.max.x + 10.0, y), vec2(170.0, 36.0));
    if button(app, ui, create_r, "newProject.create", tl!("Create"), true) || enter {
        create(app, &ctx);
    }
    if button(app, ui, open_r, "import.file.open", tl!("Open project…"), false) {
        let r = crate::menus::invoke(app, &ctx, "file.open", json!({}));
        // a cancelled file dialog answers null: stay on Home
        if r.is_ok_and(|v| !v.is_null()) {
            app.ui.mode = crate::state::Mode::Edit;
        }
    }
    let form_bottom = y + 36.0;

    // Recent projects: a column to the right of the form, below it in a narrow window.
    let wide = rect.max.x - (left + form_w + 48.0) >= 320.0;
    let (rx, ry, rw) = if wide {
        (left + form_w + 48.0, rect.min.y + 116.0, rect.max.x - 32.0 - (left + form_w + 48.0))
    } else {
        (left, form_bottom + 40.0, rect.width() - 64.0)
    };
    let recent_bottom = recent(app, ui, rx, ry, rw);
    let y = if wide { form_bottom.max(recent_bottom) } else { recent_bottom } + 48.0;

    // Community and project links.
    // This build's logo (see `brand.rs`), then the section title.
    let logo = crate::brand::paint_wordmark(ui, pos2(left, y), 15.0, app.ui.dark);
    let tx = logo.map_or(left, |r| r.max.x + 12.0);
    ui.painter().text(pos2(tx, y), Align2::LEFT_CENTER, tl!("Community"), Tokens::semibold(15.0), t.text);
    let mut x = left;
    for (i, (id, label, url)) in crate::links::ALL.iter().enumerate() {
        let primary = i == 0;
        let label = if primary { tl!("Join us on Discord") } else { crate::i18n::t(label) };
        let g = ui.painter().layout_no_wrap(label.to_string(), Tokens::ui(13.0), t.text);
        let r = Rect::from_min_size(pos2(x, y + 18.0), vec2(g.size().x + 44.0, 34.0));
        let resp = ui.interact(r, egui::Id::unique(("imp-link", *id)), Sense::click()).on_hover_text(*url);
        app.auto.add(&format!("import.link.{}", id.trim_start_matches("help.")), r, label);
        let (bg, fg) = if primary {
            (if resp.hovered() { t.accent_hover } else { t.accent }, Color32::WHITE)
        } else {
            (if resp.hovered() { t.hover } else { t.tl_header_bg }, t.text)
        };
        ui.painter().rect_filled(r, 8.0, bg);
        let icon = [crate::icons::Icon::Chat, crate::icons::Icon::Globe, crate::icons::Icon::Globe, crate::icons::Icon::Code][i.min(3)];
        crate::icons::paint(ui.painter(), Rect::from_center_size(pos2(r.min.x + 20.0, r.center().y), vec2(16.0, 16.0)), icon, fg);
        ui.painter().galley(pos2(r.min.x + 34.0, r.center().y - g.size().y / 2.0), g, fg);
        if resp.clicked() {
            crate::links::open(app, &ctx, url);
        }
        x = r.max.x + 12.0;
    }
}

/// The background of a form field.
fn field(ui: &egui::Ui, r: Rect, t: &Tokens) {
    ui.painter().rect(r, 6.0, t.field_bg, egui::Stroke::new(1.0, t.field_border), egui::StrokeKind::Inside);
}

/// A push button at `r`; `primary` is the accented one. Registers `id` for automation.
fn button(app: &mut FilmcraftApp, ui: &mut egui::Ui, r: Rect, id: &str, label: &str, primary: bool) -> bool {
    let t = app.tokens;
    let resp = ui.interact(r, egui::Id::unique(("imp-button", id)), Sense::click());
    app.auto.add(id, r, label);
    let (bg, fg) = if primary {
        (if resp.hovered() { t.accent_hover } else { t.accent }, Color32::WHITE)
    } else {
        (if resp.hovered() { t.hover } else { t.tl_header_bg }, t.text)
    };
    ui.painter().rect_filled(r, 6.0, bg);
    ui.painter().text(r.center(), Align2::CENTER_CENTER, label, Tokens::semibold(13.5), fg);
    resp.clicked()
}

/// Create: a new project named as typed, written to the chosen folder right away, then Edit.
/// Unsaved work in the open project goes through Close Project first: it asks Save / Don't Save /
/// Cancel and leaves Home showing, and Create is clicked again after the answer.
fn create(app: &mut FilmcraftApp, ctx: &egui::Context) {
    if app.session.is_dirty() {
        let _ = crate::menus::invoke(app, ctx, "file.closeProject", json!({}));
        return;
    }
    let name = file_name(&app.ui.new_project.name);
    let location = app.ui.new_project.location.clone().unwrap_or_else(default_location);
    let path = (!location.is_empty()).then(|| project_path(&location, &name));
    if let Some(p) = &path {
        if p.exists() {
            app.ui.status = tlf!("A project named {name} already exists in {folder}", name, folder = location);
            return;
        }
        if let Err(e) = std::fs::create_dir_all(&location) {
            app.ui.status = tlf!("Can't create the folder {folder}: {e}", folder = location, e);
            return;
        }
    }
    if crate::menus::invoke(app, ctx, "file.newProject", json!({"name": name})).is_err() {
        return;
    }
    if let Some(p) = path {
        // the error, if any, is in the status bar; the project is open either way
        let _ = crate::menus::invoke(app, ctx, "file.saveAs", json!({"path": p.to_string_lossy()}));
    }
    app.ui.new_project.name = NewProjectDraft::default().name;
    app.ui.mode = crate::state::Mode::Edit;
}

/// Recent projects listed on Home, as in File ▸ Open Recent.
const RECENT_ROWS: usize = 8;
const RECENT_ROW_H: f32 = 28.0;

/// Home's Recent list at (`x`, `y`), `width` wide: the recent projects (newest first, file name with
/// its folder beside it), a click opens one. Returns where the list ends (`y` when it is empty).
fn recent(app: &mut FilmcraftApp, ui: &mut egui::Ui, x: f32, y: f32, width: f32) -> f32 {
    let paths: Vec<String> = app.session.prefs.general.recent_projects.iter().take(RECENT_ROWS).cloned().collect();
    if paths.is_empty() {
        return y;
    }
    let t = app.tokens;
    ui.painter().text(pos2(x, y), Align2::LEFT_CENTER, tl!("Recent"), Tokens::semibold(15.0), t.text);
    let mut open = None;
    for (i, path) in paths.iter().enumerate() {
        let row = Rect::from_min_size(pos2(x - 8.0, y + 16.0 + i as f32 * RECENT_ROW_H), vec2(560.0_f32.min(width).max(0.0), RECENT_ROW_H));
        let name = crate::menus::recent_name(path);
        let resp = ui.interact(row, egui::Id::unique(("imp-recent", i)), Sense::click()).on_hover_text(path.as_str());
        app.auto.add(&format!("import.recent.{i}"), row, &name);
        if resp.hovered() {
            ui.painter().rect_filled(row, 4.0, t.hover);
        }
        let p = ui.painter().with_clip_rect(row);
        crate::icons::paint(&p, Rect::from_center_size(pos2(row.min.x + 16.0, row.center().y), vec2(14.0, 14.0)), crate::icons::Icon::Sequence, t.text_dim);
        let name_r = p.text(pos2(row.min.x + 32.0, row.center().y), Align2::LEFT_CENTER, &name, Tokens::ui(13.0), t.text);
        if let Some(folder) = std::path::Path::new(path).parent() {
            p.text(pos2(name_r.max.x + 12.0, row.center().y), Align2::LEFT_CENTER, folder.to_string_lossy(), Tokens::ui(11.5), t.text_dim);
        }
        if resp.clicked() {
            open = Some(path.clone());
        }
    }
    if let Some(path) = open {
        let ctx = ui.ctx().clone();
        // unsaved changes: Save / Don't Save / Cancel first, asked over the project they were made in;
        // Home switches to Edit once the chosen project opens, and stays after Cancel
        if crate::panels::clip_dialogs::ask_before_open(app, &path).is_some() {
            if let Some(info) = app.ui.clip_dialog.as_mut().and_then(|d| d.info.as_object_mut()) {
                info.insert("editAfterOpen".into(), json!(true));
            }
        } else if crate::menus::invoke(app, &ctx, "file.open", json!({"path": path})).is_ok() {
            app.ui.mode = crate::state::Mode::Edit;
        }
    }
    y + 16.0 + paths.len() as f32 * RECENT_ROW_H
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_names_become_safe_file_names() {
        assert_eq!(file_name("  My film  "), "My film");
        assert_eq!(file_name("a/b:c*d?e\"f<g>h|i\\j"), "a-b-c-d-e-f-g-h-i-j");
        assert_eq!(file_name(""), "Untitled");
        assert_eq!(file_name(" ... "), "Untitled");
        assert_eq!(project_path("/tmp/x", "cut").to_string_lossy(), "/tmp/x/cut.fcproj");
    }

    /// Create writes `<location>/<name>.fcproj`, opens it as the project, and switches to Edit;
    /// a second Create with the same name refuses to overwrite the file.
    #[test]
    fn create_writes_the_project_where_asked_and_opens_it() {
        let dir = std::env::temp_dir().join(format!("filmcraft-new-project-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut app = FilmcraftApp::new(filmcraft_engine::Session::default());
        app.ui.mode = crate::state::Mode::Import;
        app.ui.new_project.name = "First cut".into();
        app.ui.new_project.location = Some(dir.join("sub").to_string_lossy().into_owned());
        let ctx = egui::Context::default();
        create(&mut app, &ctx);
        let file = dir.join("sub").join("First cut.fcproj");
        assert!(file.is_file(), "{}", file.display());
        assert_eq!(app.session.path.as_deref(), Some(file.to_string_lossy().as_ref()));
        assert_eq!(app.session.project.name, "First cut");
        assert_eq!(app.ui.mode, crate::state::Mode::Edit);
        assert!(!app.session.is_dirty());
        assert_eq!(app.ui.new_project.name, "Untitled", "the name is ready for the next project");

        app.ui.mode = crate::state::Mode::Import;
        app.ui.new_project.name = "First cut".into();
        create(&mut app, &ctx);
        assert_eq!(app.ui.mode, crate::state::Mode::Import, "an existing file is not replaced");
        assert!(app.ui.status.contains("already exists"), "{}", app.ui.status);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// With no folder known the project starts unsaved, like File ▸ New ▸ Project.
    #[test]
    fn create_without_a_location_opens_an_unsaved_project() {
        let mut app = FilmcraftApp::new(filmcraft_engine::Session::default());
        app.ui.mode = crate::state::Mode::Import;
        app.ui.new_project.name = "x".into();
        app.ui.new_project.location = Some(String::new());
        create(&mut app, &egui::Context::default());
        assert_eq!(app.session.path, None);
        assert_eq!(app.session.project.name, "x");
        assert_eq!(app.ui.mode, crate::state::Mode::Edit);
    }
}
