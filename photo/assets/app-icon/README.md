# NTM Digital Photo app icon

The **logo of NTM Digital Photo**: a cyan camera with a teal field at its lower left, a lens with
one glint inside a white ring, and one loose cyan square at the lower right, on a transparent
ground. The NTM Digital Editor logo has the same ring and square. It stays readable at
16 px.

## Palette

| Colour | Hex | Used for |
|---|---|---|
| Cyan | `#01b1de` | camera body and the loose square |
| Teal | `#3da992` | field at the lower left, fading into the cyan at the bottom |
| White | `#ffffff` | lens ring and the glint |
| Near-black | `#2f2d2f` | lens |

## Geometry

A 512-unit canvas (`viewBox="0 0 512 512"`) with no background; the mark fills 94 % of its width.
One path for the camera (three rounded corners, a notch for the loose square), the teal field
clipped to it (a crisp arc on the left, a short gradient into the cyan under the lens), one square
and three circles. The macOS renders pad it onto Apple's 824/1024 icon grid; the Windows and Linux
renders use it at full size.

## Provenance

The owner's logo (a 1254 px raster), redrawn as paths and circles fitted to it and compared with it
pixel by pixel. It replaced the first mark (a lens over four colour blocks) on 2026-11-12. Licence: see `LICENSE.txt` (a trademark, not open source).

## Files

The files keep the names upstream PhotoCraft uses, so the build and packaging scripts find them.

- `photocraft.svg`: canonical master. `photocraft-small.svg` is the same drawing; it is also the
  hicolor scalable icon.
- `photocraft-1024.png`: 1024 px render on the macOS grid.
- `photocraft.icns`: macOS bundle icon (`CFBundleIconFile`).
- `photocraft.ico`: Windows icon, 16–256 px, embedded in the `.exe` by `apps/photocraft/build.rs`.
- `hicolor/<size>/apps/ai.storyteller.photocraft.png` (16–512) and `hicolor/scalable/...svg`:
  Linux icon theme. The 128 px render is also the title bar's mark and the update prompt's icon,
  and the 256 px one is the window icon and the installer wizard's art.

The app also sets the window icon and Wayland app ID at runtime (`apps/photocraft/src/app_icon.rs`,
`main.rs`). On Windows the taskbar shows the icon of the Start Menu shortcut that launches the
exe, so the MSI's `<Icon Id>` keeps the `.exe` extension (ICE50); `app_icon.rs` documents the
details and tests the `.ico` sizes and the WiX icon references.

## Regenerate

Replace `photocraft.svg` (and `photocraft-small.svg`), then run `packaging/icons.sh`. It needs
`resvg`, plus `iconutil` on macOS; the `.ico` is packed by `cargo xtask ico`.
