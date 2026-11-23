# Skin Editor

A desktop editor for Minecraft skins. You paint directly on a 3D player model
that you can rotate, move and zoom. The interface follows Material 3, and the
app is written in Rust with [iced](https://iced.rs).

- Paint, erase, fill and pick colours straight on the 3D model
- Base layer and overlay layer, with each body part shown or hidden per layer
- Classic (4 px) and Slim (3 px) arm models, detected when a skin is opened
- Opens 64×64 skins and legacy 64×32 skins, which are upgraded to 64×64
- Several skins open at once in tabs, each with up to 200 undo steps
- Eight-slot palette with a colour square, hue strip, and hex and RGB fields
- Light and dark themes in five accent colours

## Download

Builds are on the [Releases](../../releases) page.

| Platform | File | How to run |
| --- | --- | --- |
| Linux | `skin-editor.flatpak` | `flatpak install --user skin-editor.flatpak`, then `flatpak run io.github.SkinEditor` |
| Windows | `skin-editor.exe` | Run the `.exe`; no installer is needed |

The Flatpak needs the `org.freedesktop.Platform//26.08` runtime from Flathub.
Flatpak offers to install it if it is missing.

## Build from source

```sh
cargo run --release                     # start with the default character
cargo run --release -- skin.png …       # open one or more skins in tabs
```

You need a recent stable Rust toolchain. No system development libraries are
needed, because the windowing and graphics libraries are loaded at run time.
[docs/building.md](docs/building.md) explains how to build the Flatpak and the
Windows `.exe`.

## Documentation

- [User guide](docs/user-guide.md): tools, layers, models, colours, and keyboard shortcuts
- [Building and packaging](docs/building.md): building from source, the Flatpak, and the Windows cross-build
- [Architecture](docs/architecture.md): how the code is organised, for contributors
- [Changelog](CHANGELOG.md)

## Quick reference

| Action | How |
| --- | --- |
| Paint | Drag on the model with the left button |
| Rotate | Drag beside the model, or drag with the right button |
| Move | Hold the scroll wheel and drag |
| Zoom | Scroll |
| New / open / save / save as | Ctrl+N / Ctrl+O / Ctrl+S / Ctrl+Shift+S |
| Undo / redo | Ctrl+Z / Ctrl+Shift+Z or Ctrl+Y |
| Close tab | Ctrl+W |

## License

GPL-3.0-or-later. See [LICENSE](LICENSE). The bundled Material Symbols font
is Apache-2.0 licensed (see [assets/fonts/LICENSE-MaterialSymbols](assets/fonts/LICENSE-MaterialSymbols)).

Minecraft is a trademark of Mojang AB. This project is not affiliated with or
endorsed by Mojang or Microsoft.
