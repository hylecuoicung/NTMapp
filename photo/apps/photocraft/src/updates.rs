//! Release checks through `craft-update`, the library every Craft App shares. Only public release
//! metadata is requested.
//!
//! This build is not code-signed, and the updater installs only signed packages. So a newer
//! release is announced with a Download button (`photocraft_ui_egui`'s update prompt), and the
//! address it opens is chosen here: the package of that release for this copy of the app.

use photocraft_engine::updates_cmds::{CheckFn, CheckResult, Release};

pub fn service() -> CheckFn {
    Box::new(|| {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("update check".into())
            .spawn(move || {
                let result = check();
                if let Err(e) = &result {
                    log::warn!("Update check: {e}");
                }
                let _ = tx.send(result);
            })
            .map_err(|e| format!("Could not start update check: {e}"))?;
        Ok(rx)
    })
}

/// What the last self-update came to, once: `craft-update` removes its records as it reads them.
pub fn last_update() -> Option<Result<String, String>> {
    use craft_update::self_update::{Outcome, current_version, take_outcome};
    let app = &photocraft_ui_egui::UPDATE_APP;
    let paths = craft_update::Paths::for_app(app).ok()?;
    let version = photocraft_engine::build_info::VERSION;
    let running = current_version(app, version).map_or_else(|| version.to_string(), |v| v.to_string());
    match take_outcome(&paths, &running)? {
        Outcome::Installed(installed) => Some(Ok(installed.to_string())),
        Outcome::Failed(reason) => Some(Err(reason)),
    }
}

fn check() -> CheckResult {
    let http = craft_update::Http::new(&format!("{}/Update", ntm_brand::photo::FOLDER))?;
    let found = photocraft_ui_egui::release_feed::check_for_update(&http, photocraft_engine::build_info::VERSION)?;
    let portable = crate::app_dirs::current().mode == crate::app_dirs::Mode::Portable;
    Ok(found.map(|release| Release {
        version: release.version.as_ref().map_or_else(|| release.tag.clone(), ToString::to_string),
        url: package_url(&release, cfg!(windows), portable),
    }))
}

/// Where Download goes: this platform's package of `release` (the portable zip for a portable
/// copy, else the installer), or the release page when the release has neither.
fn package_url(release: &craft_update::Release, windows: bool, portable: bool) -> String {
    let suffix = match (windows, portable) {
        (true, true) => Some("-windows-x64-portable.zip"),
        (true, false) => Some("-windows-x64.msi"),
        _ => None,
    };
    suffix.and_then(|s| release.assets.iter().find(|a| a.name.ends_with(s))).map_or_else(|| release.html_url.clone(), |a| a.url.clone())
}

#[cfg(test)]
mod tests {
    use super::package_url;

    #[test]
    fn download_opens_the_package_for_this_copy_or_the_release_page() {
        let page = "https://github.com/hylecuoicung/NTMapp/releases/tag/photo-v99.0.0";
        let file = |name: &str| craft_update::Asset {
            name: name.into(),
            url: format!("https://github.com/hylecuoicung/NTMapp/releases/download/photo-v99.0.0/{name}"),
            ..Default::default()
        };
        let release = craft_update::Release {
            tag: "v99.0.0".into(),
            html_url: page.into(),
            assets: vec![file("ntmdigital-photo-99.0.0-windows-x64.msi"), file("ntmdigital-photo-99.0.0-windows-x64-portable.zip"), file("SHA256SUMS.txt")],
            ..Default::default()
        };
        assert!(package_url(&release, true, false).ends_with("-windows-x64.msi"));
        assert!(package_url(&release, true, true).ends_with("-windows-x64-portable.zip"));
        assert_eq!(package_url(&release, false, false), page, "no package for other systems");
        let bare = craft_update::Release { html_url: page.into(), ..Default::default() };
        assert_eq!(package_url(&bare, true, false), page, "a release without files");
    }
}
