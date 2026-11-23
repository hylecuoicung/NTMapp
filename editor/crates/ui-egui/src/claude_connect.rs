//! Help ▸ Connect Claude Desktop…: registers this app's MCP server (`<exe>-cli mcp`, the headless
//! mode of `docs/agents.md`) in Claude Desktop's `claude_desktop_config.json`, so that people who
//! are not developers never edit that JSON by hand. Fork-only (the NTM Digital / Pixel Digital
//! builds).
//!
//! Where Claude Desktop keeps that file depends on how it was installed: `%APPDATA%\Claude` for
//! the classic installer, `%LOCALAPPDATA%\Packages\Claude_*\LocalCache\Roaming\Claude` for the
//! packaged (MSIX) one, `%LOCALAPPDATA%\Claude-Data\claude_*` for builds that keep their data
//! there, and `~/Library/Application Support/Claude` on macOS. Every one of those folders that
//! exists gets the entry; a folder this Claude does not read only holds a file nobody uses.
//!
//! Connecting keeps everything else in the file (Claude's own preferences, other MCP servers)
//! and copies the file to `claude_desktop_config.json.bak` before changing it. It also drops the
//! entries an uninstalled copy of the editor left behind (their program is gone, so Claude would
//! only report a failed server) and other entries that run this same program under another
//! name. A file that is not valid JSON is left alone until the person asks to replace it.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use crate::FilmcraftApp;

/// The Help menu command.
pub const COMMAND: &str = "help.connectClaude";
/// Claude Desktop's settings file, in each of [`config_dirs`].
pub const FILE: &str = "claude_desktop_config.json";

/// What the last Connect did, shown until closed (`ui.claudeConnect`).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    /// The name Claude lists this app's tools under.
    pub server: String,
    /// The command-line tool Claude runs; empty when it is missing.
    pub cli: String,
    /// The folders that were looked at (Replace uses them again).
    pub config_dirs: Vec<String>,
    /// Settings files that now hold the entry.
    pub connected: Vec<String>,
    /// Entries taken out: an uninstalled copy's, or this program's under another name.
    pub removed: Vec<String>,
    /// Settings files left as they were because they are not valid JSON, with the reason.
    pub broken: Vec<(String, String)>,
    /// Why nothing could be connected; empty when something was.
    pub error: String,
}

/// The name Claude lists the tools under: the brand's program name, such as `pixeldigital-editor`.
pub fn server_name() -> &'static str {
    ntm_brand::editor::EXE
}

/// The command-line tool beside the running app: `<exe>-cli.exe` as installed, or `filmcraft-cli`
/// in a developer build.
pub fn cli_beside(exe: &Path) -> Option<PathBuf> {
    let dir = exe.parent()?;
    let ext = std::env::consts::EXE_SUFFIX;
    [format!("{}-cli{ext}", ntm_brand::editor::EXE), format!("filmcraft-cli{ext}")].into_iter().map(|name| dir.join(name)).find(|p| p.is_file())
}

/// Claude Desktop's settings folders that exist on this computer, read from the environment
/// variables `env` returns (see the module docs for the places).
pub fn config_dirs(env: &dyn Fn(&str) -> Option<String>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let var = |k: &str| env(k).filter(|v| !v.trim().is_empty()).map(PathBuf::from);
    if let Some(appdata) = var("APPDATA") {
        out.push(appdata.join("Claude"));
    }
    if let Some(local) = var("LOCALAPPDATA") {
        for dir in subdirs(&local.join("Packages"), "claude_") {
            out.push(dir.join("LocalCache").join("Roaming").join("Claude"));
        }
        out.extend(subdirs(&local.join("Claude-Data"), "claude"));
    }
    if let Some(home) = var("HOME") {
        out.push(home.join("Library").join("Application Support").join("Claude"));
        if var("XDG_CONFIG_HOME").is_none() {
            out.push(home.join(".config").join("Claude"));
        }
    }
    if let Some(config) = var("XDG_CONFIG_HOME") {
        out.push(config.join("Claude"));
    }
    out.retain(|d| d.is_dir());
    out.dedup();
    out
}

