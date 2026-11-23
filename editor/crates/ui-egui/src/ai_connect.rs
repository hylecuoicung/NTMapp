//! Help ▸ Connect AI Apps…: registers this app's MCP server (`<exe>-cli mcp`, the headless mode of
//! `docs/agents.md`) in the settings of the AI apps found on this computer, so that people who are
//! not developers never edit those files by hand. Fork-only (the NTM Digital / Pixel Digital
//! builds).
//!
//! - **Claude Desktop**: `claude_desktop_config.json`, under `mcpServers` (JSON). Where it keeps
//!   that file depends on how it was installed: `%APPDATA%\Claude` for the classic installer,
//!   `%LOCALAPPDATA%\Packages\Claude_*\LocalCache\Roaming\Claude` for the packaged (MSIX) one,
//!   `%LOCALAPPDATA%\Claude-Data\claude_*` for builds that keep their data there, and
//!   `~/Library/Application Support/Claude` on macOS. Every one of those folders that exists gets
//!   the entry; a folder this Claude does not read only holds a file nobody uses.
//! - **ChatGPT desktop**, and the Codex CLI and IDE extension, which share its settings:
//!   `config.toml` in `%CODEX_HOME%`, else `~/.codex`, under `[mcp_servers.<name>]` (TOML). Codex
//!   gives a tool 60 seconds unless told otherwise, too short for transcribing or exporting, so a
//!   missing `tool_timeout_sec` is set to half an hour (and `startup_timeout_sec` to a minute, for
//!   a first start under a virus scan).
//! - **Cursor**: `~/.cursor/mcp.json`, under `mcpServers` (JSON).
//!
//! An app is connected only when its settings folder exists: it is installed and was opened once.
//! Connecting keeps everything else in each file (the app's own preferences, other MCP servers,
//! the comments and layout of a TOML file) and copies the file to `<file>.bak` before changing it.
//! It also drops the entries an uninstalled copy of the editor left behind (their program is gone,
//! so the app would only report a failed server) and other entries that run this same program
//! under another name. A file that cannot be read is left alone until the person asks to replace
//! it. For any other AI app, the window shows the entry in the `mcpServers` form most of them
//! take, with a button that copies it.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};
use toml_edit::{Item, Table};

use crate::FilmcraftApp;

/// The Help menu command.
pub const COMMAND: &str = "help.connectAiApps";
/// Seconds Codex lets one tool call run (its own default is 60).
pub const TOOL_TIMEOUT_SECS: i64 = 1800;
/// Seconds Codex waits for the server to start (its own default is 10).
pub const STARTUP_TIMEOUT_SECS: i64 = 60;

/// An AI app whose settings can hold this app's MCP server.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AiApp {
    Claude,
    /// ChatGPT desktop and the Codex CLI / IDE extension (one settings file).
    Codex,
    Cursor,
}

/// The apps this window knows, in the order it lists them.
pub const APPS: [AiApp; 3] = [AiApp::Claude, AiApp::Codex, AiApp::Cursor];

impl AiApp {
    /// The product's name (not translated).
    pub fn name(self) -> &'static str {
        match self {
            AiApp::Claude => "Claude Desktop",
            AiApp::Codex => "ChatGPT desktop / Codex",
            AiApp::Cursor => "Cursor",
        }
    }

    /// The settings file in each of its folders.
    pub fn file(self) -> &'static str {
        match self {
            AiApp::Claude => "claude_desktop_config.json",
            AiApp::Codex => "config.toml",
            AiApp::Cursor => "mcp.json",
        }
    }
}

/// A settings folder of one AI app.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub app: AiApp,
    pub dir: String,
}

/// A settings file that now holds this app's entry.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Connected {
    pub app: AiApp,
    pub path: String,
}

/// What the last Connect did, shown until closed (`ui.aiConnect`).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    /// The name the AI apps list this app's tools under.
    pub server: String,
    /// The command-line tool they run; empty when it is missing.
    pub cli: String,
    /// The settings folders that were looked at (Replace uses them again).
    pub targets: Vec<Target>,
    /// Settings files that now hold the entry.
    pub connected: Vec<Connected>,
    /// Entries taken out: an uninstalled copy's, or this program's under another name.
    pub removed: Vec<String>,
    /// Settings files left as they were because they could not be read, with the reason.
    pub broken: Vec<(String, String)>,
    /// Why nothing could be connected; empty when something was.
    pub error: String,
}

