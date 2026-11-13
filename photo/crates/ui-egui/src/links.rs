//! The community link: the Help menu, the About dialog and the start screen open it.

use serde_json::{Value, json};

use crate::PhotocraftApp;

/// The community Discord.
pub const DISCORD: &str = "https://discord.com/channels/1004088367216070770/1004088367824248976";

/// Help-menu link commands: (id, url). Labels live in the engine's `shell_commands::SHELL_COMMANDS`.
pub const COMMANDS: &[(&str, &str)] = &[("help.discord", DISCORD)];

pub fn url_for(id: &str) -> Option<&'static str> {
    COMMANDS.iter().find(|c| c.0 == id).map(|c| c.1)
}

/// Open `url` in the system browser (a new tab on the web) and note it in the status bar. Prefers
/// the platform `open_url` service (reliable on Windows/macOS/Linux); falls back to `ctx.open_url`.
pub fn open(app: &mut PhotocraftApp, ctx: &egui::Context, url: &str) -> Value {
    let opened = app.services.open_url.as_ref().map(|f| f(url).is_ok()).unwrap_or(false);
    if !opened {
        ctx.open_url(egui::OpenUrl::new_tab(url));
    }
    let template = "Opened {url}";
    app.ui.status = crate::i18n::fmt(tl!(template), &[("url", url)]);
    json!({"url": url})
}

/// The prominent "Join us on Discord" button.
pub fn discord_button(app: &mut PhotocraftApp, ui: &mut egui::Ui, min_width: f32) -> egui::Response {
    let r = crate::widgets::primary_button(ui, tl!("Join us on Discord"), min_width).on_hover_text(DISCORD);
    if r.clicked() {
        open(app, ui.ctx(), DISCORD);
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_open_through_the_platform_service() {
        // Issue #14: links (Help menu, Discord button, start-page links) must open in the browser.
        // They route through the `open_url` service rather than the unreliable `ctx.open_url`.
        use std::sync::{Arc, Mutex};
        let opened: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let rec = opened.clone();
        let services = crate::Services {
            open_url: Some(Box::new(move |u: &str| {
                rec.lock().unwrap().push(u.to_string());
                Ok(())
            })),
            ..Default::default()
        };
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), services);
        let ctx = egui::Context::default();
        // Every Help-menu link id reaches the service with its URL.
        for (id, url) in COMMANDS {
            crate::menus::invoke(&mut app, &ctx, id, json!({})).unwrap();
            assert_eq!(opened.lock().unwrap().last().map(String::as_str), Some(*url), "{id}");
        }
        // The direct open() helper (Discord button / start-page link) also uses it.
        open(&mut app, &ctx, DISCORD);
        assert_eq!(opened.lock().unwrap().last().map(String::as_str), Some(DISCORD));
    }

    #[test]
    fn help_menu_lists_links_then_separator_then_system_info_and_about() {
        let app = PhotocraftApp::new(photocraft_engine::Session::new(), Default::default());
        let help: Vec<String> = crate::menus::menu_items(&app).into_iter().filter(|i| i.path == ["Help"]).map(|i| i.id).collect();
        assert_eq!(help, ["help.discord", "help.checkForUpdates", "---", "help.systemInfo", "help.about"]);
        for (id, _) in COMMANDS {
            assert!(crate::menus::is_live(id) && crate::menus::is_enabled(&app, id), "{id}");
        }
    }

    #[test]
    fn link_commands_open_their_urls() {
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), Default::default());
        let ctx = egui::Context::default();
        let url = "https://discord.com/channels/1004088367216070770/1004088367824248976";
        let r = crate::menus::invoke(&mut app, &ctx, "help.discord", serde_json::json!({})).unwrap();
        assert_eq!(r["url"], url);
        assert_eq!(app.ui.status, format!("Opened {url}"));
    }
}
