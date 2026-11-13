<p align="center">
  <img src="assets/app-icon/hicolor/256x256/apps/ai.storyteller.photocraft.png" width="128" height="128" alt="NTM Digital Photo icon">
</p>

# NTM Digital Photo

An image editor for Windows, from NTM Digital.

NTM Digital Photo is a modified version of [PhotoCraft](https://github.com/storytold/photocraft)
by the ArtCraft team and the PhotoCraft contributors. It has its own name, installer and update
channel. It is not an ArtCraft product, and the ArtCraft team does not support it: report problems
with this build here, not to them.

## Download

Get the latest build from the repository's [Releases](https://github.com/hylecuoicung/NTMapp/releases) page.
The photo editor's releases are the ones tagged `photo-v<version>` (the `editor-v…` ones are NTM
Digital Editor's):

| File | What it is |
|---|---|
| `ntmdigital-photo-<version>-windows-x64.msi` | Installer. Run it again with a newer version to update. |
| `ntmdigital-photo-<version>-windows-x64-portable.zip` | Portable copy: unzip and run `ntmdigital-photo.exe`. |
| `SHA256SUMS.txt` | Checksums of the files above. |

The builds are not code-signed, so Windows warns before it runs the installer.

When a newer release is published, the app says so at start and offers a Download button. Help ▸
Check for Updates asks on demand. Nothing installs by itself.

It installs next to NTM Digital Editor (the video editor) without touching it: the two have separate
installers, folders and settings.

## Build it yourself

This app is the `photo/` folder of the [NTMapp](https://github.com/hylecuoicung/NTMapp) repository. On
Windows, with Rust (MSVC toolchain), the Windows SDK and WiX 5.0.2 (with its UI and Util extensions),
from this folder:

```powershell
pwsh packaging/windows/package.ps1 -Arch x64
```

The installer and the portable zip land in `dist/release`.

Two GitHub Actions workflows at the repository root (`.github/workflows/photo-*.yml`) do the same on a
hosted runner:

| Workflow | What it does |
|---|---|
| NTM Digital Photo: test build | The installer and portable zip as a workflow artifact; nothing is published. |
| NTM Digital Photo Release | Sets the version, builds, and publishes a release (tag `photo-v<version>`) that installed apps are told about. Leave the version empty to continue the numbering, or give one such as `0.24.0`. |

The documentation in `docs/` and `book/` is upstream PhotoCraft's and describes the editor itself.

## Licence

Like PhotoCraft, this code is licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at
your option. Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors; changes in this
version copyright (c) 2026 NTM Digital. Required notices are in [NOTICE](NOTICE), and every
bundled asset is listed in [ATTRIBUTION.md](ATTRIBUTION.md).

The ArtCraft name and logos are trademarks of the ArtCraft Team. This version does not use them.

The NTM Digital name and logo are trademarks of NTM Digital and are not covered by the licence
above: see [assets/app-icon/LICENSE.txt](assets/app-icon/LICENSE.txt). If you publish your own
modified version, give it your own name and artwork.