impl Report {
    /// The apps that were connected, each once, in [`APPS`] order.
    pub fn connected_apps(&self) -> Vec<AiApp> {
        APPS.into_iter().filter(|a| self.connected.iter().any(|c| c.app == *a)).collect()
    }

    /// The apps with no settings folder on this computer.
    pub fn missing_apps(&self) -> Vec<AiApp> {
        APPS.into_iter().filter(|a| !self.targets.iter().any(|t| t.app == *a)).collect()
    }
}

/// The name the AI apps list the tools under: the brand's program name, such as
/// `pixeldigital-editor`.
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

/// The settings folders of every known AI app that exist on this computer, read from the
/// environment variables `env` returns (see the module docs for the places).
pub fn find_targets(env: &dyn Fn(&str) -> Option<String>) -> Vec<Target> {
    let var = |k: &str| env(k).filter(|v| !v.trim().is_empty()).map(PathBuf::from);
    let target = |app, dir: PathBuf| Target { app, dir: dir.display().to_string() };
    let mut out: Vec<Target> = claude_dirs(env).into_iter().map(|d| target(AiApp::Claude, d)).collect();
    // Windows names the home folder USERPROFILE; HOME is the others' (and Git Bash's)
    let home = var("USERPROFILE").or_else(|| var("HOME"));
    let codex = var("CODEX_HOME").or_else(|| home.as_ref().map(|h| h.join(".codex")));
    if let Some(dir) = codex.filter(|d| d.is_dir()) {
        out.push(target(AiApp::Codex, dir));
    }
    if let Some(dir) = home.map(|h| h.join(".cursor")).filter(|d| d.is_dir()) {
        out.push(target(AiApp::Cursor, dir));
    }
    out
}

