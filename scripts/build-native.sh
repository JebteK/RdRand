#!/usr/bin/env bash
# Build the shared hardware-RNG library (clients/native) for every platform the
# Node.js, Kotlin and .NET clients ship, into clients/native/dist/<platform>/.
# Platform names follow Node's `${process.platform}-${process.arch}`.
#
# Needs (macOS host): rustup targets x86_64/aarch64-unknown-linux-gnu,
# x86_64-pc-windows-gnu, x86_64/aarch64-apple-darwin; zig; cargo-zigbuild.
set -euo pipefail
cd "$(dirname "$0")/../clients/native"

DIST=dist
rm -rf "$DIST"
build() { # platform rust-target build-command file
  local platform=$1 target=$2 cmd=$3 file=$4
  echo "== $platform ($target)"
  $cmd --release --locked --target "$target" -q
  mkdir -p "$DIST/$platform"
  cp "target/${target%%.*}/release/$file" "$DIST/$platform/$file"
}
# glibc 2.17 (CentOS 7 era) baseline so the .so loads on any current Linux.
build linux-x64    x86_64-unknown-linux-gnu.2.17  "cargo zigbuild" librdrand_native.so
build linux-arm64  aarch64-unknown-linux-gnu.2.17 "cargo zigbuild" librdrand_native.so
build win32-x64    x86_64-pc-windows-gnu          "cargo zigbuild" rdrand_native.dll
# A relative install name instead of this machine's build path.
export RUSTFLAGS="-C link-arg=-Wl,-install_name,@rpath/librdrand_native.dylib"
build darwin-x64   x86_64-apple-darwin            "cargo build"    librdrand_native.dylib
build darwin-arm64 aarch64-apple-darwin           "cargo build"    librdrand_native.dylib
unset RUSTFLAGS
find "$DIST" -type f -exec ls -l {} \; | awk '{print $5, $9}'
