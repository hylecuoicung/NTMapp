# Camera Raw Local Adjustments (Linear and Radial Gradient masks)

Camera Raw's **Local Adjustments** section confines a set of develop sliders to part of the
picture. It is original work: the behaviour follows Adobe's public description of the Masking
panel, and the maths is plain geometry plus the develop pipeline this editor already has. It does
not claim numerical identity with Adobe's develop pipeline.

## What a mask is

A mask has a **shape**, an **Amount**, an **Invert** switch and its own **sliders**: Exposure,
Contrast, Highlights, Shadows, Whites, Blacks, Temperature, Tint, Vibrance, Saturation, Texture,
Clarity and Dehaze, in the units of the global panels of the same names.

| Shape | Where it applies |
|---|---|
| Linear Gradient | Full effect at the **start** point, none at the **end** point, a straight ramp between them along the line joining them. Beyond the start it is full, beyond the end it is none. |
| Radial Gradient | Inside an ellipse at the **center** with a half-width and a half-height (as fractions of the picture's width and height), turned by **Angle** degrees clockwise. The effect is full inside and fades to none at the edge over the outer **Feather** percent of the radius. Feather 0 is a hard edge about a pixel wide. |

Invert swaps covered and uncovered. Positions are stored as fractions of the picture (0 at the
left or top, 1 at the right or bottom), so a mask keeps its place on the reduced preview and on the
full image, and may sit up to one picture's width outside it.

## How it develops

The picture is developed with the global sliders first. Then each active mask, in list order,
develops a copy of that result with the mask's sliders alone (the same stages, with the document's
encoding and the preview's pixel scale carried over), and the copy is mixed over the picture by
the mask's coverage times its Amount. So:

* masks stack, and a later mask sees the earlier ones' result;
* every slider means what it means in the global panels;
* alpha is never touched;
* a mask that is switched off, has Amount 0 or all sliders at 0 changes nothing, and a filter
  with only such masks is an identity.

Each active mask costs one more develop pass, so at most **16** masks are kept. The develop with
masks runs on the CPU; the GPU preview handles only settings without masks.

## Using it

Open **Local Adjustments** in the Camera Raw settings panel and choose **Linear Gradient** or
**Radial Gradient**. The new mask is selected and drawn over the preview:

* **Linear:** a filled handle at the start and an open one at the end; drag either. The two lines
  across the picture mark full effect and no effect.
* **Radial:** drag the center handle to move the ellipse, and the open handles on its axes to set
  the width and the height. A second outline shows where the feather begins. The Angle and
  Feather sliders are in the panel.
* Every position also has a slider, for precise values. Delete removes a mask, **On** switches it
  off without losing it, and a mask can be edited again after it is switched back on.

The guides are not drawn in Before view. Cancel discards the masks with the other settings;
dragging a handle is one undo step.

The section is in the Camera Raw Filter and in a Smart Filter's dialog. It is not offered in the
open-time dialog for a raw file: the `.xmp` sidecar that dialog writes has no place for masks.
Open the raw, then use Filter ▸ Camera Raw Filter.

## Automation

`filter.cameraRaw` takes `masks`: an array of objects with `shape` (`"linear"` or `"radial"`),
`enabled`, `amount` (0–100), `invert`, `start` and `end` (linear), `center`, `radius`, `angle` and
`feather` (radial), and the slider names above. Everything is optional; a mask with only a shape
is a default mask without effect. A new setting must be valid: at most 16 masks, finite numbers,
`exposure` in −5…5, the other sliders and `feather` and `amount` in their ranges, `radius` from
0.001 to 2. A smart filter's stored settings are read leniently (the list is cut at 16 and every
number is bounded) so they still render. The Camera Raw dialog's `ui.set` accepts `masks` the same
way, and `ui.inspect.cameraRaw.maskSelected` is the mask being edited.

## PSD

Masks have no verified Camera Raw Filter mapping. A PSD save that would write a filter with active
masks reports the usual warning and leaves that filter out, and neutral masks are the default, so
they are written as nothing. They survive `.pcraft`, undo, and reopening the dialog.

## Not yet

Brush, Color Range, Luminance Range, Depth Range, Select Subject, Select Sky and Select
Background masks; mask subtraction and intersection; a mask's own noise, sharpening, curve and
color grading; and a GPU path for masks.
