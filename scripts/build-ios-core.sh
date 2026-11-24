#!/usr/bin/env bash
# Usage: scripts/build-ios-core.sh [debug|release]   (default: debug)
#
# Builds the editor core (crates/inkuno-ffi) for the iPhone and the iPhone simulator, generates the
# Swift bindings and packs both libraries into ios/Frameworks/InkunoCore.xcframework. Outputs:
#   ios/Generated/InkunoCore.swift     the Swift API (module InkunoCore)
#   ios/Frameworks/InkunoCore.xcframework
# The C header and the modulemap (module InkunoCoreFFI) are in target/ios-core/headers. Run it on macOS
# with Xcode installed. The ios/ outputs are generated: never edit them by hand.
set -euo pipefail
cd "$(dirname "$0")/.."

profile=${1:-debug}
case "$profile" in
debug | release) ;;
*)
    echo "usage: scripts/build-ios-core.sh [debug|release]" >&2
    exit 2
    ;;
esac

# xcode-select points at the Command Line Tools on the build machine: use the full Xcode.
if [ -z "${DEVELOPER_DIR:-}" ] && [ -d /Applications/Xcode.app/Contents/Developer ]; then
    export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
fi
export IPHONEOS_DEPLOYMENT_TARGET=17.0

# An empty build_flag passes no argument (bash 3.2, the one macOS ships, rejects "${array[@]}" when empty).
build_flag=
profile_dir=debug
if [ "$profile" = release ]; then
    build_flag=--release
    profile_dir=release
fi

device=aarch64-apple-ios
simulator=aarch64-apple-ios-sim
for target in "$device" "$simulator"; do
    cargo build -p inkuno-ffi --locked --lib --target "$target" ${build_flag:+"$build_flag"}
done

# The bindings are generated from the simulator library: both carry the same metadata.
work=target/ios-core
bindings="$work/bindings"
headers="$work/headers"
rm -rf "$bindings" "$headers"
mkdir -p "$bindings" "$headers" ios/Generated
cargo run -p inkuno-ffi --features bindgen --bin uniffi-bindgen --locked -- \
    generate --language swift --out-dir "$bindings" \
    --config crates/inkuno-ffi/uniffi.toml \
    "target/$simulator/$profile_dir/libinkuno_ffi.a"

install -m644 "$bindings/InkunoCore.swift" ios/Generated/InkunoCore.swift
install -m644 "$bindings/InkunoCoreFFI.h" "$headers/InkunoCoreFFI.h"
# The modulemap is named module.modulemap so that xcodebuild and Clang find it in the headers folder.
install -m644 "$bindings/InkunoCoreFFI.modulemap" "$headers/module.modulemap"

# Only this path is replaced; the framework is rebuilt from scratch.
xcframework=ios/Frameworks/InkunoCore.xcframework
rm -rf "$xcframework"
mkdir -p ios/Frameworks
xcodebuild -create-xcframework \
    -library "target/$device/$profile_dir/libinkuno_ffi.a" -headers "$headers" \
    -library "target/$simulator/$profile_dir/libinkuno_ffi.a" -headers "$headers" \
    -output "$xcframework"

echo "built $xcframework and ios/Generated/InkunoCore.swift ($profile)"
