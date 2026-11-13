# NTM Digital Editor app icon

The logo of NTM Digital Editor, the video editor: a magenta clapperboard with an orange stripe on its
clapper, a play button inside a white lens ring, and one loose magenta square at the lower right
(the NTM Digital Photo logo has the same ring and square). It stays readable at 16 px.

The files keep the names upstream FilmCraft uses (`filmcraft.*`, `ai.storyteller.filmcraft.*`), so the
build and packaging scripts find them.

## Palette

| Colour | Hex | Used for |
|---|---|---|
| Magenta | `#da0257` | clapper, body and the loose square |
| Orange | `#fcae07` | stripe on the clapper |
| White | `#ffffff` | lens ring and the play triangle |
| Near-black | `#2a292b` | disc behind the play triangle |

## Geometry

- A 512-unit viewBox with a transparent background: the mark is a shape, not a tile. Its longer side
  (the height) takes 94 % of the canvas, and the artwork is centred.
- Three rounded paths (the clapper, the body with its notch, the play triangle), the stripe, one
  square and two circles that share a centre. The clapper and the body's top edge are tilted, as
  in the owner's logo.
- macOS renders (`filmcraft.icns`, `filmcraft-macos-512.png`) put the artwork in Apple's grid (824 of
  1024 px, transparent margin). Windows, Linux and the web use it at full size.

## Provenance

Redrawn as vector from the owner's 1254 px logo: the shapes were fitted to the raster and the result
compared with it pixel by pixel. It replaced the first mark (a lens over four colour blocks) on
2026-11-12. Licence: [LICENSE.txt](LICENSE.txt). The logo is an NTM Digital
trademark and is not open source.

## Files

| File | What |
|---|---|
| `filmcraft.svg` | canonical artwork |
| `filmcraft-small.svg` | the same drawing (upstream keeps a lighter trace under this name) |
| `filmcraft-1024.png` | 1024 px render |
| `filmcraft-macos-512.png` | runtime Dock icon on macOS (embedded by `apps/filmcraft/src/main.rs`) |
| `filmcraft.icns` | macOS bundle icon (`CFBundleIconFile`) |
| `filmcraft.ico` | Windows icon, 16–256 px (embedded in the exe by `apps/filmcraft/build.rs`, and the installer's icon) |
| `hicolor/<size>/apps/ai.storyteller.filmcraft.png`, `hicolor/scalable/…svg` | Linux icon theme, used by `packaging/linux/ai.storyteller.filmcraft.desktop` |

The web app's `apps/filmcraft-web/web/favicon.png` (128 px) comes from the same artwork. In the app,
`crates/ui-egui/src/brand.rs` draws a hicolor render of fitting size next to the name on Home and in About, and the
update prompt shows the 128 px one.

## How the icon reaches each OS

- **Windows:** `build.rs` embeds `filmcraft.ico` as the exe's resource icon (Explorer, Start menu,
  pinned taskbar), the installer uses it for its shortcut and its Apps & features entry, and the
  runtime icon covers the window, taskbar and Alt-Tab.
- **macOS:** the window icon set at startup (`ViewportBuilder::with_icon`) is the Dock and app-switcher
  icon when the app runs unbundled. An `.app` bundle points `CFBundleIconFile` at `filmcraft.icns`.
- **Linux:** the runtime icon plus `with_app_id("ai.storyteller.filmcraft")`, which Wayland matches to
  the `.desktop` file. Install the `hicolor/` tree under `/usr/share/icons/hicolor/` and the desktop
  file under `/usr/share/applications/`.

## Regenerate

`packaging/icons.sh` (needs `resvg`; `iconutil` on macOS for the `.icns`; the `.ico` is packed by
`cargo xtask ico`). To change the artwork, replace `filmcraft.svg` and `filmcraft-small.svg`, then run
the script. Every derived file needs its `.attribution` sidecar and a row in `ATTRIBUTION.md`.
