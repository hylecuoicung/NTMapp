# Brands

The apps in this repository are built under more than one brand: the same software and the same
logo, under another company name, with their own installers, install folders, registry keys and
release tags. A brand is one file here:

| File | Brand |
| --- | --- |
| `ntm.env` | NTM Digital (the default; the names the source code spells) |
| `pixel.env` | Pixel Digital |

`NTM_BRAND=pixel` picks one when building (`ntm` when unset). The workflows have a `brand` input
for the same thing (`ntm`, `pixel`, or `both`).

## What a brand file holds

Company and product names, the file names and install folders derived from them, the GUIDs that
make an installer a product of its own, and the community links. Empty link = the app shows no such
link. `build.rs` refuses a file that breaks the naming rules (names follow the company; GUIDs are
upper case; a link has both address and label), and `cargo test` in this folder checks that no two
brands share a GUID, an exe name, an install folder or a release-tag prefix.

* **Never change a `*_UPGRADE_CODE` or `*_COMPONENT_GUID`** after a build with it was released.
  An installer with another UpgradeCode does not update the installed copy; it installs beside it.
* Release tags are `<app>-<TAG_INFIX>v<version>`: `editor-v0.20.2` (NTM), `editor-pixel-v0.20.2`
  (Pixel). An installed app only offers updates from its own brand's tags.
* The English UI text in the source code spells the default brand's names ("NTM Digital Editor");
  `ntm_brand::rebrand` swaps them for the chosen brand's where text is shown. Do not hard-code a
  brand's name in new code: use `ntm_brand::editor::PRODUCT` and friends.

## Who reads these files

* the apps: the `ntm-brand` crate in this folder (`build.rs` hands the keys to `env!`);
* the installers and workflows: `brand.ps1` (`Get-Brand`);
* the tests: `cargo test` in this folder;
* the test-build workflows: `check-identity.ps1` compares the new installer's tables (UpgradeCode,
  component GUIDs, folders, registry keys, file names) with the newest published release of the same
  app and brand, and fails if the installer would no longer update the copies people have.

## Adding a brand

1. Copy `pixel.env` to `<id>.env` (lower-case letters and digits), change every value, and give it
   new GUIDs (`[guid]::NewGuid().ToString().ToUpper()`).
2. `cargo test` in this folder; then `NTM_BRAND=<id> cargo check` in an app.
3. Add the id to the `brand` choices of the four workflows in `.github/workflows/`
   (`*-test-build.yml`, `*-release.yml`; `both` means every brand there), and run the test build
   workflow with the new brand.

## Tests

`cargo test` here checks the files and that NTM's identity is unchanged. The apps' own tests expect
the default brand's text, except the ones that go through `ntm_brand` (the release feed, the links,
the title); run `NTM_BRAND=pixel cargo test -p <crate> <filter>` for those.

## Adding an app

Give every brand file the app's keys (`<APP>_PRODUCT`, `_EXE`, `_FOLDER`, `_UPGRADE_CODE`,
`_COMPONENT_GUID`), add the app to `APPS` in `src/parse.rs` and a module for it in `src/lib.rs`.
