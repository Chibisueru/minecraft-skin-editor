#!/usr/bin/env bash
# Packs the release binary into a single-file Flatpak bundle.
#
#   packaging/build-flatpak.sh        ->  dist/skin-editor.flatpak
#
# The binary is built on the host and copied in, so the runtime below must
# ship a glibc at least as new as the host's. Needs the runtime installed:
#   flatpak install flathub org.freedesktop.Platform//26.08
set -euo pipefail
cd "$(dirname "$0")/.."

APP_ID=io.github.SkinEditor
RUNTIME_VERSION=26.08
WORK=target/flatpak
mkdir -p dist

if command -v cargo >/dev/null; then
  cargo build --release
else
  # No Rust toolchain on the host: build in a container instead. Its glibc
  # (Debian bookworm) is older than the runtime's, so the binary runs there.
  ENGINE="${CONTAINER_ENGINE:-podman}"
  mkdir -p target/cargo-home/registry target/cargo-home/git
  "$ENGINE" run --rm --security-opt label=disable \
    -v "$PWD":/src \
    -v "$PWD/target/cargo-home/registry":/usr/local/cargo/registry \
    -v "$PWD/target/cargo-home/git":/usr/local/cargo/git \
    -v "$PWD/target":/build \
    -e CARGO_TARGET_DIR=/build \
    -w /src docker.io/library/rust:1-bookworm cargo build --release
fi

rm -rf "$WORK"
mkdir -p "$WORK"
# Nothing is compiled inside the sandbox, so the Platform stands in for the
# SDK; that saves installing the (much larger) org.freedesktop.Sdk.
flatpak build-init "$WORK/app" "$APP_ID" \
  org.freedesktop.Platform org.freedesktop.Platform "$RUNTIME_VERSION"

install -Dm755 target/release/skin-editor "$WORK/app/files/bin/skin-editor"
install -Dm644 packaging/$APP_ID.desktop "$WORK/app/files/share/applications/$APP_ID.desktop"
install -Dm644 packaging/$APP_ID.svg "$WORK/app/files/share/icons/hicolor/scalable/apps/$APP_ID.svg"

# Files are opened and saved through the desktop portal, and the light/dark
# preference is read from the settings portal, so no home access is requested.
flatpak build-finish "$WORK/app" \
  --command=skin-editor \
  --socket=wayland --socket=fallback-x11 --share=ipc --device=dri

flatpak build-export "$WORK/repo" "$WORK/app" stable
flatpak build-bundle "$WORK/repo" dist/skin-editor.flatpak "$APP_ID" stable \
  --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo
ls -lh dist/skin-editor.flatpak
