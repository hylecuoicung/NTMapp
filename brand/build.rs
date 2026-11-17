//! Hands the chosen brand to the compiler. `NTM_BRAND` names it (`ntm`, the default, or `pixel`):
//! the file `<id>.env` next to this one is checked and every key becomes `NTM_BRAND_<KEY>` for
//! `env!` in lib.rs. The default brand's names are handed over too (`NTM_BRAND_SRC_*`): the
//! sources spell them, and `rebrand` swaps them for the chosen ones.

#[path = "src/parse.rs"]
mod parse;

use std::path::Path;

const DEFAULT: &str = "ntm";

fn load(dir: &Path, id: &str) -> parse::Brand {
    let path = dir.join(format!("{id}.env"));
    println!("cargo:rerun-if-changed={}", path.display());
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("brand {id:?}: can't read {}: {e}", path.display()));
    let brand = parse::parse(&text).unwrap_or_else(|e| panic!("brand {id:?} ({}): {e}", path.display()));
    if let Err(e) = parse::check(&brand, id) {
        panic!("brand {id:?} ({}): {e}", path.display());
    }
    brand
}

fn main() {
    println!("cargo:rerun-if-env-changed=NTM_BRAND");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/parse.rs");
    let dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()));
    let id = std::env::var("NTM_BRAND").map(|s| s.trim().to_lowercase()).ok().filter(|s| !s.is_empty()).unwrap_or_else(|| DEFAULT.to_string());
    let chosen = load(&dir, &id);
    let default = load(&dir, DEFAULT);
    for (k, v) in &chosen {
        println!("cargo:rustc-env=NTM_BRAND_{k}={v}");
    }
    for (p, name, _) in parse::APPS {
        println!("cargo:rustc-env=NTM_BRAND_{p}_TAG_PREFIX={}", parse::tag_prefix(&chosen, name));
    }
    for k in ["COMPANY", "EDITOR_PRODUCT", "PHOTO_PRODUCT"] {
        println!("cargo:rustc-env=NTM_BRAND_SRC_{k}={}", parse::get(&default, k));
    }
}
