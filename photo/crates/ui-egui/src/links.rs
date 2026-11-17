//! The community links: the Help menu, the About dialog and the start screen open them. They are
//! the brand's (`brand/*.env`): NTM Digital has its Discord, Pixel Digital its website and
//! Facebook page. A link the brand does not have is not shown anywhere.

use std::sync::OnceLock;

use serde_json::{Value, json};

use crate::PhotocraftApp;

/// The community Discord (empty when the brand has none).
pub const DISCORD: &str = ntm_brand::DISCORD.url;

/// Help-menu link commands the brand has: (id, url), Discord first. Labels live in the engine's
/// `shell_commands::SHELL_COMMANDS`.
pub fn commands() -> &'static [(&'static str, &'static str)] {
    static COMMANDS: OnceLock<Vec<(&str, &str)>> = OnceLock::new();
    COMMANDS.get_or_init(|| {
        [("help.discord", ntm_brand::DISCORD), ("help.website", ntm_brand::WEBSITE), ("help.facebook", ntm_brand::FACEBOOK)]
            .into_iter()
            .filter(|(_, link)| link.is_set())
            .map(|(id, link)| (id, link.url))
            .collect()
    })
}

pub fn url_for(id: &str) -> Option<&'static str> {
    commands().iter().find(|c| c.0 == id).map(|c| c.1)
}

/// The brand's label of link command `id` ("Join the NTM Discord…"; empty for a link it lacks).
pub fn label_for(id: &str) -> &'static str {
    match id {
        "help.discord" => ntm_brand::DISCORD.label,
        "help.website" => ntm_brand::WEBSITE.label,
        "help.facebook" => ntm_brand::FACEBOOK.label,
        _ => "",
    }
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

/// The prominent button of link command `id` (`help.discord`: "Join us on Discord"; the others
/// carry their brand label, "Pixel Digital on Facebook").
pub fn link_button(app: &mut PhotocraftApp, ui: &mut egui::Ui, id: &str, url: &str, min_width: f32) -> egui::Response {
    let label = if id == "help.discord" { tl!("Join us on Discord") } else { crate::i18n::t(label_for(id).trim_end_matches('…')) };
    let r = crate::widgets::primary_button(ui, label, min_width).on_hover_text(url);
    if r.clicked() {
        open(app, ui.ctx(), url);
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
        for (id, url) in commands() {
            crate::menus::invoke(&mut app, &ctx, id, json!({})).unwrap();
            assert_eq!(opened.lock().unwrap().last().map(String::as_str), Some(*url), "{id}");
        }
        // The direct open() helper (link button / start-page link) also uses it.
        let first = commands()[0].1;
        open(&mut app, &ctx, first);
        assert_eq!(opened.lock().unwrap().last().map(String::as_str), Some(first));
    }

    #[test]
    fn help_menu_lists_links_then_separator_then_system_info_and_about() {
        let app = PhotocraftApp::new(photocraft_engine::Session::new(), Default::default());
        let help: Vec<String> = crate::menus::menu_items(&app).into_iter().filter(|i| i.path == ["Help"]).map(|i| i.id).collect();
        // the brand's links lead the menu; a link it lacks is not there at all
        let mut want: Vec<&str> = commands().iter().map(|c| c.0).collect();
        want.extend(["help.checkForUpdates", "---", "help.systemInfo", "help.about"]);
        assert_eq!(help, want);
        for (id, _) in commands() {
            assert!(crate::menus::is_live(id) && crate::menus::is_enabled(&app, id), "{id}");
        }
    }

    #[test]
    fn link_commands_open_their_urls() {
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), Default::default());
        let ctx = egui::Context::default();
        let (id, url) = commands()[0];
        let r = crate::menus::invoke(&mut app, &ctx, id, serde_json::json!({})).unwrap();
        assert_eq!(r["url"], url);
        assert_eq!(app.ui.status, format!("Opened {url}"));
        // every link the brand has is a command with a label, and the labels of the others are empty
        for (id, url) in commands() {
            assert!(!label_for(id).is_empty() && url.starts_with("https://"), "{id}");
        }
        for id in ["help.discord", "help.website", "help.facebook"] {
            assert_eq!(url_for(id).is_none(), label_for(id).is_empty(), "{id}");
        }
    }
}
