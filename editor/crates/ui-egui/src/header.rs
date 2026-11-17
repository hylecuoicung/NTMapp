//! The header (Premiere 26 layout): Home and Import / Edit / Export on the left of a 38 pt bar, the
//! document title centred, the workspace name and quick actions on the right. On macOS the menus
//! live in the native menu bar (set up by the app); elsewhere the in-window menu bar is its own
//! row on top, above that bar, as in Premiere on Windows.

use egui::{Color32, Rect, Sense, Stroke, pos2, vec2};

use crate::FilmcraftApp;
use crate::icons::{self, Icon};
use crate::state::Mode;
use crate::theme::Tokens;

/// Height of the header bar. With `integrated_titlebar` it sits in the macOS title-bar strip.
pub const HEIGHT: f32 = 38.0;

/// Height of the in-window menu bar's row (File, Edit, …), drawn above the bar.
pub const MENU_HEIGHT: f32 = 28.0;

/// Height of the whole header: the bar, and the menu bar's row above it when there is one.
pub fn total_height(menu_bar: bool) -> f32 {
    HEIGHT + if menu_bar { MENU_HEIGHT } else { 0.0 }
}

/// Draws the header in `full` (see [`total_height`]) and handles its clicks: the menus, Home, the
/// mode tabs, the workspace name and its menu (the workspaces, then Reset to Saved Layout, Save
/// Changes to this Workspace, Save as New Workspace… and Edit Workspaces…), Discord and the quick
/// actions.
pub fn show(app: &mut FilmcraftApp, ui: &mut egui::Ui, full: Rect) {
    let t = app.tokens;
    let p = ui.painter().clone();
    // The menu bar's row on top; `rect` is the bar below it, which everything else lays out in.
    let menu_row = Rect::from_min_size(full.min, vec2(full.width(), if app.ui.show_menu_bar { MENU_HEIGHT } else { 0.0 }));
    let rect = Rect::from_min_max(pos2(full.min.x, menu_row.max.y), full.max);
    p.rect_filled(full, 0.0, t.header_bg);
    p.line_segment([full.left_bottom(), full.right_bottom()], Stroke::new(1.0, Color32::BLACK));
    let drag = ui.interact(full, egui::Id::unique("header-drag"), Sense::click_and_drag());
    if drag.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    if drag.double_clicked() {
        let max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
    }
    let mut x = rect.min.x + if app.integrated_titlebar { 104.0 } else { 14.0 };
    // Home
    let home = Rect::from_center_size(pos2(x + 10.0, rect.center().y), vec2(26.0, 26.0));
    let hresp = ui.interact(home, egui::Id::unique("hdr-home"), Sense::click()).on_hover_text(tl!("Home"));
    app.auto.add("header.home", home, "Home");
    if hresp.hovered() {
        p.rect_filled(home, 4.0, t.hover);
    }
    icons::paint(&p, home.shrink(5.0), Icon::Home, if hresp.hovered() { t.text } else { t.text_dim });
    if hresp.clicked() {
        app.ui.mode = Mode::Import;
    }
    x = home.max.x + 16.0;
    // Mode tabs (14 pt; active = primary text with a 2 pt underline under the label)
    for (m, label, shown) in [(Mode::Import, "Import", tl!("Import")), (Mode::Edit, "Edit", tl!("Edit")), (Mode::Export, "Export", tl!("Export"))] {
        let active = app.ui.mode == m;
        let galley = p.layout_no_wrap(shown.to_string(), Tokens::ui(14.0), if active { t.tab_text_active } else { t.tab_text });
        let r = Rect::from_min_size(pos2(x - 6.0, rect.min.y + 6.0), vec2(galley.size().x + 12.0, rect.height() - 12.0));
        let resp = ui.interact(r, egui::Id::unique(("mode", label)), Sense::click());
        app.auto.add(&format!("header.mode.{}", label.to_ascii_lowercase()), r, label);
        let col = if active || resp.hovered() { t.tab_text_active } else { t.tab_text };
        let gw = galley.size().x;
        p.galley_with_override_text_color(pos2(x, rect.center().y - galley.size().y / 2.0), galley, col);
        if active {
            p.line_segment([pos2(x, rect.min.y + 29.0), pos2(x + gw, rect.min.y + 29.0)], Stroke::new(2.0, t.tab_text_active));
        }
        if resp.clicked() {
            app.ui.mode = m;
        }
        x += gw + 24.0;
    }
    let left_end = x;
    // In-window menus when there is no native menu bar: their own row above the bar.
    if app.ui.show_menu_bar {
        let menu_rect = Rect::from_min_max(pos2(menu_row.min.x + 8.0, menu_row.min.y + 2.0), pos2(menu_row.max.x - 8.0, menu_row.max.y - 2.0));
        let mut mu = ui.new_child(egui::UiBuilder::new().max_rect(menu_rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
        mu.style_mut().visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        mu.style_mut().visuals.widgets.inactive.bg_stroke = Stroke::NONE;
        mu.style_mut().visuals.override_text_color = Some(t.text);
        crate::menus::menu_bar(app, &mut mu);
    }
    // Document title, centred
    let title = if app.session.is_dirty() { tlf!("{name} - Edited", name = app.session.project.name) } else { app.session.project.name.clone() };
    // Right cluster: icons at ~38 pt pitch, then workspace name in caps.
    let mut rx = rect.max.x - 14.0;
    let mut btn = |ui: &mut egui::Ui, icon: Icon, id: &str, tip: &str, app: &mut FilmcraftApp| -> egui::Response {
        let r = Rect::from_center_size(pos2(rx - 14.0, rect.center().y), vec2(28.0, 28.0));
        rx -= 38.0;
        let resp = ui.interact(r, egui::Id::unique(("hdr", id)), Sense::click()).on_hover_text(tip);
        app.auto.add(&format!("header.{id}"), r, tip);
        if resp.hovered() {
            ui.painter().rect_filled(r, 4.0, t.hover);
        }
        icons::paint(ui.painter(), r.shrink(6.0), icon, if resp.hovered() { t.text } else { t.text_dim });
        resp
    };
    if btn(ui, Icon::Fullscreen, "fullscreen", tl!("Full screen"), app).clicked() {
        let fs = ui.ctx().input(|i| i.viewport().fullscreen.unwrap_or(false));
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fs));
    }
    if btn(ui, Icon::Speaker, "volume", tl!("Volume"), app).clicked() {
        app.ui.status = tl!("Master volume: use the Audio Track Mixer").into();
    }
    if btn(ui, Icon::Search, "search", tl!("Search"), app).clicked() {
        app.show_panel(crate::dock::PanelKind::Effects);
    }
    if btn(ui, Icon::Bell, "notifications", tl!("Progress"), app).clicked() {
        app.ui.mode = Mode::Export;
    }
    // Quick Export: a popup with File Name & Location, a preset list and Export (Premiere 26)
    let qx = rect.max.x - 14.0 - 4.0 * 38.0; // the fifth button from the right
    if btn(ui, Icon::Export, "quickExport", tl!("Quick Export"), app).clicked() {
        app.ui.export.quick_open = !app.ui.export.quick_open;
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::unique("quick-export-toggled"), true));
    }
    crate::panels::export_mode::quick_export(app, ui.ctx(), pos2(qx - 340.0, rect.max.y + 4.0));
    // Appearance Mode: click for the next one (Auto, Light, Dark); the icon shows the current one.
    let (icon, mode) = match app.session.prefs.appearance.appearance_mode.as_str() {
        "auto" => (Icon::Monitor, tl!("Sync with system")),
        "light" => (Icon::Sun, tl!("Light")),
        _ => (Icon::Moon, tl!("Dark")),
    };
    let tip = format!("{}: {mode}", tl!("Appearance Mode"));
    let appearance = btn(ui, icon, "appearance", &tip, app);
    appearance.context_menu(|ui| crate::panels::settings::appearance_context_menu(app, ui));
    if appearance.clicked() {
        crate::panels::settings::cycle_appearance(app, ui.ctx());
    }
    let ws_resp = btn(ui, Icon::Workspaces, "workspaces", tl!("Workspaces"), app);
    // workspace name (caps)
    let ws = crate::i18n::to_uppercase(crate::i18n::t(&app.ui.workspace));
    let wg = p.layout_no_wrap(ws.clone(), Tokens::ui(11.0), t.text_dim);
    let wr = Rect::from_min_size(pos2(rx - wg.size().x + 10.0, rect.center().y - 10.0), vec2(wg.size().x + 8.0, 20.0));
    let wresp = ui.interact(wr, egui::Id::unique("hdr-ws-name"), Sense::click());
    app.auto.add("header.workspaceName", wr, &ws);
    p.galley_with_override_text_color(pos2(wr.min.x + 4.0, rect.center().y - wg.size().y / 2.0), wg, if wresp.hovered() { t.text } else { t.text_dim });
    // Community: a labelled Discord button, always one click away (for a brand that has a Discord).
    // The title is centred in what is left of the bar between the tabs and this button.
    let discord = ntm_brand::DISCORD.is_set().then(|| {
        let label = "Discord";
        let join = ntm_brand::DISCORD.label.trim_end_matches('…');
        let g = p.layout_no_wrap(label.to_string(), Tokens::ui(12.0), Color32::WHITE);
        let w = g.size().x + 34.0;
        let r = Rect::from_min_size(pos2(wr.min.x - w - 14.0, rect.center().y - 12.0), vec2(w, 24.0));
        let resp = ui.interact(r, egui::Id::unique("hdr-discord"), Sense::click()).on_hover_text(crate::i18n::t(join));
        app.auto.add("header.discord", r, join);
        ui.painter().rect_filled(r, 12.0, if resp.hovered() { t.accent_hover } else { t.accent });
        icons::paint(ui.painter(), Rect::from_center_size(pos2(r.min.x + 14.0, r.center().y), vec2(14.0, 14.0)), Icon::Chat, Color32::WHITE);
        ui.painter().galley(pos2(r.min.x + 25.0, r.center().y - g.size().y / 2.0), g, Color32::WHITE);
        (r, resp)
    });
    {
        // Localized menus and user titles can be wider than English. Use the actual gap.
        let left = left_end + 12.0;
        let right = discord.as_ref().map_or(wr.min.x, |(r, _)| r.min.x) - 12.0;
        if right > left + 40.0 {
            let mut job = egui::text::LayoutJob::simple_singleline(title, Tokens::ui(14.0), t.tab_text_active);
            job.wrap.max_width = right - left;
            job.wrap.max_rows = 1;
            let galley = p.layout_job(job);
            let half = galley.size().x / 2.0;
            let center = if rect.center().x - half >= left && rect.center().x + half <= right { rect.center().x } else { (left + right) / 2.0 };
            p.with_clip_rect(Rect::from_min_max(pos2(left, rect.min.y), pos2(right, rect.max.y))).galley(
                pos2(center - half, rect.center().y - galley.size().y / 2.0),
                galley,
                t.tab_text_active,
            );
        }
    }
    if discord.is_some_and(|(_, resp)| resp.clicked()) {
        crate::links::open(app, ui.ctx(), crate::links::DISCORD);
    }
    let popup_id = egui::Id::unique("workspaces-popup");
    if ws_resp.clicked() || wresp.clicked() {
        ui.ctx().data_mut(|d| d.insert_temp(popup_id, true));
    }
    let open = ui.ctx().data(|d| d.get_temp::<bool>(popup_id).unwrap_or(false));
    if open {
        let anchor = pos2(wr.min.x, rect.max.y + 4.0);
        let area = egui::Area::new(popup_id.with("area")).order(egui::Order::Foreground).fixed_pos(anchor).show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                    ui.set_min_width(220.0);
                    for w in crate::dock::names(&app.workspaces) {
                        let sel = app.ui.workspace == w;
                        if ui.selectable_label(sel, crate::i18n::t(&w)).clicked() {
                            app.set_workspace(&w);
                            ui.ctx().data_mut(|d| d.insert_temp(popup_id, false));
                        }
                    }
                    ui.separator();
                    // the actions of Window ▸ Workspaces, as in Premiere's workspace menu
                    let actions = [
                        ("reset", tl!("Reset to Saved Layout")),
                        ("saveChanges", tl!("Save Changes to this Workspace")),
                        ("saveAs", tl!("Save as New Workspace…")),
                        ("edit", tl!("Edit Workspaces…")),
                    ];
                    for (action, label) in actions {
                        let resp = ui.button(label);
                        app.auto.add(&format!("header.workspaces.{action}"), resp.rect, label);
                        if resp.clicked() {
                            ui.ctx().data_mut(|d| d.insert_temp(popup_id, false));
                            let ctx = ui.ctx().clone();
                            if let Err(e) = crate::menus::invoke(app, &ctx, &format!("window.workspace.{action}"), serde_json::json!({})) {
                                app.ui.status = e;
                            }
                        }
                    }
                });
            });
        });
        if area.response.clicked_elsewhere() && !(ws_resp.clicked() || wresp.clicked()) {
            ui.ctx().data_mut(|d| d.insert_temp(popup_id, false));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_name_uppercasing_is_turkish_aware() {
        crate::i18n::set_current(crate::i18n::Language::Tr);
        assert_eq!(crate::i18n::to_uppercase("Birleştirme"), "BİRLEŞTİRME");
        assert_eq!(crate::i18n::to_uppercase("Altyazılar ve Grafikler"), "ALTYAZILAR VE GRAFİKLER");
        crate::i18n::set_current(crate::i18n::Language::En);
        assert_eq!(crate::i18n::to_uppercase("Birleştirme"), "BIRLEŞTIRME");
    }

    /// Draws the header in a 1400-wide window and returns where each automation id landed
    /// (`[x, y, width, height]`).
    fn layout(show_menu_bar: bool) -> (FilmcraftApp, f32) {
        let mut app = FilmcraftApp::new(filmcraft_engine::Session::default());
        app.ui.show_menu_bar = show_menu_bar;
        let ctx = egui::Context::default();
        let height = total_height(show_menu_bar);
        for _ in 0..2 {
            app.auto.begin_frame();
            let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1400.0, 900.0))), ..Default::default() };
            let mut out = ctx.run_ui(input, |ui| show(&mut app, ui, Rect::from_min_size(pos2(0.0, 0.0), vec2(1400.0, height))));
            out.textures_delta.clear();
        }
        (app, height)
    }

    fn rect(app: &FilmcraftApp, id: &str) -> [f32; 4] {
        app.auto.elements.iter().find(|e| e.id == id).unwrap_or_else(|| panic!("no element {id}")).rect
    }

    /// As in Premiere on Windows, the menus (File, Edit, …) are the top row; Home and
    /// Import / Edit / Export are the bar below them, and the bar keeps its own height.
    #[test]
    fn the_menu_bar_is_its_own_row_above_the_mode_bar() {
        let (app, height) = layout(true);
        assert_eq!(height, HEIGHT + MENU_HEIGHT);
        let file = rect(&app, "menu.File");
        let help = rect(&app, "menu.Help");
        assert!(file[1] + file[3] <= MENU_HEIGHT + 0.5, "the menus sit in the top row: {file:?}");
        assert!(help[0] > file[0], "the menus run left to right: {file:?} {help:?}");
        let discord = ntm_brand::DISCORD.is_set().then_some("header.discord");
        for id in ["header.home", "header.mode.import", "header.mode.edit", "header.mode.export"].into_iter().chain(discord) {
            let r = rect(&app, id);
            assert!(r[1] >= MENU_HEIGHT - 0.5, "{id} is below the menu row: {r:?}");
            assert!(r[1] + r[3] <= height + 0.5, "{id} is inside the header: {r:?}");
        }
        // the mode tabs no longer share their row with the menus
        let edit = rect(&app, "header.mode.edit");
        assert!(file[1] + file[3] <= edit[1], "File is above Edit: {file:?} {edit:?}");
    }

    /// Without an in-window menu bar (macOS has the native one) the header is the bar alone.
    #[test]
    fn without_a_menu_bar_the_header_is_just_the_bar() {
        let (app, height) = layout(false);
        assert_eq!(height, HEIGHT);
        assert!(app.auto.elements.iter().all(|e| !e.id.starts_with("menu.")), "no in-window menus");
        let home = rect(&app, "header.home");
        assert!(home[1] >= 0.0 && home[1] + home[3] <= HEIGHT + 0.5, "{home:?}");
    }
}
