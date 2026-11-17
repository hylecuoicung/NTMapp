//! The brand files (`ntm.env`, `pixel.env`): `KEY=value` lines, `#` comment lines, blank lines.
//! One parser and one set of rules for `build.rs` (which refuses a malformed brand) and the tests.
#![allow(dead_code)]

use std::collections::BTreeMap;

pub type Brand = BTreeMap<String, String>;

/// The apps a brand names: (key prefix in the brand files, app name used in tags and file names, title).
pub const APPS: &[(&str, &str, &str)] = &[("EDITOR", "editor", "Editor"), ("PHOTO", "photo", "Photo")];

/// Every key a brand file has. The link keys may be empty; all the others may not.
pub fn keys() -> Vec<String> {
    let mut v: Vec<String> = ["ID", "COMPANY", "SLUG", "VENDOR_DIR", "REPO", "TAG_INFIX"].iter().map(|s| s.to_string()).collect();
    for link in ["DISCORD", "WEBSITE", "FACEBOOK"] {
        v.push(format!("{link}_URL"));
        v.push(format!("{link}_LABEL"));
    }
    for (p, _, _) in APPS {
        for k in ["PRODUCT", "EXE", "FOLDER", "UPGRADE_CODE", "COMPONENT_GUID"] {
            v.push(format!("{p}_{k}"));
        }
    }
    v
}

pub fn parse(text: &str) -> Result<Brand, String> {
    let mut out = Brand::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { return Err(format!("line {}: expected KEY=value", n + 1)) };
        let k = k.trim();
        if out.insert(k.to_string(), v.trim().to_string()).is_some() {
            return Err(format!("line {}: {k} is given twice", n + 1));
        }
    }
    Ok(out)
}

/// `a=b` for every key of `brand`, in a form PowerShell and the build script both read the same way.
pub fn get<'a>(brand: &'a Brand, key: &str) -> &'a str {
    brand.get(key).map_or("", String::as_str)
}

/// The start of the release tags of `app` (`editor-`, `editor-pixel-`): `<prefix>v<version>`.
pub fn tag_prefix(brand: &Brand, app: &str) -> String {
    format!("{app}-{}", get(brand, "TAG_INFIX"))
}

fn is_guid(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() == 5 && parts.iter().zip([8, 4, 4, 4, 12]).all(|(p, n)| p.len() == n && p.chars().all(|c| c.is_ascii_digit() || ('A'..='F').contains(&c)))
}

/// The rules a brand file follows. Names are derived from the company on purpose: the sources spell
/// the default brand's names and `rebrand` swaps them for these, which only works if they line up.
pub fn check(brand: &Brand, id: &str) -> Result<(), String> {
    let known = keys();
    for k in &known {
        if !brand.contains_key(k) {
            return Err(format!("{k} is missing"));
        }
    }
    if let Some(k) = brand.keys().find(|k| !known.contains(k)) {
        return Err(format!("{k} is not a brand key (a typo?)"));
    }
    let need = |k: &str| -> Result<&str, String> {
        let v = get(brand, k);
        if v.is_empty() { Err(format!("{k} is empty")) } else { Ok(v) }
    };
    if need("ID")? != id {
        return Err(format!("ID is {:?} but the file is {id}.env", get(brand, "ID")));
    }
    if !id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) {
        return Err(format!("the brand id {id:?} must be lower-case letters and digits"));
    }
    let company = need("COMPANY")?;
    let vendor = company.replace(' ', "");
    if need("VENDOR_DIR")? != vendor {
        return Err(format!("VENDOR_DIR must be the company without spaces: {vendor}"));
    }
    if need("SLUG")? != vendor.to_lowercase() {
        return Err(format!("SLUG must be the company in lower case without spaces: {}", vendor.to_lowercase()));
    }
    let repo = need("REPO")?;
    if repo.split('/').count() != 2 || repo.contains(' ') {
        return Err(format!("REPO is owner/name, not {repo:?}"));
    }
    let infix = get(brand, "TAG_INFIX");
    if !infix.is_empty() && !(infix.ends_with('-') && infix[..infix.len() - 1].chars().all(|c| c.is_ascii_lowercase())) {
        return Err(format!("TAG_INFIX is empty or lower-case letters and a dash (pixel-), not {infix:?}"));
    }
    for link in ["DISCORD", "WEBSITE", "FACEBOOK"] {
        let (url, label) = (get(brand, &format!("{link}_URL")), get(brand, &format!("{link}_LABEL")));
        if !url.is_empty() && !url.starts_with("https://") {
            return Err(format!("{link}_URL must be empty or start with https://"));
        }
        if url.is_empty() != label.is_empty() {
            return Err(format!("{link}_URL and {link}_LABEL are both given or both empty"));
        }
    }
    for (p, name, title) in APPS {
        let want = [("PRODUCT", format!("{company} {title}")), ("EXE", format!("{}-{name}", vendor.to_lowercase())), ("FOLDER", format!("{vendor}{title}"))];
        for (k, v) in want {
            if need(&format!("{p}_{k}"))? != v {
                return Err(format!("{p}_{k} must be {v}"));
            }
        }
        for k in ["UPGRADE_CODE", "COMPONENT_GUID"] {
            if !is_guid(need(&format!("{p}_{k}"))?) {
                return Err(format!("{p}_{k} is not an upper-case GUID like BB5F6996-DF46-4CB3-9742-12A7CE95C4F6"));
            }
        }
    }
    Ok(())
}