/// The folders in `dir` whose names start with `prefix`, ignoring case, in name order (none when
/// `dir` cannot be read).
fn subdirs(dir: &Path, prefix: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut found: Vec<PathBuf> =
        entries.flatten().filter(|e| e.file_name().to_string_lossy().to_lowercase().starts_with(prefix)).map(|e| e.path()).filter(|p| p.is_dir()).collect();
    found.sort();
    found
}

/// Whether `command` runs a copy of the editor's command-line tool, whatever its brand.
fn is_editor_cli(command: &str) -> bool {
    let name = command.rsplit(['/', '\\']).next().unwrap_or(command).to_lowercase();
    let name = name.strip_suffix(".exe").unwrap_or(&name);
    name.ends_with("-editor-cli") || name == "filmcraft-cli"
}

/// Whether two program paths name the same file (case and slash direction aside on Windows).
fn same_program(a: &str, b: &str) -> bool {
    if cfg!(windows) { a.replace('/', "\\").eq_ignore_ascii_case(&b.replace('/', "\\")) } else { a == b }
}

/// The settings `text` with `server` registered to run `cli mcp`, and the names of the entries
/// taken out ([`Report::removed`]). `Ok(None)` when it already was and nothing was taken out. `Err`
/// when `text` is not a JSON object (with the line and column), so it is never overwritten by
/// mistake. `exists` says whether a program file is there.
pub fn register(text: &str, server: &str, cli: &str, exists: &dyn Fn(&str) -> bool) -> Result<(Option<String>, Vec<String>), String> {
    // Notepad may save a byte-order mark, which no JSON reader accepts
    let text = text.trim_start_matches('\u{feff}');
    let mut root = if text.trim().is_empty() {
        Value::Object(Map::new())
    } else {
        serde_json::from_str::<Value>(text).map_err(|e| tlf!("not valid JSON at line {line}, column {column}", line = e.line(), column = e.column()))?
    };
    let Some(settings) = root.as_object_mut() else { return Err(tl!("not a JSON object").to_string()) };
    let servers = settings.entry("mcpServers").or_insert_with(|| Value::Object(Map::new()));
    let Some(servers) = servers.as_object_mut() else { return Err(tl!("its \"mcpServers\" is not a JSON object").to_string()) };
    let removed: Vec<String> = servers
        .iter()
        .filter(|(name, _)| name.as_str() != server)
        .filter_map(|(name, entry)| {
            let command = entry.get("command").and_then(Value::as_str)?;
            (is_editor_cli(command) && (same_program(command, cli) || !exists(command))).then(|| name.clone())
        })
        .collect();
    for name in &removed {
        servers.remove(name);
    }
    let wanted = json!({"command": cli, "args": ["mcp"]});
    if servers.get(server) == Some(&wanted) && removed.is_empty() {
        return Ok((None, removed));
    }
    servers.insert(server.to_string(), wanted);
    let mut out = serde_json::to_string_pretty(&root).map_err(|e| e.to_string())?;
    out.push('\n');
    Ok((Some(out), removed))
}

/// Register this app in the settings file of each of `dirs`. A file that is not valid JSON is
/// left alone and listed in [`Report::broken`], unless `replace_broken`: then it is copied to
/// `.bak` and started afresh with this app's entry.
pub fn connect(dirs: &[PathBuf], cli: Option<&Path>, replace_broken: bool) -> Report {
    let mut report = Report {
        server: server_name().to_string(),
        cli: cli.map(|c| c.display().to_string()).unwrap_or_default(),
        config_dirs: dirs.iter().map(|d| d.display().to_string()).collect(),
        ..Report::default()
    };
    if cli.is_none() {
        report.error = tlf!(
            "The command-line tool ({name}) is missing from this app's folder. Install NTM Digital Editor again, then try once more.",
            name = format!("{}-cli{}", ntm_brand::editor::EXE, std::env::consts::EXE_SUFFIX)
        );
        return report;
    }
    if dirs.is_empty() {
        report.error =
            tl!("Claude Desktop was not found on this computer. Install it from claude.ai/download, open it once and sign in, then try again.").to_string();
        return report;
    }
    let mut failures: Vec<String> = Vec::new();
    for dir in dirs {
        let path = dir.join(FILE);
        match connect_file(&path, &report.cli, replace_broken) {
            Ok(removed) => {
                report.connected.push(path.display().to_string());
                for name in removed {
                    if !report.removed.contains(&name) {
                        report.removed.push(name);
                    }
                }
            }
            Err(FileError::Broken(why)) => report.broken.push((path.display().to_string(), why)),
            Err(FileError::Io(why)) => failures.push(format!("{}: {why}", path.display())),
        }
    }
    if report.connected.is_empty() && report.broken.is_empty() {
        report.error = tlf!("Claude's settings could not be changed: {why}", why = failures.join("; "));
    }
    report
}

