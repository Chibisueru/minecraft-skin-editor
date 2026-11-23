# Building and packaging

## From source

You need a recent stable Rust toolchain ([rustup](https://rustup.rs)).

```sh
cargo build --release
./target/release/skin-editor [skin.png …]
```

No `-dev` packages are needed. iced loads Wayland, X11 and the GPU drivers at
run time, and file dialogs go through the XDG desktop portal on Linux and the
native dialog on Windows and macOS.

Run the tests with:

```sh
cargo test
```

## Packages

Both scripts are run from anywhere in the repository and write to `dist/`.

### Flatpak (Linux)

```sh
packaging/build-flatpak.sh     # -> dist/skin-editor.flatpak
```

Requirements:

- `flatpak` (the script uses `flatpak build-init`, `build-finish`,
  `build-export` and `build-bundle`; `flatpak-builder` is not needed)
- the runtime: `flatpak install flathub org.freedesktop.Platform//26.08`
- either `cargo`, or podman (`CONTAINER_ENGINE=docker` to use Docker
  instead) to build in a `rust:1-bookworm` container

The script builds the release binary on the host and copies it into the
bundle. No SDK is needed. Because of this, the runtime has to carry a glibc at
least as new as the build host's. When cargo is missing, the container build
uses Debian bookworm's older glibc, so the result runs on the runtime either
way.

The sandbox gets Wayland, X11 fallback, IPC and the GPU (`--device=dri`). It
has no home folder access: files are opened and saved through the desktop
portal, and the light/dark preference comes from the settings portal.

Install and run:

```sh
flatpak install --user dist/skin-editor.flatpak
flatpak run io.github.SkinEditor
```

### Windows

```sh
packaging/build-windows.sh     # -> dist/skin-editor.exe
```

This cross-compiles for `x86_64-pc-windows-gnu` with MinGW inside a
`rust:1-bookworm` container, so the only thing you need on the host is podman
(or `CONTAINER_ENGINE=docker`). The source is mounted read-only and copied
inside the container, so your `Cargo.lock` stays untouched.

| Variable | Default | Meaning |
| --- | --- | --- |
| `CONTAINER_ENGINE` | `podman` | Container tool to use |
| `JOBS` | `6` | Parallel compile jobs |

The first run downloads the image and compiles every dependency. Later runs
reuse `target/windows-cache/`.

## Making a release

1. Bump `version` in `Cargo.toml` and add an entry to `CHANGELOG.md`.
2. Build both packages with the scripts above.
3. Tag and publish:

   ```sh
   git tag -a vX.Y.Z -m "vX.Y.Z"
   git push origin vX.Y.Z
   gh release create vX.Y.Z dist/skin-editor.flatpak dist/skin-editor.exe \
     --title "vX.Y.Z" --notes-file <notes>
   ```

   Add `--prerelease` for test builds.

`dist/` is ignored by git. Binaries go on the release page, not in the
repository.
