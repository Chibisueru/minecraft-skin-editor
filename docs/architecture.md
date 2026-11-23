# Architecture

The app is one binary crate with four modules and no build script.

```
src/
├── main.rs          iced application: state, messages, layout, canvases
├── skin.rs          texture, player geometry, camera, software ray caster
├── material.rs      Material 3 colour schemes, type scale, widget styles, icons
└── default_skin.rs  the starter character as embedded PNG bytes
assets/fonts/        Material Symbols Rounded (outlined + filled), subset
packaging/           Flatpak and Windows build scripts, .desktop file, icon
```

## `skin.rs`: the model

- **`Skin`**: a 64×64 RGBA texture. Handles loading (and upgrading legacy
  64×32 skins by mirroring the right limbs onto the left), saving, flood fill
  limited to one face, clearing the overlay, and guessing Classic or Slim
  from which arm columns are used.
- **`BoxDef` / `boxes(model)`**: the twelve cuboids of the player model
  (six base parts and six overlay shells), each with its size, position, UV
  origin and layer. Classic and Slim differ only in arm width.
- **`Visibility`**: which of the six parts are shown on each layer.
- **`Camera`**: an orbit camera (yaw, pitch, distance, pan) around the
  model's centre.
- **`Scene`**: boxes, visibility and texture together. It answers two
  questions with the same ray/box intersection code:
  - `render` casts one ray per pixel to draw the preview as an image (with an
    optional texel grid);
  - `pick_visible` / `pick_surface` find the texel under the cursor. This is
    how "paint on the outermost visible layer" works: the ray hits the
    overlay box first wherever that part's overlay is shown.

Because the preview is ray cast on the CPU, what you see and what you paint
always come from the same geometry. There is no separate hit-testing mesh
that could drift out of sync.

## `main.rs`: the application

The usual iced structure: `SkinEditor` (state), `Message`, `update`, `view`,
and `subscription` (keyboard shortcuts, plus a 250 ms tick while a snackbar
is shown).

- **`Document`** is one tab: the skin, the last saved copy (for the
  unsaved-changes check), model, visibility, camera, and undo/redo stacks of
  whole `Skin` snapshots (up to 200; a 64×64 texture is 16 KiB). The rendered
  preview is cached and keyed on a revision counter, the viewport size and
  the grid flag.
- **Canvases**:
  - `Viewport` turns mouse input into `ViewEvent`s (stroke, rotate, pan,
    zoom) and draws the cached render.
  - `Palette` is the eight colour slots.
  - `Picker` is the saturation/value square and hue strip. It is drawn as
    two stacked canvases so the handles sit above the gradient images.
- **Strokes**: `StrokeMove` walks the segment from the last point in
  1.5 px steps, so fast drags leave no gaps. A checkpoint is pushed at
  stroke start and dropped at stroke end if nothing changed.
- **Settings**: accent and theme are stored as `key=value` lines in
  `<config dir>/skin-editor/settings`. Without that file, the theme follows
  the system preference through `dark-light`.

## `material.rs`: the look

- Light and dark schemes are generated from an accent seed colour with
  `material-colors`.
- The M3 type scale is exposed as helpers (`title_small`, `body_medium`, …).
- There are style functions for each widget used: filled, tonal, outlined
  and text buttons, nav items, tabs, cards, switches, checkboxes, text
  fields, dialogs, the scrim, and the snackbar.
- `Icon` maps names to Material Symbols codepoints.

### Adding an icon

The fonts in `assets/fonts/` are subset to only the glyphs the app uses.

1. Look up the icon's codepoint in Material Symbols' `.codepoints` file.
2. Add that codepoint to the subset of both the outlined and the filled font
   (e.g. with `pyftsubset --unicodes=…`) and replace the files.
3. Add a variant to `Icon` in `src/material.rs`.

## Dependencies

| Crate | Why |
| --- | --- |
| `iced` 0.13 | GUI (canvas, image, tokio features) |
| `image` | PNG decode/encode only |
| `rfd` | Native file dialogs (XDG portal on Linux) |
| `dirs` | Locating the config folder |
| `dark-light` | Reading the system light/dark preference |
| `material-colors` | Generating M3 schemes from a seed colour |