/// Claude Desktop's settings folders that exist on this computer.
pub fn claude_dirs(env: &dyn Fn(&str) -> Option<String>) -> Vec<PathBuf> {
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

/// Whether an entry running `command` under `name` is stale next to this app's `server` running
/// `cli`: another copy of the editor that is gone, or this same program under another name.
fn stale(name: &str, command: &str, server: &str, cli: &str, exists: &dyn Fn(&str) -> bool) -> bool {
    name != server && is_editor_cli(command) && (same_program(command, cli) || !exists(command))
}

/// What a `register_*` function returns: the new text (`None` when nothing changed) and the names
/// of the entries taken out ([`Report::removed`]).
pub type Registered = (Option<String>, Vec<String>);

/// The JSON settings `text` (Claude Desktop, Cursor) with `server` registered under `mcpServers` to
/// run `cli mcp`. `Err` when `text` is not a JSON object (with the line and column), so it is never
/// overwritten by mistake. `exists` says whether a program file is there.
pub fn register_json(text: &str, server: &str, cli: &str, exists: &dyn Fn(&str) -> bool) -> Result<Registered, String> {
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
        .filter_map(|(name, entry)| {
            let command = entry.get("command").and_then(Value::as_str)?;
            stale(name, command, server, cli, exists).then(|| name.clone())
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

/// The table under `key` in `parent`, made a table if it is written inline (`{ … }`) and created
/// when missing. `None` when `key` holds something that is not a table.
fn child_table<'a>(parent: &'a mut Table, key: &str, implicit: bool) -> Option<&'a mut Table> {
    if !parent.contains_key(key) {
        let mut t = Table::new();
        // an implicit table prints no `[mcp_servers]` header of its own, only its sub-tables
        t.set_implicit(implicit);
        parent.insert(key, Item::Table(t));
    }
    let item = parent.get_mut(key)?;
    if let Some(inline) = item.as_inline_table() {
        *item = Item::Table(inline.clone().into_table());
    }
    item.as_table_mut()
}

/// The TOML settings `text` (ChatGPT desktop / Codex) with `server` registered under
/// `[mcp_servers.<server>]` to run `cli mcp`, with room for long tools ([`TOOL_TIMEOUT_SECS`]) unless
/// the person set their own. Everything else keeps its text, comments and order. `Err` when `text`
/// is not valid TOML (with where it breaks).
pub fn register_toml(text: &str, server: &str, cli: &str, exists: &dyn Fn(&str) -> bool) -> Result<Registered, String> {
    let text = text.trim_start_matches('\u{feff}');
    let mut doc: toml_edit::DocumentMut = text.parse().map_err(|e: toml_edit::TomlError| {
        let why = e.to_string();
        tlf!("not valid TOML ({why})", why = why.lines().next().unwrap_or_default().trim())
    })?;
    let root = doc.as_table_mut();
    if root.get("mcp_servers").is_some_and(|i| !i.is_table_like()) {
        return Err(tl!("its \"mcp_servers\" is not a table").to_string());
    }
    let Some(servers) = child_table(root, "mcp_servers", true) else { return Err(tl!("its \"mcp_servers\" is not a table").to_string()) };
    let removed: Vec<String> = servers
        .iter()
        .filter_map(|(name, entry)| {
            let command = entry.as_table_like()?.get("command")?.as_str()?;
            stale(name, command, server, cli, exists).then(|| name.to_string())
        })
        .collect();
    for name in &removed {
        servers.remove(name);
    }
    if servers.get(server).is_some_and(|i| !i.is_table_like()) {
        servers.remove(server);
    }
    let Some(entry) = child_table(servers, server, false) else { return Err(tl!("its \"mcp_servers\" is not a table").to_string()) };
    // only what is missing or different is written: the person's own keys (env, timeouts) stay
    if entry.get("command").and_then(Item::as_str) != Some(cli) {
        entry.insert("command", toml_edit::value(cli));
    }
    let args_ok = entry.get("args").and_then(Item::as_array).is_some_and(|a| a.len() == 1 && a.get(0).and_then(|v| v.as_str()) == Some("mcp"));
    if !args_ok {
        entry.insert("args", toml_edit::value(toml_edit::Array::from_iter(["mcp"])));
    }
    if !entry.contains_key("tool_timeout_sec") {
        entry.insert("tool_timeout_sec", toml_edit::value(TOOL_TIMEOUT_SECS));
    }
    if !entry.contains_key("startup_timeout_sec") {
        entry.insert("startup_timeout_sec", toml_edit::value(STARTUP_TIMEOUT_SECS));
    }
    let mut out = doc.to_string();
    if out == text {
        return Ok((None, removed));
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    Ok((Some(out), removed))
}

/// The entry for any other AI app, in the `mcpServers` form most of them take.
pub fn snippet(server: &str, cli: &str) -> String {
    let v = json!({"mcpServers": {server: {"command": cli, "args": ["mcp"]}}});
    serde_json::to_string_pretty(&v).unwrap_or_default()
}

/// Register this app in the settings file of each of `targets`. A file that cannot be read is
/// left alone and listed in [`Report::broken`], unless `replace_broken`: then it is copied to
/// `.bak` and started afresh with this app's entry.
pub fn connect(targets: &[Target], cli: Option<&Path>, replace_broken: bool) -> Report {
    let mut report = Report {
        server: server_name().to_string(),
        cli: cli.map(|c| c.display().to_string()).unwrap_or_default(),
        targets: targets.to_vec(),
        ..Report::default()
    };
    if cli.is_none() {
        report.error = tlf!(
            "The command-line tool ({name}) is missing from this app's folder. Install NTM Digital Editor again, then try once more.",
            name = format!("{}-cli{}", ntm_brand::editor::EXE, std::env::consts::EXE_SUFFIX)
        );
        return report;
    }
    if targets.is_empty() {
        report.error = tl!(
            "No AI app that this window can set up was found on this computer. Install Claude Desktop (claude.ai/download), ChatGPT desktop or Cursor, open it once, then try again. For another AI app, copy the settings below into it."
        )
        .to_string();
        return report;
    }
    let mut failures: Vec<String> = Vec::new();
    for t in targets {
        let path = Path::new(&t.dir).join(t.app.file());
        match connect_file(&path, t.app, &report.cli, replace_broken) {
            Ok(removed) => {
                report.connected.push(Connected { app: t.app, path: path.display().to_string() });
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
        report.error = tlf!("The AI apps' settings could not be changed: {why}", why = failures.join("; "));
    }
    report
}

enum FileError {
    /// Not readable as JSON / TOML: left as it was.
    Broken(String),
    /// Reading or writing failed.
    Io(String),
}

/// Register this app in one settings file; returns the entries taken out.
fn connect_file(path: &Path, app: AiApp, cli: &str, replace_broken: bool) -> Result<Vec<String>, FileError> {
    let old = match std::fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(FileError::Io(e.to_string())),
    };
    let exists = |p: &str| Path::new(p).is_file();
    let server = server_name();
    let register = match app {
        AiApp::Codex => register_toml,
        AiApp::Claude | AiApp::Cursor => register_json,
    };
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
    // write beside it, then swap it in: a failed write never leaves the app a half-written file
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

/// Help ▸ Connect AI Apps…. Params (all optional, for automation and tests): `targets` (the
/// `[{"app": "claude"|"codex"|"cursor", "dir": …}]` to use instead of finding the apps'),
/// `configDirs` (Claude Desktop folders only, the older form), `cli` (the program to register
/// instead of the one beside this app), `replace` (start unreadable settings files afresh, keeping
/// a copy).
pub fn run(app: &mut FilmcraftApp, params: &Value) -> Result<Value, String> {
    let targets: Vec<Target> = if let Some(list) = params.get("targets") {
        serde_json::from_value(list.clone()).map_err(|e| format!("targets: {e}"))?
    } else if let Some(list) = params.get("configDirs").and_then(Value::as_array) {
        list.iter().filter_map(Value::as_str).map(|d| Target { app: AiApp::Claude, dir: d.to_string() }).collect()
    } else {
        find_targets(&|k| std::env::var(k).ok())
    };
    let cli = match params.get("cli").and_then(Value::as_str) {
        Some(c) => Some(PathBuf::from(c)),
        None => std::env::current_exe().ok().and_then(|exe| cli_beside(&exe)),
    };
    let replace = params.get("replace").and_then(Value::as_bool).unwrap_or(false);
    let report = connect(&targets, cli.as_deref(), replace);
    let apps = report.connected_apps();
    app.ui.status = if apps.is_empty() {
        tl!("No AI app was connected").to_string()
    } else {
        tlf!("Connected {apps}: quit and reopen them", apps = apps.iter().map(|a| a.name()).collect::<Vec<_>>().join(", "))
    };
    app.ui.ai_connect = Some(report.clone());
    serde_json::to_value(&report).map_err(|e| e.to_string())
}

/// The result window, until closed. Automation ids: `aiConnect.close`, `aiConnect.replace`,
/// `aiConnect.copy`.
pub fn show(app: &mut FilmcraftApp, ctx: &egui::Context) {
    let Some(report) = app.ui.ai_connect.clone() else { return };
    let mut close = false;
    let mut replace = false;
    let mut copy = false;
    let mut elems: Vec<(&str, egui::Rect, String)> = Vec::new();
    egui::Window::new(tl!("Connect AI Apps"))
        .id(egui::Id::unique("ai-connect"))
        .collapsible(false)
        .resizable(false)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(crate::widgets::dialog_center(ctx))
        .show(ctx, |ui| {
            ui.set_max_width(560.0);
            let apps = report.connected_apps();
            if !apps.is_empty() {
                ui.label(egui::RichText::new(tl!("These AI apps can now use NTM Digital Editor:")).strong());
                for a in &apps {
                    ui.label(format!("  • {}", a.name()));
                }
                ui.add_space(4.0);
                ui.label(tl!(
                    "1. Quit each of them completely and open it again. Claude Desktop and ChatGPT keep running next to the clock: right-click the icon there and choose Quit."
                ));
                ui.label(tlf!("2. Ask, for example: “Use {server} to list the File commands.”", server = report.server));
                ui.add_space(4.0);
                ui.label(tl!(
                    "They edit project files, not this window: save and close a project here before asking an AI app to change it, then open it again to see the result."
                ));
            }
            if !report.error.is_empty() {
                ui.label(&report.error);
            }
            let missing = report.missing_apps();
            if !missing.is_empty() && report.error.is_empty() {
                ui.label(
                    egui::RichText::new(tlf!("Not found on this computer: {apps}", apps = missing.iter().map(|a| a.name()).collect::<Vec<_>>().join(", ")))
                        .small()
                        .weak(),
                );
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
                ui.label(tl!("These settings files are damaged, so they were left as they are:"));
                for (path, why) in &report.broken {
                    ui.label(egui::RichText::new(format!("{path} ({why})")).small());
                }
                ui.label(tl!("Replace starts them afresh with this app's entry and keeps a copy of each as .bak."));
            }
            if !report.cli.is_empty() {
                ui.add_space(6.0);
                ui.separator();
                ui.label(tl!("Another AI app: add an MCP server that runs this program with the argument mcp, or paste these settings into it:"));
                ui.label(egui::RichText::new(snippet(&report.server, &report.cli)).monospace().small());
                let r = ui.button(tl!("Copy settings"));
                elems.push(("aiConnect.copy", r.rect, "Copy settings".into()));
                copy |= r.clicked();
            }
            ui.add_space(4.0);
            for c in &report.connected {
                ui.label(egui::RichText::new(tlf!("{app} settings: {path}", app = c.app.name(), path = c.path)).small().weak());
            }
            if !report.cli.is_empty() {
                ui.label(egui::RichText::new(tlf!("Program: {cli}", cli = report.cli)).small().weak());
            }
            ui.add_space(6.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let r = ui.button(tl!("Close"));
                elems.push(("aiConnect.close", r.rect, "Close".into()));
                close |= r.clicked();
                if !report.broken.is_empty() && !report.cli.is_empty() {
                    let r = ui.button(tl!("Replace"));
                    elems.push(("aiConnect.replace", r.rect, "Replace".into()));
                    replace |= r.clicked();
                }
            });
        });
    for (id, r, label) in elems {
        app.auto.add(id, r, &label);
    }
    if copy {
        ctx.copy_text(snippet(&report.server, &report.cli));
        app.ui.status = tl!("Copied the MCP settings").to_string();
    }
    if replace {
        let params = json!({"targets": report.targets, "cli": report.cli, "replace": true});
        let _ = run(app, &params);
    } else if close || crate::widgets::escape_closes(ctx) {
        app.ui.ai_connect = None;
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

    fn toml_servers(text: &str) -> toml_edit::Table {
        let doc: toml_edit::DocumentMut = text.parse().unwrap();
        doc["mcp_servers"].as_table().unwrap().clone()
    }

    #[test]
    fn an_empty_or_missing_file_gets_just_this_server() {
        for text in ["", "  \n", "{}"] {
            let (out, removed) = register_json(text, "pixeldigital-editor", CLI, &nothing_exists).unwrap();
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
        let (out, _) = register_json(text, "pixeldigital-editor", CLI, &nothing_exists).unwrap();
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
        // NTM's copy was uninstalled: its entry would only fail in the AI app
        let (out, removed) = register_json(&text, "pixeldigital-editor", CLI, &nothing_exists).unwrap();
        assert_eq!(removed, ["ntm-editor"]);
        let s = servers(&out.unwrap());
        assert!(!s.contains_key("ntm-editor"));
        assert!(s.contains_key("unrelated-missing"), "only editor tools are cleaned up");
        // both brands installed side by side: each keeps its own entry
        let (out, removed) = register_json(&text, "pixeldigital-editor", CLI, &|p| p == ntm).unwrap();
        assert!(removed.is_empty());
        assert!(servers(&out.unwrap()).contains_key("ntm-editor"));
    }

    #[test]
    fn the_same_program_under_another_name_is_not_listed_twice() {
        let other_case = CLI.to_uppercase();
        let text = json!({"mcpServers": {"my-editor": {"command": if cfg!(windows) { other_case.as_str() } else { CLI }, "args": ["mcp"]}}}).to_string();
        let (out, removed) = register_json(&text, "pixeldigital-editor", CLI, &|_| true).unwrap();
        assert_eq!(removed, ["my-editor"]);
        assert_eq!(servers(&out.unwrap()).keys().collect::<Vec<_>>(), ["pixeldigital-editor"]);
    }

    #[test]
    fn connecting_again_changes_nothing() {
        let (first, _) = register_json("{}", "pixeldigital-editor", CLI, &nothing_exists).unwrap();
        let first = first.unwrap();
        assert_eq!(register_json(&first, "pixeldigital-editor", CLI, &nothing_exists).unwrap(), (None, vec![]));
        // a moved install points the entry at the new place
        let moved = r"C:\Program Files\PixelDigital\PixelDigitalEditor\pixeldigital-editor-cli.exe";
        let (out, _) = register_json(&first, "pixeldigital-editor", moved, &nothing_exists).unwrap();
        assert_eq!(servers(&out.unwrap())["pixeldigital-editor"]["command"], moved);
    }

    #[test]
    fn a_byte_order_mark_is_read_and_not_written_back() {
        let (out, _) = register_json("\u{feff}{\"a\": 1}", "pixeldigital-editor", CLI, &nothing_exists).unwrap();
        let out = out.unwrap();
        assert!(!out.starts_with('\u{feff}'));
        assert_eq!(serde_json::from_str::<Value>(&out).unwrap()["a"], 1);
        let (out, _) = register_toml("\u{feff}model = \"x\"\n", "pixeldigital-editor", CLI, &nothing_exists).unwrap();
        assert!(out.unwrap().starts_with("model = \"x\"\n"));
    }

    #[test]
    fn damaged_settings_are_refused_with_where_they_break() {
        // the mistake people make by hand: a block pasted after the closing brace
        let text = "{\n  \"preferences\": {}\n}\n{ \"mcpServers\": {} }\n";
        let err = register_json(text, "pixeldigital-editor", CLI, &nothing_exists).unwrap_err();
        assert!(err.contains("line 4"), "{err}");
        for bad in ["[1, 2]", "\"text\"", "{\"mcpServers\": [1]}", "{\"a\": 1,}"] {
            assert!(register_json(bad, "pixeldigital-editor", CLI, &nothing_exists).is_err(), "{bad}");
        }
        let err = register_toml("[desktop]\nsansFontSize = 14\ncommand = C:\\x\n", "pixeldigital-editor", CLI, &nothing_exists).unwrap_err();
        assert!(err.contains("line 3"), "{err}");
        for bad in ["mcp_servers = 3\n", "mcp_servers = [1]\n", "[a]\nb = 1\n[a]\nb = 2\n"] {
            assert!(register_toml(bad, "pixeldigital-editor", CLI, &nothing_exists).is_err(), "{bad}");
        }
    }

    /// A ChatGPT desktop config.toml like a real one: its own servers, sub-tables, literal strings.
    const CODEX_CONFIG: &str = r#"notify = [ "C:\\Program Files\\WindowsApps\\OpenAI.Codex\\codex-computer-use.exe", "turn-ended" ]

[plugins."browser@openai-bundled"]
enabled = true

[mcp_servers.node_repl]
args = []
command = 'C:\Program Files\WindowsApps\OpenAI.Codex\node_repl.exe'
startup_timeout_sec = 120

[mcp_servers.node_repl.env]
CODEX_HOME = 'C:\Users\u\.codex'

[windows]
sandbox = "elevated"

# set by hand
[mcp_servers.pixeldigital-editor]
command = 'C:\Users\u\AppData\Local\Programs\PixelDigital\PixelDigitalEditor\pixeldigital-editor-cli.exe'
args = ["mcp"]
"#;

    #[test]
    fn chatgpts_config_keeps_every_line_and_only_gains_the_timeouts() {
        let (out, removed) = register_toml(CODEX_CONFIG, "pixeldigital-editor", CLI, &nothing_exists).unwrap();
        let out = out.unwrap();
        assert!(removed.is_empty());
        // the person's file, untouched up to the end of the entry they wrote, then the two timeouts
        assert_eq!(out, format!("{CODEX_CONFIG}tool_timeout_sec = {TOOL_TIMEOUT_SECS}\nstartup_timeout_sec = {STARTUP_TIMEOUT_SECS}\n"));
        let s = toml_servers(&out);
        assert_eq!(s["node_repl"]["startup_timeout_sec"].as_integer(), Some(120));
        assert_eq!(s["node_repl"]["env"]["CODEX_HOME"].as_str(), Some(r"C:\Users\u\.codex"));
        // connecting again changes nothing
        assert_eq!(register_toml(&out, "pixeldigital-editor", CLI, &nothing_exists).unwrap(), (None, vec![]));
    }

    #[test]
    fn chatgpts_config_gets_a_new_entry_and_loses_an_uninstalled_copys_with_its_sub_tables() {
        let ntm = r"C:\Program Files\NTMDigital\NTMDigitalEditor\ntmdigital-editor-cli.exe";
        let text = format!(
            "model = \"gpt-5\"\n\n[mcp_servers.ntm]\ncommand = '{ntm}'\nargs = [\"mcp\"]\n\n[mcp_servers.ntm.env]\nA = \"1\"\n\n[mcp_servers.other]\ncommand = \"node\"\n"
        );
        let (out, removed) = register_toml(&text, "pixeldigital-editor", CLI, &nothing_exists).unwrap();
        let out = out.unwrap();
        assert_eq!(removed, ["ntm"]);
        assert!(!out.contains("[mcp_servers.ntm"), "{out}");
        assert!(out.starts_with("model = \"gpt-5\"\n"));
        let s = toml_servers(&out);
        assert_eq!(s["other"]["command"].as_str(), Some("node"));
        let e = &s["pixeldigital-editor"];
        assert_eq!(e["command"].as_str(), Some(CLI));
        assert_eq!(e["args"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect::<Vec<_>>(), ["mcp"]);
        assert_eq!(e["tool_timeout_sec"].as_integer(), Some(TOOL_TIMEOUT_SECS));
        // an empty file gets just the entry, under its own header
        let (out, _) = register_toml("", "pixeldigital-editor", CLI, &nothing_exists).unwrap();
        let out = out.unwrap();
        assert!(out.starts_with("[mcp_servers.pixeldigital-editor]\n"), "{out}");
        assert_eq!(toml_servers(&out).len(), 1);
    }

    #[test]
    fn chatgpts_config_keeps_the_persons_own_timeout_and_moves_the_program() {
        let text = "[mcp_servers.pixeldigital-editor]\ncommand = 'C:\\old\\pixeldigital-editor-cli.exe'\nargs = [\"mcp\", \"--x\"]\ntool_timeout_sec = 90\n";
        let (out, _) = register_toml(text, "pixeldigital-editor", CLI, &nothing_exists).unwrap();
        let e = toml_servers(&out.unwrap())["pixeldigital-editor"].clone();
        assert_eq!(e["command"].as_str(), Some(CLI));
        assert_eq!(e["args"].as_array().unwrap().len(), 1);
        assert_eq!(e["tool_timeout_sec"].as_integer(), Some(90), "the person's choice stays");
        assert_eq!(e["startup_timeout_sec"].as_integer(), Some(STARTUP_TIMEOUT_SECS));
        // servers written inline are made a table and kept
        let (out, _) = register_toml("mcp_servers = { other = { command = \"node\" } }\n", "pixeldigital-editor", CLI, &nothing_exists).unwrap();
        let s = toml_servers(&out.unwrap());
        assert_eq!(s["other"]["command"].as_str(), Some("node"));
        assert_eq!(s["pixeldigital-editor"]["command"].as_str(), Some(CLI));
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
        assert_eq!(claude_dirs(&env), [classic.clone(), msix, data]);
        // a computer without Claude Desktop has none
        std::fs::remove_dir_all(&roaming).unwrap();
        std::fs::remove_dir_all(&local).unwrap();
        assert!(claude_dirs(&env).is_empty());
        assert!(claude_dirs(&|_| None).is_empty());
    }

    #[test]
    fn chatgpt_and_cursor_are_found_in_the_home_folder_or_codex_home() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("u");
        let roaming = tmp.path().join("Roaming");
        std::fs::create_dir_all(roaming.join("Claude")).unwrap();
        std::fs::create_dir_all(home.join(".codex")).unwrap();
        std::fs::create_dir_all(home.join(".cursor")).unwrap();
        let t = |app, dir: PathBuf| Target { app, dir: dir.display().to_string() };
        let env = |k: &str| match k {
            "APPDATA" => Some(roaming.display().to_string()),
            "USERPROFILE" => Some(home.display().to_string()),
            _ => None,
        };
        assert_eq!(
            find_targets(&env),
            [t(AiApp::Claude, roaming.join("Claude")), t(AiApp::Codex, home.join(".codex")), t(AiApp::Cursor, home.join(".cursor"))]
        );
        // CODEX_HOME moves Codex's folder; HOME stands in for USERPROFILE outside Windows
        let elsewhere = tmp.path().join("codex-home");
        std::fs::create_dir_all(&elsewhere).unwrap();
        let env = |k: &str| match k {
            "HOME" => Some(home.display().to_string()),
            "CODEX_HOME" => Some(elsewhere.display().to_string()),
            _ => None,
        };
        assert_eq!(find_targets(&env), [t(AiApp::Codex, elsewhere.clone()), t(AiApp::Cursor, home.join(".cursor"))]);
        // apps that were never opened have no folder and are not set up
        std::fs::remove_dir_all(&home).unwrap();
        std::fs::remove_dir_all(&elsewhere).unwrap();
        assert!(find_targets(&env).is_empty());
    }

    #[test]
    fn connect_writes_each_app_keeps_a_copy_and_leaves_damaged_files_until_replaced() {
        let tmp = tempfile::tempdir().unwrap();
        let (fresh, kept, damaged, codex, cursor) =
            (tmp.path().join("a"), tmp.path().join("b"), tmp.path().join("c"), tmp.path().join(".codex"), tmp.path().join(".cursor"));
        for d in [&fresh, &kept, &damaged, &codex, &cursor] {
            std::fs::create_dir_all(d).unwrap();
        }
        const CLAUDE: &str = "claude_desktop_config.json";
        std::fs::write(kept.join(CLAUDE), "{\"preferences\": {\"x\": 1}}").unwrap();
        std::fs::write(damaged.join(CLAUDE), "{\"preferences\": {}} }").unwrap();
        std::fs::write(codex.join("config.toml"), "# mine\nmodel = \"gpt-5\"\n").unwrap();
        let cli = tmp.path().join("pixeldigital-editor-cli.exe");
        let t = |app, dir: &PathBuf| Target { app, dir: dir.display().to_string() };
        let targets = [t(AiApp::Claude, &fresh), t(AiApp::Claude, &kept), t(AiApp::Claude, &damaged), t(AiApp::Codex, &codex), t(AiApp::Cursor, &cursor)];
        let r = connect(&targets, Some(&cli), false);
        let path = |dir: &PathBuf, f: &str| dir.join(f).display().to_string();
        assert_eq!(
            r.connected,
            [
                Connected { app: AiApp::Claude, path: path(&fresh, CLAUDE) },
                Connected { app: AiApp::Claude, path: path(&kept, CLAUDE) },
                Connected { app: AiApp::Codex, path: path(&codex, "config.toml") },
                Connected { app: AiApp::Cursor, path: path(&cursor, "mcp.json") },
            ]
        );
        assert_eq!(r.connected_apps(), APPS);
        assert!(r.missing_apps().is_empty());
        assert_eq!(r.broken.len(), 1);
        assert!(r.error.is_empty());
        assert!(!fresh.join("claude_desktop_config.json.bak").exists(), "nothing to back up");
        assert_eq!(std::fs::read_to_string(kept.join("claude_desktop_config.json.bak")).unwrap(), "{\"preferences\": {\"x\": 1}}");
        assert_eq!(std::fs::read_to_string(codex.join("config.toml.bak")).unwrap(), "# mine\nmodel = \"gpt-5\"\n");
        assert_eq!(std::fs::read_to_string(damaged.join(CLAUDE)).unwrap(), "{\"preferences\": {}} }", "left as it was");
        let kept_now: Value = serde_json::from_str(&std::fs::read_to_string(kept.join(CLAUDE)).unwrap()).unwrap();
        assert_eq!(kept_now["preferences"]["x"], 1);
        assert_eq!(kept_now["mcpServers"][server_name()]["command"], cli.display().to_string());
        let codex_now = std::fs::read_to_string(codex.join("config.toml")).unwrap();
        assert!(codex_now.starts_with("# mine\nmodel = \"gpt-5\"\n"), "{codex_now}");
        assert_eq!(toml_servers(&codex_now)[server_name()]["command"].as_str(), Some(cli.display().to_string().as_str()));
        assert!(servers(&std::fs::read_to_string(cursor.join("mcp.json")).unwrap()).contains_key(server_name()));
        // Replace starts the damaged one afresh and keeps a copy
        let r = connect(&targets, Some(&cli), true);
        assert_eq!(r.connected.len(), 5);
        assert!(r.broken.is_empty());
        assert_eq!(std::fs::read_to_string(damaged.join("claude_desktop_config.json.bak")).unwrap(), "{\"preferences\": {}} }");
        assert!(servers(&std::fs::read_to_string(damaged.join(CLAUDE)).unwrap()).contains_key(server_name()));
        assert!(!damaged.join("claude_desktop_config.json.tmp").exists());
    }

    #[test]
    fn connect_says_what_is_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let r = connect(&[], Some(&tmp.path().join("x-editor-cli.exe")), false);
        assert!(r.error.contains("claude.ai/download"), "{}", r.error);
        assert_eq!(r.missing_apps(), APPS);
        let target = Target { app: AiApp::Cursor, dir: tmp.path().display().to_string() };
        let r = connect(&[target], None, false);
        assert!(r.error.contains("-cli"), "{}", r.error);
        assert!(!tmp.path().join("mcp.json").exists());
    }

    #[test]
    fn the_snippet_for_other_apps_is_the_usual_mcp_servers_form() {
        let v: Value = serde_json::from_str(&snippet("pixeldigital-editor", CLI)).unwrap();
        assert_eq!(v, json!({"mcpServers": {"pixeldigital-editor": {"command": CLI, "args": ["mcp"]}}}));
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
