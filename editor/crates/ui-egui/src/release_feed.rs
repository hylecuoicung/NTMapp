//! Which release of the shared repository is this app's. `hylecuoicung/NTMapp` holds more than one app, so
//! `releases/latest` can be another app's release: this app's releases are tagged
//! `<TAG_PREFIX>v<version>` and are picked out of the release list by that prefix.

/// The start of this app's release tags: `editor-` for NTM Digital (`editor-v0.20.1`),
/// `editor-pixel-` for Pixel Digital (`editor-pixel-v0.20.1`). A brand's apps only ever see its own.
pub const TAG_PREFIX: &str = ntm_brand::editor::TAG_PREFIX;

/// This app's newest published release when it is newer than `running`, else `None`: from the
/// repository's release list, the stable non-draft releases whose tag starts with [`TAG_PREFIX`].
pub fn check_for_update(http: &craft_update::Http, running: &str) -> Result<Option<craft_update::Release>, String> {
    use craft_update::http::Fetched;
    let url = format!("https://api.github.com/repos/{}/releases?per_page=100", crate::UPDATE_APP.repo);
    match http.get_api(&url, None)? {
        Fetched::Body { text, .. } => newest(&text, running),
        Fetched::RateLimited { .. } => Err("GitHub's request limit was reached".into()),
        Fetched::NotModified | Fetched::NotFound => Ok(None),
    }
}

/// [`check_for_update`] on the release list `json` (GitHub's `GET /repos/<repo>/releases`).
pub fn newest(json: &str, running: &str) -> Result<Option<craft_update::Release>, String> {
    newest_with_prefix(TAG_PREFIX, json, running)
}

fn newest_with_prefix(prefix: &str, json: &str, running: &str) -> Result<Option<craft_update::Release>, String> {
    let list: Vec<serde_json::Value> = serde_json::from_str(json).map_err(|e| format!("unreadable answer from GitHub ({e})"))?;
    let Some(running) = craft_update::parse_version(running) else { return Ok(None) };
    let mut best: Option<craft_update::Release> = None;
    for item in &list {
        if item["draft"].as_bool().unwrap_or(false) || item["prerelease"].as_bool().unwrap_or(false) {
            continue;
        }
        let Some(rest) = item["tag_name"].as_str().and_then(|t| t.strip_prefix(prefix)) else { continue };
        let Some(version) = craft_update::parse_version(rest).filter(craft_update::version::is_stable) else { continue };
        let Ok(mut release) = craft_update::parse_github_release(&item.to_string(), crate::UPDATE_APP.repo) else { continue };
        let newer = best.as_ref().and_then(|b| b.version.as_ref()).is_none_or(|b| craft_update::is_newer(&version, b));
        if newer {
            release.version = Some(version);
            best = Some(release);
        }
    }
    Ok(best.filter(|r| r.version.as_ref().is_some_and(|v| craft_update::version::offers_update(v, &running))))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shared repository's release list: both brands of both apps, a draft, strays.
    const LIST: &str = r#"[
        {"tag_name":"photo-v0.99.0","prerelease":false,"draft":false,"html_url":"https://github.com/hylecuoicung/NTMapp/releases/tag/photo-v0.99.0","assets":[]},
        {"tag_name":"photo-pixel-v0.99.0","prerelease":false,"draft":false,"assets":[]},
        {"tag_name":"editor-pixel-v0.30.0","prerelease":false,"draft":false,"html_url":"https://github.com/hylecuoicung/NTMapp/releases/tag/editor-pixel-v0.30.0",
         "assets":[{"name":"pixeldigital-editor-0.30.0-windows-x64.msi","size":1,"browser_download_url":"https://github.com/hylecuoicung/NTMapp/releases/download/editor-pixel-v0.30.0/pixeldigital-editor-0.30.0-windows-x64.msi"}]},
        {"tag_name":"editor-v0.20.2","prerelease":false,"draft":true,"assets":[]},
        {"tag_name":"editor-v0.20.1","prerelease":false,"draft":false,"html_url":"https://github.com/hylecuoicung/NTMapp/releases/tag/editor-v0.20.1",
         "assets":[{"name":"ntmdigital-editor-0.20.1-windows-x64.msi","size":1,"browser_download_url":"https://github.com/hylecuoicung/NTMapp/releases/download/editor-v0.20.1/ntmdigital-editor-0.20.1-windows-x64.msi"}]},
        {"tag_name":"editor-v0.1.0","prerelease":false,"draft":false,"assets":[]},
        {"tag_name":"v0.99.0","prerelease":false,"draft":false,"assets":[]}
    ]"#;

    #[test]
    fn picks_this_apps_newest_stable_release() {
        let r = newest_with_prefix("editor-", LIST, "0.1.0").unwrap().expect("an update");
        // the other app's newer release, the other brand's, the unprefixed tag and the draft are passed over
        assert_eq!(r.tag, "editor-v0.20.1");
        assert_eq!(r.version.clone().map(|v| v.to_string()), Some("0.20.1".into()));
        assert_eq!(r.assets.len(), 1, "{:?}", r.assets);
        assert!(r.html_url.ends_with("/editor-v0.20.1"), "{}", r.html_url);
        assert_eq!(newest_with_prefix("editor-", LIST, "0.20.1").unwrap(), None, "nothing newer");
        assert_eq!(newest_with_prefix("editor-", LIST, "0.20.2").unwrap(), None);
        assert_eq!(newest_with_prefix("editor-", "[]", "0.1.0").unwrap(), None);
        assert!(newest_with_prefix("editor-", "nope", "0.1.0").is_err());
    }

    /// A brand's copies are only ever offered their own brand's release, whichever is newer. The
    /// NTM Digital copies already out there carry the `editor-` prefix and this very matching.
    #[test]
    fn each_brand_is_offered_only_its_own_releases() {
        let ntm = newest_with_prefix("editor-", LIST, "0.1.0").unwrap().expect("NTM's release");
        assert_eq!(ntm.tag, "editor-v0.20.1", "not Pixel's newer editor-pixel-v0.30.0");
        let pixel = newest_with_prefix("editor-pixel-", LIST, "0.1.0").unwrap().expect("Pixel's release");
        assert_eq!(pixel.tag, "editor-pixel-v0.30.0", "not NTM's editor-v tags");
        assert!(pixel.assets[0].name.starts_with("pixeldigital-editor-"), "{:?}", pixel.assets);
        assert_eq!(newest_with_prefix("editor-pixel-", LIST, "0.30.0").unwrap(), None);
        // this build's own prefix is one of the two
        assert!(["editor-", "editor-pixel-"].contains(&TAG_PREFIX), "{TAG_PREFIX}");
        assert_eq!(
            newest(LIST, "0.1.0").unwrap().map(|r| r.tag),
            Some(if TAG_PREFIX == "editor-" { "editor-v0.20.1" } else { "editor-pixel-v0.30.0" }.to_string())
        );
    }
}