enum FileError {
    /// Not valid JSON: left as it was.
    Broken(String),
    /// Reading or writing failed.
    Io(String),
}

/// Register this app in one settings file; returns the entries taken out.
fn connect_file(path: &Path, cli: &str, replace_broken: bool) -> Result<Vec<String>, FileError> {
    let old = match std::fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(FileError::Io(e.to_string())),
    };
    let exists = |p: &str| Path::new(p).is_file();
    let server = server_name();
    let (new, removed) = match register(old.as_deref().unwrap_or_default(), server, cli, &exists) {
        Ok(r) => r,
        Err(why) if !replace_broken => return Err(FileError::Broken(why)),
        Err(_) => register("", server, cli, &exists).map_err(FileError::Broken)?,
    };
    let Some(new) = new else { return Ok(removed) };
    if old.is_some() {
        let mut backup = path.as_os_str().to_owned();
        backup.push(".bak");
        std::fs::copy(path, PathBuf::from(backup)).map_err(|e| FileError::Io(e.to_string()))?;
    }
    // write beside it, then swap it in: a failed write never leaves Claude a half-written file
    let mut temp = path.as_os_str().to_owned();
    temp.push(".tmp");
    let temp = PathBuf::from(temp);
    std::fs::write(&temp, new).map_err(|e| FileError::Io(e.to_string()))?;
    std::fs::rename(&temp, path).map_err(|e| {
        let _ = std::fs::remove_file(&temp);
        FileError::Io(e.to_string())
    })?;
    Ok(removed)
}

/// Help ▸ Connect Claude Desktop…. Params (all optional, for automation and tests): `configDirs`
/// (the folders to use instead of finding Claude's), `cli` (the program to register instead of the
/// one beside this app), `replace` (start broken settings files afresh, keeping a copy).
pub fn run(app: &mut FilmcraftApp, params: &Value) -> Result<Value, String> {
    let dirs: Vec<PathBuf> = match params.get("configDirs").and_then(Value::as_array) {
        Some(list) => list.iter().filter_map(Value::as_str).map(PathBuf::from).collect(),
        None => config_dirs(&|k| std::env::var(k).ok()),
    };
    let cli = match params.get("cli").and_then(Value::as_str) {
        Some(c) => Some(PathBuf::from(c)),
        None => std::env::current_exe().ok().and_then(|exe| cli_beside(&exe)),
    };
    let replace = params.get("replace").and_then(Value::as_bool).unwrap_or(false);
    let report = connect(&dirs, cli.as_deref(), replace);
    app.ui.status = if report.connected.is_empty() {
        tl!("Claude Desktop is not connected").to_string()
    } else {
        tl!("Claude Desktop is connected: quit it and open it again").to_string()
    };
    app.ui.claude_connect = Some(report.clone());
    serde_json::to_value(&report).map_err(|e| e.to_string())
}

