//! The brand a build carries: the company and product names, the install identity and the
//! community links. One source tree builds under more than one brand (`ntm.env`, `pixel.env`):
//! `NTM_BRAND=pixel cargo build` picks one, the default is `ntm`. The logo is the same for all.
//!
//! The sources spell the default brand's names ("NTM Digital Editor") in their English UI text,
//! so that text stays as upstream-merge friendly as before; [`rebrand`] swaps those names for the
//! chosen brand's where the text is shown. Everything else (window titles, update checks, the
//! exe's version resource, the installer) reads the constants below.

use std::borrow::Cow;

#[cfg(test)]
mod parse;

macro_rules! text {
    ($key:literal) => {
        env!(concat!("NTM_BRAND_", $key))
    };
}

/// The chosen brand's id: `ntm` or `pixel`.
pub const ID: &str = text!("ID");
/// The company, as shown to people: `NTM Digital`.
pub const COMPANY: &str = text!("COMPANY");
/// The company in lower case, the start of every shipped file name: `ntmdigital`.
pub const SLUG: &str = text!("SLUG");
/// The company without spaces, the folder the apps install under and the start of registry names.
pub const VENDOR_DIR: &str = text!("VENDOR_DIR");
/// The GitHub repository the releases are published in (`owner/name`).
pub const REPO: &str = text!("REPO");

/// Is this build the default brand, the one the sources spell? Then [`rebrand`] has nothing to do.
pub const IS_DEFAULT: bool = same(ID, "ntm");

/// A community link: the address and the label of its menu item or button. The address is empty
/// when the brand has no such link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Link {
    pub url: &'static str,
    pub label: &'static str,
}

pub const DISCORD: Link = Link { url: text!("DISCORD_URL"), label: text!("DISCORD_LABEL") };
pub const WEBSITE: Link = Link { url: text!("WEBSITE_URL"), label: text!("WEBSITE_LABEL") };
pub const FACEBOOK: Link = Link { url: text!("FACEBOOK_URL"), label: text!("FACEBOOK_LABEL") };

impl Link {
    pub const fn is_set(&self) -> bool {
        !self.url.is_empty()
    }
}

/// The video editor.
pub mod editor {
    /// `NTM Digital Editor`.
    pub const PRODUCT: &str = text!("EDITOR_PRODUCT");
    /// The stem of the shipped file names and of the update id: `ntmdigital-editor`.
    pub const EXE: &str = text!("EDITOR_EXE");
    /// The install folder and registry key: `NTMDigitalEditor`.
    pub const FOLDER: &str = text!("EDITOR_FOLDER");
    /// The start of this brand's release tags: `editor-` (`editor-pixel-` for Pixel Digital); a tag is `<prefix>v<version>`.
    pub const TAG_PREFIX: &str = text!("EDITOR_TAG_PREFIX");
}

/// The image editor.
pub mod photo {
    /// `NTM Digital Photo`.
    pub const PRODUCT: &str = text!("PHOTO_PRODUCT");
    /// The stem of the shipped file names and of the update id: `ntmdigital-photo`.
    pub const EXE: &str = text!("PHOTO_EXE");
    /// The install folder, registry key and ProgId prefix: `NTMDigitalPhoto`.
    pub const FOLDER: &str = text!("PHOTO_FOLDER");
    /// The start of this brand's release tags: `photo-` (`photo-pixel-` for Pixel Digital); a tag is `<prefix>v<version>`.
    pub const TAG_PREFIX: &str = text!("PHOTO_TAG_PREFIX");
}

/// The names the sources spell (the default brand's).
pub mod source {
    pub const COMPANY: &str = text!("SRC_COMPANY");
    pub const EDITOR_PRODUCT: &str = text!("SRC_EDITOR_PRODUCT");
    pub const PHOTO_PRODUCT: &str = text!("SRC_PHOTO_PRODUCT");
}

const fn same(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// The same text with the names the sources spell swapped for the chosen brand's: "NTM Digital
/// Editor" becomes "Pixel Digital Editor", a bare "NTM Digital" becomes "Pixel Digital". The
/// default brand's text comes back as it is.
pub fn rebrand(text: &str) -> Cow<'_, str> {
    if IS_DEFAULT {
        return Cow::Borrowed(text);
    }
    rebrand_with(text, &[(source::EDITOR_PRODUCT, editor::PRODUCT), (source::PHOTO_PRODUCT, photo::PRODUCT), (source::COMPANY, COMPANY)])
}

