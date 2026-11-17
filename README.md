# NTMapp

NTM Digital's desktop apps for Windows, each in its own folder:

| Folder | App | Built on | Release tags |
|---|---|---|---|
| [`editor/`](editor/) | **NTM Digital Editor**, a non-linear video editor | [storytold/filmcraft](https://github.com/storytold/filmcraft) | `editor-v<version>` |
| [`photo/`](photo/) | **NTM Digital Photo**, an image editor | [storytold/photocraft](https://github.com/storytold/photocraft) | `photo-v<version>` |
| [`brand/`](brand/) | The brands the apps are built under | | |

## Brands

The same apps are also built under a second name, **Pixel Digital** (Pixel Digital Editor, Pixel
Digital Photo): the same software and logo with another company name, its own installers, install
folders, registry keys, links and release tags (`editor-pixel-v<version>`, `photo-pixel-v<version>`).
The two can be installed side by side, and each only ever offers its own brand's updates. A brand is
one small file in [`brand/`](brand/) (`ntm.env`, `pixel.env`); see [`brand/README.md`](brand/README.md).
`NTM_BRAND=pixel` picks one when building; the workflows take a `brand` input (`ntm`, `pixel` or `both`).

## Download

The [Releases](https://github.com/hylecuoicung/NTMapp/releases) page lists both apps, under both brands;
the tag says which app and which brand a release is. Each release has an installer (`.msi`), a portable copy (`-portable.zip`)
and `SHA256SUMS.txt`. The builds are not code-signed, so Windows warns before it runs an installer.
Each app checks this repository for a newer release of itself when it starts and offers a
Download button; nothing installs by itself.

## Build and release

Each folder is a complete Rust workspace with its own README: build from inside the folder. The
GitHub Actions workflows live here, in [`.github/workflows/`](.github/workflows/):

| Workflow | What it does |
|---|---|
| NTM Digital Editor: test build, NTM Digital Photo: test build | The installer and portable zip as a workflow artifact; nothing is published. |
| NTM Digital Editor Release, NTM Digital Photo Release | Sets the version, builds, and publishes a release for the chosen brand (or both, with the same version). Leave the version empty to continue the app's numbering. |

The `.github/` folders inside `editor/` and `photo/` are upstream's; GitHub does not run them.

## Taking upstream's changes

Each folder is a squashed [git subtree](https://git-scm.com/book/en/v2/Git-Tools-Advanced-Merging#_subtree_merge)
of its upstream repository. To bring in upstream's newer commits:

```sh
git subtree pull --prefix=editor https://github.com/storytold/filmcraft main --squash
git subtree pull --prefix=photo https://github.com/storytold/photocraft main --squash
```

Conflicts come up in the files this repository changes: names and links, the About dialog, the
Home screen, the update check and the Windows packaging.

## Licence

Both apps are MIT OR Apache-2.0, like the projects they are built on; each folder has the licence
texts, `NOTICE` and `ATTRIBUTION.md`. The NTM Digital and Pixel Digital names and the logos are trademarks of their owner
and are not covered by those licences (see `assets/app-icon/LICENSE.txt` in each folder). If you
publish your own modified version, give it your own name and artwork.