/// The result window, until closed. Automation ids: `claudeConnect.close`, `claudeConnect.replace`.
pub fn show(app: &mut FilmcraftApp, ctx: &egui::Context) {
    let Some(report) = app.ui.claude_connect.clone() else { return };
    let mut close = false;
    let mut replace = false;
    let mut elems: Vec<(&str, egui::Rect, String)> = Vec::new();
    egui::Window::new(tl!("Connect Claude Desktop"))
        .id(egui::Id::unique("claude-connect"))
        .collapsible(false)
        .resizable(false)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(crate::widgets::dialog_center(ctx))
        .show(ctx, |ui| {
            ui.set_max_width(520.0);
            if !report.connected.is_empty() {
                ui.label(egui::RichText::new(tl!("Claude Desktop can now use NTM Digital Editor.")).strong());
                ui.add_space(4.0);
                ui.label(tl!("1. Quit Claude Desktop completely: right-click its icon next to the clock and choose Quit."));
                ui.label(tl!("2. Open Claude Desktop again."));
                ui.label(tlf!("3. Ask Claude, for example: “Use {server} to list the File commands.”", server = report.server));
                ui.add_space(4.0);
                ui.label(tl!(
                    "Claude edits project files, not this window: save and close a project here before asking Claude to change it, then open it again to see the result."
                ));
            }
            if !report.error.is_empty() {
                ui.label(&report.error);
            }
            if !report.removed.is_empty() {
                ui.add_space(4.0);
                ui.label(tlf!(
                    "Removed old entries for a copy that is no longer installed or that duplicated this one: {names}",
                    names = report.removed.join(", ")
                ));
            }
            if !report.broken.is_empty() {
                ui.add_space(4.0);
                ui.label(tl!("These Claude settings files are damaged, so they were left as they are:"));
                for (path, why) in &report.broken {
                    ui.label(egui::RichText::new(format!("{path} ({why})")).small());
                }
                ui.label(tl!("Replace starts them afresh with this app's entry and keeps a copy of each as .bak."));
            }
            ui.add_space(4.0);
            for path in &report.connected {
                ui.label(egui::RichText::new(tlf!("Settings: {path}", path)).small().weak());
            }
            if !report.cli.is_empty() {
                ui.label(egui::RichText::new(tlf!("Program: {cli}", cli = report.cli)).small().weak());
            }
            ui.add_space(6.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let r = ui.button(tl!("Close"));
                elems.push(("claudeConnect.close", r.rect, "Close".into()));
                close |= r.clicked();
                if !report.broken.is_empty() && !report.cli.is_empty() {
                    let r = ui.button(tl!("Replace"));
                    elems.push(("claudeConnect.replace", r.rect, "Replace".into()));
                    replace |= r.clicked();
                }
            });
        });
    for (id, r, label) in elems {
        app.auto.add(id, r, &label);
    }
    if replace {
        let params = json!({"configDirs": report.config_dirs, "cli": report.cli, "replace": true});
        let _ = run(app, &params);
    } else if close || crate::widgets::escape_closes(ctx) {
        app.ui.claude_connect = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLI: &str = r"C:\Users\u\AppData\Local\Programs\PixelDigital\PixelDigitalEditor\pixeldigital-editor-cli.exe";

    fn nothing_exists(_: &str) -> bool {
        false
    }

    fn servers(text: &str) -> Map<String, Value> {
        let v: Value = serde_json::from_str(text).unwrap();
        v["mcpServers"].as_object().unwrap().clone()
    }

    #[test]
    fn an_empty_or_missing_file_gets_just_this_server() {
        for text in ["", "  \n", "{}"] {
            let (out, removed) = register(text, "pixeldigital-editor", CLI, &nothing_exists).unwrap();
            let s = servers(&out.unwrap());
            assert_eq!(s.len(), 1);
            assert_eq!(s["pixeldigital-editor"], json!({"command": CLI, "args": ["mcp"]}));
            assert!(removed.is_empty());
        }
    }

    #[test]
    fn claudes_own_settings_and_other_servers_are_kept() {
        let text = r#"{
  "coworkUserFilesPath": "C:\\Users\\u\\Claude",
  "preferences": {"menuBarEnabled": false, "sidebarMode": "chat"},
  "mcpServers": {"other": {"command": "node", "args": ["server.js"], "env": {"K": "v"}}}
}"#;
        let (out, _) = register(text, "pixeldigital-editor", CLI, &nothing_exists).unwrap();
        let v: Value = serde_json::from_str(&out.unwrap()).unwrap();
        assert_eq!(v["coworkUserFilesPath"], r"C:\Users\u\Claude");
        assert_eq!(v["preferences"], json!({"menuBarEnabled": false, "sidebarMode": "chat"}));
        assert_eq!(v["mcpServers"]["other"], json!({"command": "node", "args": ["server.js"], "env": {"K": "v"}}));
        assert_eq!(v["mcpServers"]["pixeldigital-editor"]["command"], CLI);
    }

    #[test]
    fn an_uninstalled_copys_entry_goes_and_an_installed_one_stays() {
        let ntm = r"C:\Users\u\AppData\Local\Programs\NTMDigital\NTMDigitalEditor\ntmdigital-editor-cli.exe";
        let text = json!({"mcpServers": {
            "ntm-editor": {"command": ntm, "args": ["mcp"]},
            "unrelated-missing": {"command": r"C:\gone\tool.exe", "args": []},
        }})
        .to_string();
        // NTM's copy was uninstalled: its entry would only fail in Claude
        let (out, removed) = register(&text, "pixeldigital-editor", CLI, &nothing_exists).unwrap();
        assert_eq!(removed, ["ntm-editor"]);
        let s = servers(&out.unwrap());
        assert!(!s.contains_key("ntm-editor"));
        assert!(s.contains_key("unrelated-missing"), "only editor tools are cleaned up");
        // both brands installed side by side: each keeps its own entry
        let (out, removed) = register(&text, "pixeldigital-editor", CLI, &|p| p == ntm).unwrap();
        assert!(removed.is_empty());
        assert!(servers(&out.unwrap()).contains_key("ntm-editor"));
    }

    #[test]
    fn the_same_program_under_another_name_is_not_listed_twice() {
        let other_case = CLI.to_uppercase();
        let text = json!({"mcpServers": {"my-editor": {"command": if cfg!(windows) { other_case.as_str() } else { CLI }, "args": ["mcp"]}}}).to_string();
        let (out, removed) = register(&text, "pixeldigital-editor", CLI, &|_| true).unwrap();
        assert_eq!(removed, ["my-editor"]);
        assert_eq!(servers(&out.unwrap()).keys().collect::<Vec<_>>(), ["pixeldigital-editor"]);
    }

    #[test]
    fn connecting_again_changes_nothing() {
        let (first, _) = register("{}", "pixeldigital-editor", CLI, &nothing_exists).unwrap();
        let first = first.unwrap();
        assert_eq!(register(&first, "pixeldigital-editor", CLI, &nothing_exists).unwrap(), (None, vec![]));
        // a moved install points the entry at the new place
        let moved = r"C:\Program Files\PixelDigital\PixelDigitalEditor\pixeldigital-editor-cli.exe";
        let (out, _) = register(&first, "pixeldigital-editor", moved, &nothing_exists).unwrap();
        assert_eq!(servers(&out.unwrap())["pixeldigital-editor"]["command"], moved);
    }

    #[test]
    fn a_byte_order_mark_is_read_and_not_written_back() {
        let (out, _) = register("\u{feff}{\"a\": 1}", "pixeldigital-editor", CLI, &nothing_exists).unwrap();
        let out = out.unwrap();
        assert!(!out.starts_with('\u{feff}'));
        assert_eq!(serde_json::from_str::<Value>(&out).unwrap()["a"], 1);
    }

    #[test]
    fn damaged_settings_are_refused_with_where_they_break() {
        // the mistake people make by hand: a block pasted after the closing brace
        let text = "{\n  \"preferences\": {}\n}\n{ \"mcpServers\": {} }\n";
        let err = register(text, "pixeldigital-editor", CLI, &nothing_exists).unwrap_err();
        assert!(err.contains("line 4"), "{err}");
        for bad in ["[1, 2]", "\"text\"", "{\"mcpServers\": [1]}", "{\"a\": 1,}"] {
            assert!(register(bad, "pixeldigital-editor", CLI, &nothing_exists).is_err(), "{bad}");
        }
    }

    #[test]
    fn editor_tools_are_recognised_by_their_file_name() {
        for yes in [CLI, r"C:\x\NTMDIGITAL-EDITOR-CLI.EXE", "/usr/bin/filmcraft-cli", "pixeldigital-editor-cli"] {
            assert!(is_editor_cli(yes), "{yes}");
        }
        for no in ["node", r"C:\x\pixeldigital-photo-cli.exe", r"C:\x\editor-cli-helper.exe", ""] {
            assert!(!is_editor_cli(no), "{no}");
        }
    }

    #[test]
    fn claudes_folders_are_found_for_each_way_it_installs() {
        let tmp = tempfile::tempdir().unwrap();
        let roaming = tmp.path().join("Roaming");
        let local = tmp.path().join("Local");
        let classic = roaming.join("Claude");
        let msix = local.join("Packages").join("Claude_pzs8sxrjxfjjc").join("LocalCache").join("Roaming").join("Claude");
        let data = local.join("Claude-Data").join("claude_pzs8sxrjxfjjc");
        for d in [&classic, &msix, &data, &local.join("Packages").join("Other_1"), &local.join("Claude-Data").join("logs")] {
            std::fs::create_dir_all(d).unwrap();
        }
        let env = |k: &str| match k {
            "APPDATA" => Some(roaming.display().to_string()),
            "LOCALAPPDATA" => Some(local.display().to_string()),
            _ => None,
        };
        assert_eq!(config_dirs(&env), [classic.clone(), msix, data]);
        // a computer without Claude Desktop has none
        std::fs::remove_dir_all(&roaming).unwrap();
        std::fs::remove_dir_all(&local).unwrap();
        assert!(config_dirs(&env).is_empty());
        assert!(config_dirs(&|_| None).is_empty());
    }

    #[test]
    fn connect_writes_each_folder_keeps_a_copy_and_leaves_damaged_files_until_replaced() {
        let tmp = tempfile::tempdir().unwrap();
        let (fresh, kept, damaged) = (tmp.path().join("a"), tmp.path().join("b"), tmp.path().join("c"));
        for d in [&fresh, &kept, &damaged] {
            std::fs::create_dir_all(d).unwrap();
        }
        std::fs::write(kept.join(FILE), "{\"preferences\": {\"x\": 1}}").unwrap();
        std::fs::write(damaged.join(FILE), "{\"preferences\": {}} }").unwrap();
        let cli = tmp.path().join("pixeldigital-editor-cli.exe");
        let dirs = [fresh.clone(), kept.clone(), damaged.clone()];
        let r = connect(&dirs, Some(&cli), false);
        assert_eq!(r.connected, [fresh.join(FILE).display().to_string(), kept.join(FILE).display().to_string()]);
        assert_eq!(r.broken.len(), 1);
        assert!(r.error.is_empty());
        assert!(!fresh.join("claude_desktop_config.json.bak").exists(), "nothing to back up");
        assert_eq!(std::fs::read_to_string(kept.join("claude_desktop_config.json.bak")).unwrap(), "{\"preferences\": {\"x\": 1}}");
        assert_eq!(std::fs::read_to_string(damaged.join(FILE)).unwrap(), "{\"preferences\": {}} }", "left as it was");
        let kept_now: Value = serde_json::from_str(&std::fs::read_to_string(kept.join(FILE)).unwrap()).unwrap();
        assert_eq!(kept_now["preferences"]["x"], 1);
        assert_eq!(kept_now["mcpServers"][server_name()]["command"], cli.display().to_string());
        // Replace starts the damaged one afresh and keeps a copy
        let r = connect(&dirs, Some(&cli), true);
        assert_eq!(r.connected.len(), 3);
        assert!(r.broken.is_empty());
        assert_eq!(std::fs::read_to_string(damaged.join("claude_desktop_config.json.bak")).unwrap(), "{\"preferences\": {}} }");
        assert!(servers(&std::fs::read_to_string(damaged.join(FILE)).unwrap()).contains_key(server_name()));
        assert!(!damaged.join("claude_desktop_config.json.tmp").exists());
    }

    #[test]
    fn connect_says_what_is_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let r = connect(&[], Some(&tmp.path().join("x-editor-cli.exe")), false);
        assert!(r.error.contains("claude.ai/download"), "{}", r.error);
        let r = connect(&[tmp.path().to_path_buf()], None, false);
        assert!(r.error.contains("-cli"), "{}", r.error);
        assert!(!tmp.path().join(FILE).exists());
    }

    #[test]
    fn the_cli_beside_the_app_is_found_by_the_brand_name_or_the_developer_name() {
        let tmp = tempfile::tempdir().unwrap();
        let exe = tmp.path().join("app.exe");
        assert_eq!(cli_beside(&exe), None);
        let dev = tmp.path().join(format!("filmcraft-cli{}", std::env::consts::EXE_SUFFIX));
        std::fs::write(&dev, b"").unwrap();
        assert_eq!(cli_beside(&exe), Some(dev));
        let branded = tmp.path().join(format!("{}-cli{}", ntm_brand::editor::EXE, std::env::consts::EXE_SUFFIX));
        std::fs::write(&branded, b"").unwrap();
        assert_eq!(cli_beside(&exe), Some(branded));
    }
}