/// [`rebrand`] with the names given: each `(from, to)` in turn.
pub fn rebrand_with<'a>(text: &'a str, names: &[(&str, &str)]) -> Cow<'a, str> {
    let mut out = Cow::Borrowed(text);
    for (from, to) in names {
        if from != to && out.contains(from) {
            out = Cow::Owned(out.replace(from, to));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn files() -> Vec<(String, parse::Brand)> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut out = Vec::new();
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "env") {
                let id = path.file_stem().unwrap().to_string_lossy().to_string();
                let brand = parse::parse(&std::fs::read_to_string(&path).unwrap()).unwrap_or_else(|e| panic!("{id}.env: {e}"));
                out.push((id, brand));
            }
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    #[test]
    fn there_are_brand_files_and_each_follows_the_rules() {
        let all = files();
        assert!(all.iter().any(|(id, _)| id == "ntm") && all.iter().any(|(id, _)| id == "pixel"), "{:?}", all.iter().map(|a| &a.0).collect::<Vec<_>>());
        for (id, brand) in &all {
            parse::check(brand, id).unwrap_or_else(|e| panic!("{id}.env: {e}"));
        }
    }

    /// What NTM Digital's installers were released with. Changing any of these stops installed
    /// copies from updating, or makes the new installer sit beside them.
    #[test]
    fn ntm_keeps_the_identity_it_shipped_with() {
        let all = files();
        let ntm = &all.iter().find(|(id, _)| id == "ntm").unwrap().1;
        let get = |k: &str| parse::get(ntm, k);
        assert_eq!(get("EDITOR_UPGRADE_CODE"), "BB5F6996-DF46-4CB3-9742-12A7CE95C4F6");
        assert_eq!(get("EDITOR_COMPONENT_GUID"), "E229F7E2-2C3A-4508-989E-1A1250F3C81D");
        assert_eq!(get("PHOTO_UPGRADE_CODE"), "5C704DD5-97D3-444B-8773-212A037E822B");
        assert_eq!(get("PHOTO_COMPONENT_GUID"), "6344F377-2A6C-4596-B8CA-805F4D6EA367");
        assert_eq!((get("EDITOR_EXE"), get("EDITOR_FOLDER"), get("EDITOR_PRODUCT")), ("ntmdigital-editor", "NTMDigitalEditor", "NTM Digital Editor"));
        assert_eq!((get("PHOTO_EXE"), get("PHOTO_FOLDER"), get("PHOTO_PRODUCT")), ("ntmdigital-photo", "NTMDigitalPhoto", "NTM Digital Photo"));
        assert_eq!((get("COMPANY"), get("VENDOR_DIR"), get("REPO"), get("TAG_INFIX")), ("NTM Digital", "NTMDigital", "hylecuoicung/NTMapp", ""));
        // installed copies look for these tags
        assert_eq!(parse::tag_prefix(ntm, "editor"), "editor-");
        assert_eq!(parse::tag_prefix(ntm, "photo"), "photo-");
    }

    /// Two brands installed on one computer must not share anything the installer owns, and one
    /// brand's release tags must never be taken for another's (an app picks its releases by prefix).
    #[test]
    fn brands_share_no_identity() {
        let all = files();
        for (i, (a_id, a)) in all.iter().enumerate() {
            for (b_id, b) in &all[i + 1..] {
                for (p, name, _) in parse::APPS {
                    for k in ["UPGRADE_CODE", "COMPONENT_GUID", "EXE", "FOLDER", "PRODUCT"] {
                        let key = format!("{p}_{k}");
                        assert_ne!(parse::get(a, &key), parse::get(b, &key), "{a_id} and {b_id} share {key}");
                    }
                    let (ta, tb) = (format!("{}v", parse::tag_prefix(a, name)), format!("{}v", parse::tag_prefix(b, name)));
                    assert!(!ta.starts_with(&tb) && !tb.starts_with(&ta), "{a_id}'s tags {ta}* and {b_id}'s {tb}* overlap");
                }
                assert_ne!(parse::get(a, "COMPANY"), parse::get(b, "COMPANY"));
            }
            // within a brand, the apps differ too
            let codes: Vec<&str> =
                parse::APPS.iter().flat_map(|(p, _, _)| ["UPGRADE_CODE", "COMPONENT_GUID"].map(|k| parse::get(a, &format!("{p}_{k}")))).collect();
            let mut sorted = codes.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), codes.len(), "{a_id} reuses a GUID");
        }
    }

    #[test]
    fn the_built_brand_is_its_file() {
        let all = files();
        let (_, brand) = all.iter().find(|(id, _)| id == ID).unwrap_or_else(|| panic!("no {ID}.env"));
        let get = |k: &str| parse::get(brand, k);
        assert_eq!(COMPANY, get("COMPANY"));
        assert_eq!(SLUG, get("SLUG"));
        assert_eq!(VENDOR_DIR, get("VENDOR_DIR"));
        assert_eq!(REPO, get("REPO"));
        assert_eq!((editor::PRODUCT, editor::EXE, editor::FOLDER), (get("EDITOR_PRODUCT"), get("EDITOR_EXE"), get("EDITOR_FOLDER")));
        assert_eq!((photo::PRODUCT, photo::EXE, photo::FOLDER), (get("PHOTO_PRODUCT"), get("PHOTO_EXE"), get("PHOTO_FOLDER")));
        assert_eq!(editor::TAG_PREFIX, parse::tag_prefix(brand, "editor"));
        assert_eq!(photo::TAG_PREFIX, parse::tag_prefix(brand, "photo"));
        assert_eq!((DISCORD.url, WEBSITE.url, FACEBOOK.url), (get("DISCORD_URL"), get("WEBSITE_URL"), get("FACEBOOK_URL")));
        assert_eq!(IS_DEFAULT, ID == "ntm");
        // the names the sources spell are the default brand's
        let ntm = &all.iter().find(|(id, _)| id == "ntm").unwrap().1;
        assert_eq!(source::COMPANY, parse::get(ntm, "COMPANY"));
        assert_eq!(source::EDITOR_PRODUCT, parse::get(ntm, "EDITOR_PRODUCT"));
        assert_eq!(source::PHOTO_PRODUCT, parse::get(ntm, "PHOTO_PRODUCT"));
    }

    #[test]
    fn rebranding_swaps_the_products_then_the_company() {
        let names = [("NTM Digital Editor", "Pixel Digital Editor"), ("NTM Digital Photo", "Pixel Digital Photo"), ("NTM Digital", "Pixel Digital")];
        assert_eq!(rebrand_with("About NTM Digital Editor", &names), "About Pixel Digital Editor");
        assert_eq!(rebrand_with("NTM Digital Photo could not start", &names), "Pixel Digital Photo could not start");
        assert_eq!(rebrand_with("Copyright 2026. NTM Digital.", &names), "Copyright 2026. Pixel Digital.");
        assert_eq!(rebrand_with("Hide NTM Digital Editor", &names), "Hide Pixel Digital Editor");
        // nothing to swap: the text is not copied
        assert!(matches!(rebrand_with("Quit", &names), Cow::Borrowed("Quit")));
        // the same names change nothing
        assert!(matches!(rebrand_with("NTM Digital Editor", &[("NTM Digital Editor", "NTM Digital Editor")]), Cow::Borrowed(_)));
        // this build's own mapping is the identity for the default brand
        if IS_DEFAULT {
            assert!(matches!(rebrand("About NTM Digital Editor"), Cow::Borrowed(_)));
        } else {
            assert!(!rebrand(&format!("About {}", source::EDITOR_PRODUCT)).contains(source::COMPANY));
        }
    }

    /// The conventions `check` enforces are what makes the swap above produce each brand's real names.
    #[test]
    fn rebranding_the_default_names_gives_each_brands_own_names() {
        let all = files();
        let ntm = &all.iter().find(|(id, _)| id == "ntm").unwrap().1;
        for (id, brand) in &all {
            let names = [
                (parse::get(ntm, "EDITOR_PRODUCT"), parse::get(brand, "EDITOR_PRODUCT")),
                (parse::get(ntm, "PHOTO_PRODUCT"), parse::get(brand, "PHOTO_PRODUCT")),
                (parse::get(ntm, "COMPANY"), parse::get(brand, "COMPANY")),
            ];
            assert_eq!(
                rebrand_with("NTM Digital Editor, NTM Digital Photo, NTM Digital", &names),
                format!("{}, {}, {}", names[0].1, names[1].1, names[2].1),
                "{id}"
            );
        }
    }

    #[test]
    fn the_parser_reads_what_the_files_say() {
        let b = parse::parse("# comment\n\nA=1\r\nB = two words \nC=\nD=x=y\n").unwrap();
        assert_eq!(parse::get(&b, "A"), "1");
        assert_eq!(parse::get(&b, "B"), "two words");
        assert_eq!(parse::get(&b, "C"), "");
        assert_eq!(parse::get(&b, "D"), "x=y");
        assert!(parse::parse("A=1\nA=2\n").is_err());
        assert!(parse::parse("nonsense\n").is_err());
    }

    #[test]
    fn a_brand_that_breaks_a_rule_is_refused() {
        let all = files();
        let pixel = all.iter().find(|(id, _)| id == "pixel").unwrap().1.clone();
        let broken = |k: &str, v: &str| {
            let mut b = pixel.clone();
            b.insert(k.into(), v.into());
            parse::check(&b, "pixel").is_err()
        };
        assert!(broken("EDITOR_EXE", "pixel-editor"), "names follow the company");
        assert!(broken("PHOTO_UPGRADE_CODE", "not-a-guid"));
        assert!(broken("EDITOR_COMPONENT_GUID", "b55723d9-0f08-4823-aae6-56856e95c951"), "GUIDs are upper case");
        assert!(broken("TAG_INFIX", "Pixel"));
        assert!(broken("WEBSITE_URL", "http://pixeldigital.vn"), "links are https");
        assert!(broken("WEBSITE_LABEL", ""), "a link has a label");
        assert!(broken("DISCORD_LABEL", "Join"), "a label without a link");
        assert!(broken("REPO", "NTMapp"));
        let mut missing = pixel.clone();
        missing.remove("PHOTO_PRODUCT");
        assert!(parse::check(&missing, "pixel").is_err());
        let mut extra = pixel.clone();
        extra.insert("EDITOR_PRODCUT".into(), "x".into());
        assert!(parse::check(&extra, "pixel").is_err(), "a mistyped key");
        assert!(parse::check(&pixel, "ntm").is_err(), "the id is the file's name");
    }
}
