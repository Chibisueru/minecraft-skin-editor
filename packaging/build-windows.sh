#!/usr/bin/env bash
# Cross-compiles the Windows .exe inside a container, so nothing has to be
# installed on the host apart from podman (or docker).
#
#   packaging/build-windows.sh        ->  dist/skin-editor.exe
set -euo pipefail
cd "$(dirname "$0")/.."

ENGINE="${CONTAINER_ENGINE:-podman}"
TARGET=x86_64-pc-windows-gnu
CACHE="$PWD/target/windows-cache"   # cargo registry + build output, reused between runs
mkdir -p "$CACHE/cargo-registry" "$CACHE/cargo-git" "$CACHE/target" dist

"$ENGINE" run --rm --security-opt label=disable \
  -v "$PWD":/src:ro \
  -v "$CACHE/cargo-registry":/usr/local/cargo/registry \
  -v "$CACHE/cargo-git":/usr/local/cargo/git \
  -v "$CACHE/target":/build \
  -v "$PWD/dist":/dist \
  -e CARGO_TARGET_DIR=/build \
  -e CARGO_BUILD_JOBS="${JOBS:-6}" \
  docker.io/library/rust:1-bookworm bash -euc "
    apt-get update -qq
    apt-get install -y -qq --no-install-recommends gcc-mingw-w64-x86-64 g++-mingw-w64-x86-64 >/dev/null
    rustup target add $TARGET >/dev/null
    # The source is mounted read-only; build from a copy so the host's
    # Cargo.lock and .cargo/config.toml are left alone.
    mkdir /work && cd /work
    cp -r /src/Cargo.toml /src/Cargo.lock /src/src /src/assets .
    cargo build --release --target $TARGET
    cp /build/$TARGET/release/skin-editor.exe /dist/
  "
ls -lh dist/skin-editor.exe
