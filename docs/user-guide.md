# User guide

## The window

- **App bar** (top): New, Open, Save as…, Save, and the ⚙ options button.
- **Tabs**: one per open skin. A tab with unsaved changes asks for
  confirmation before it closes.
- **Left panel**: tools, the colour palette, and undo/redo.
- **Centre**: the 3D model you paint on.
- **Right panel**: the colour picker, the arm model, layer visibility, and
  view options.

With no tabs open, the window shows New and Open buttons.

## Opening and saving

- **New** (Ctrl+N) opens a tab with the default character, named
  "Untitled", "Untitled 2", and so on.
- **Open** (Ctrl+O) lets you choose one or more PNG files. Each one opens
  in its own tab. If a file is already open, the app switches to its tab.
- **Save** (Ctrl+S) writes to the tab's file. If the tab has never been
  saved, it works like Save as.
- **Save as…** (Ctrl+Shift+S) asks for a new location.

You can also open files by passing them on the command line:
`skin-editor a.png b.png`.

### Supported files

| Size | What happens |
| --- | --- |
| 64×64 | Opened as is |
| 64×32 (legacy) | Upgraded to 64×64. The left arm and left leg are mirrored from the right ones. Saving writes a 64×64 file. |
| Anything else | Refused with an error message |

Skins are always saved as 64×64 RGBA PNG files.

## Moving around the model

| Action | Mouse |
| --- | --- |
| Paint with the current tool | Left-drag on the model |
| Rotate | Left-drag beside the model, or right-drag anywhere |
| Move (pan) | Middle-drag (hold the scroll wheel) |
| Zoom | Scroll |
| Return to the starting view | **Reset view** in the right panel |

Turn on **Pixel grid** in the right panel to see where texels meet.

## Tools

| Tool | What it does |
| --- | --- |
| **Pencil** | Paints one texel at a time with the selected colour. Fast strokes are filled in, so they leave no gaps. |
| **Eraser** | Makes texels fully transparent. |
| **Fill** | Floods the connected area of the same colour. It stays on the face you clicked, such as the front of the head, and never spills onto other faces. |
| **Pick colour** | Copies the colour under the cursor into the selected palette slot, then switches back to the Pencil. |

Each stroke, fill or overlay clear is one undo step. Each tab keeps up to 200
steps. A stroke that changes nothing is not added.

## Colours

The palette has eight slots. Click a slot to select it. To change the colour
of the selected slot:

- drag in the colour square (saturation and brightness) or along the hue
  strip,
- type a hex value such as `#3A7BD5` (the `#` is optional), or
- type 0–255 into the R, G and B fields.

Painting is always fully opaque. To make texels transparent, use the Eraser.

## Layers

A Minecraft skin has two layers:

- **Base**: the head, body, arms and legs.
- **Overlay**: a slightly larger shell around the base, used for the hat,
  jacket, sleeves and trouser legs.

**Tools always draw on the outermost layer that is shown.** Where the overlay
is visible, you paint on the overlay. Where it is hidden, you paint on the
base underneath.

The **Layers** section has:

- **Base layer** and **Overlay layer** switches, which show or hide a whole
  layer;
- a checkbox grid that shows or hides each body part (head, body, right arm,
  left arm, right leg, left leg) for each layer. Hiding parts is how you
  reach surfaces that are normally covered, such as the inside of an arm or
  the top of a leg;
- **Clear overlay**, which erases the whole overlay layer after you confirm.
  You can undo it.

Visibility only changes what you see and what you can paint. It is not saved
in the skin.

## Arm models

- **Classic**: 4 pixels wide arms (the "Steve" shape).
- **Slim**: 3 pixels wide arms (the "Alex" shape).

When a skin opens, the app guesses the model from the texture: if the columns
that only Classic arms use are empty, the skin is treated as Slim. You can
switch at any time. Switching changes how the texture wraps around the model.
It does not change the texture itself.

## Options

The ⚙ button opens the options dialog:

- **Colour**: blue, green, red, purple or yellow accent.
- **Theme**: light or dark.

The options are saved between runs in a small text file:

| OS | Location |
| --- | --- |
| Linux | `~/.config/skin-editor/settings` |
| Linux (Flatpak) | `~/.var/app/io.github.SkinEditor/config/skin-editor/settings` |
| Windows | `%APPDATA%\skin-editor\settings` |

On the first run, the theme follows the system's light or dark setting.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| Ctrl+N | New skin |
| Ctrl+O | Open |
| Ctrl+S | Save |
| Ctrl+Shift+S | Save as |
| Ctrl+Z | Undo |
| Ctrl+Shift+Z, Ctrl+Y | Redo |
| Ctrl+W | Close tab |
| Esc | Close the open dialog |

## Known limitations

- There is no flat 2D texture view yet. All painting happens on the 3D model.
- The brush is always one texel, and there is no mirror or symmetry mode.
- There is no colour transparency. Painted texels are fully opaque.
- Undo history is lost when a tab closes.
