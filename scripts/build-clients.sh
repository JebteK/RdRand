#!/usr/bin/env bash
# Build the four rdrand.com clients into dist/ (the same artifacts that are
# published at https://rdrand.com/Home/Downloads):
#
#   dist/jebtek-rdrand-2.0.0.tgz      Node.js   (npm install ./dist/jebtek-rdrand-2.0.0.tgz)
#   dist/rdrand-kotlin-2.0.0.jar      Kotlin/JVM
#   dist/JebteK.RdRand.2.0.0.nupkg    .NET
#   dist/rdrand-rust-2.0.0.zip        Rust (crate source)
#
# Also writes dist/client-tests/: self-tests for real-hardware runs on Linux
# x64 and arm64 (see scripts/test-clients-remote.sh).
#
# Needs: macOS host with the scripts/build-native.sh toolchain, node/npm,
# Docker (Gradle and .NET SDK images).
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT=$PWD
OUT=$ROOT/dist
TESTS=$ROOT/dist/client-tests
VERSION=2.0.0
GRADLE_IMAGE=gradle:8.10.2-jdk17
DOTNET_IMAGE=mcr.microsoft.com/dotnet/sdk:8.0

rm -rf "$OUT" "$TESTS"
mkdir -p "$OUT" "$TESTS"

echo "### native library"
scripts/build-native.sh
NATIVE=$ROOT/clients/native/dist

echo "### Node.js"
cd "$ROOT/clients/node"
rm -rf prebuilds && cp -R "$NATIVE" prebuilds && cp ../LICENSE .
npm pack --silent --pack-destination "$OUT" >/dev/null
cp -R test "$TESTS/node-test"

echo "### Kotlin"
cd "$ROOT/clients/kotlin"
RES=src/main/resources/com/jebtek/rdrand/native
rm -rf "$RES" && mkdir -p "$RES" && cp -R "$NATIVE"/* "$RES"/
mkdir -p src/main/resources/META-INF && cp ../LICENSE src/main/resources/META-INF/LICENSE
docker run --rm -v "$PWD":/work -w /work -v rdrand-gradle-cache:/home/gradle/.gradle \
  "$GRADLE_IMAGE" gradle --no-daemon -q clean jar selfTestJar
cp "build/libs/rdrand-kotlin-$VERSION.jar" "$OUT/"
cp "build/libs/rdrand-kotlin-selftest-$VERSION.jar" "$TESTS/"

echo "### .NET"
cd "$ROOT/clients/dotnet"
rm -rf native out selftest-out && cp -R "$NATIVE" native
docker run --rm -v "$PWD":/src -w /src -e NUGET_PACKAGES=/tmp/nuget \
  -e DOTNET_CLI_TELEMETRY_OPTOUT=1 -e DOTNET_NOLOGO=1 "$DOTNET_IMAGE" sh -ec '
    dotnet pack src/JebteK.RdRand -c Release -o out -v q -nologo
    for rid in linux-x64 linux-arm64; do
      dotnet publish selftest -c Release -r $rid --self-contained -o selftest-out/$rid -v q -nologo
    done'
cp "out/JebteK.RdRand.$VERSION.nupkg" "$OUT/"
for rid in linux-x64 linux-arm64; do cp -R "selftest-out/$rid" "$TESTS/dotnet-selftest-$rid"; done

echo "### Rust"
cd "$ROOT/clients/rust"
cargo test -q --no-default-features >/dev/null
STAGE=$(mktemp -d)
mkdir -p "$STAGE/rdrand-client-$VERSION"
cp -R Cargo.toml README.md src examples tests ../LICENSE "$STAGE/rdrand-client-$VERSION/"
(cd "$STAGE" && zip -qr "$OUT/rdrand-rust-$VERSION.zip" "rdrand-client-$VERSION")
rm -rf "$STAGE"
# Prebuilt test binaries (unit + live API tests, demo) for the hardware runs.
for t in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
  arch=${t%%-*}; mkdir -p "$TESTS/rust-$arch"
  cargo zigbuild -q --release --target "$t.2.17" --tests --examples --message-format=json 2>/dev/null |
    python3 -c '
import json, sys, shutil, os
for line in sys.stdin:
    m = json.loads(line)
    if m.get("reason") == "compiler-artifact" and m.get("executable"):
        name = m["target"]["name"] + ("-unit" if m["profile"]["test"] and m["target"]["kind"] == ["lib"] else "")
        shutil.copy(m["executable"], os.path.join(sys.argv[1], name))
' "$TESTS/rust-$arch"
done

cd "$ROOT"
echo "### artifacts"
find "$OUT" -maxdepth 1 -type f -exec ls -l {} \; | awk '{print $5, $9}'
