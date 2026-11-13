# NTM Digital Editor

A non-linear video editor for Windows, from NTM Digital.

NTM Digital Editor is a modified version of [FilmCraft](https://github.com/storytold/filmcraft) by the
ArtCraft team and the FilmCraft contributors. It has its own name, installer and update channel.
It is not an ArtCraft product, and the ArtCraft team does not support it: report problems with
this build here, not to them.

## Download

Get the latest build from the repository's [Releases](https://github.com/hylecuoicung/NTMapp/releases) page.
The editor's releases are the ones tagged `editor-v<version>` (the `photo-v…` ones are NTM Digital
Photo's):

| File | What it is |
|---|---|
| `ntmdigital-editor-<version>-windows-x64.msi` | Installer. Run it again with a newer version to update. |
| `ntmdigital-editor-<version>-windows-x64-portable.zip` | Portable copy: unzip and run `ntmdigital-editor.exe`. |
| `SHA256SUMS.txt` | Checksums of the files above. |

The builds are not code-signed, so Windows warns before it runs the installer.

When a newer release is published, the app says so at start and offers a Download button. Help ▸
Check for Updates asks on demand. Nothing installs by itself.

## Build it yourself

This app is the `editor/` folder of the [NTMapp](https://github.com/hylecuoicung/NTMapp) repository. On
Windows, with Rust (MSVC toolchain), the Windows SDK and WiX 5.0.2 (with its UI and Util extensions),
from this folder:

```powershell
pwsh packaging/windows/package.ps1 -Arch x64
```

The installer and the portable zip land in `dist/release`.

Two GitHub Actions workflows at the repository root (`.github/workflows/editor-*.yml`) do the same on a
hosted runner:

| Workflow | What it does |
|---|---|
| NTM Digital Editor: test build | The installer and portable zip as a workflow artifact; nothing is published. |
| NTM Digital Editor Release | Sets the version, builds, and publishes a release (tag `editor-v<version>`) that installed apps are told about. Leave the version empty to continue the numbering, or give one such as `0.21.0`. |

## Licence

Like FilmCraft, this code is licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at
your option. Copyright (c) 2026 ArtCraft Team and the FilmCraft contributors; changes in this
version copyright (c) 2026 NTM Digital. Required notices are in [NOTICE](NOTICE), and every
bundled asset is listed in [ATTRIBUTION.md](ATTRIBUTION.md).

The ArtCraft name and logos are trademarks of the ArtCraft Team. This version does not use them.

The NTM Digital name and logo are trademarks of NTM Digital and are not covered by the licence
above: see [assets/app-icon/LICENSE.txt](assets/app-icon/LICENSE.txt). If you publish your own
modified version, give it your own name and artwork.
