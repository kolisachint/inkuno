#!/usr/bin/env bash
# Runs the InkunoUITests walk on the iPhone 17 Pro simulator and copies its screenshots to one folder.
#
# Usage: ios/InkunoUITests/export-screenshots.sh [output-dir]     (default: /tmp/inkuno-ios/shots)
#
# The result bundle is kept at ios/build/InkunoUITests.xcresult. The screenshots are named after the
# attachments in the walk: 00-onboarding-light.png, 01-home-light.png, ... The export runs even when
# the tests fail, so the screens that were reached can be looked at; the exit status is the test's.
set -euo pipefail

ios_dir="$(cd "$(dirname "$0")/.." && pwd)"
out_dir="${1:-/tmp/inkuno-ios/shots}"
bundle="$ios_dir/build/InkunoUITests.xcresult"
export DEVELOPER_DIR="${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}"

cd "$ios_dir"
rm -rf "$bundle"
mkdir -p "$ios_dir/build"

status=0
xcodegen generate >/dev/null
xcodebuild -project Inkuno.xcodeproj -scheme Inkuno -derivedDataPath DerivedData \
    -destination 'platform=iOS Simulator,name=iPhone 17 Pro' \
    -only-testing:InkunoUITests -resultBundlePath "$bundle" \
    CODE_SIGNING_ALLOWED=NO test || status=$?

export_dir="$(mktemp -d)"
xcrun xcresulttool export attachments --path "$bundle" --output-path "$export_dir" >/dev/null

python3 - "$export_dir" "$out_dir" <<'PY'
import json, os, re, shutil, sys

src, dst = sys.argv[1], sys.argv[2]
os.makedirs(dst, exist_ok=True)
with open(os.path.join(src, "manifest.json")) as handle:
    manifest = json.load(handle)

count = 0
for test in manifest:
    for attachment in test.get("attachments", []):
        exported = attachment.get("exportedFileName")
        suggested = attachment.get("suggestedHumanReadableName", "")
        if not exported or not suggested:
            continue
        # Xcode adds "_<n>_<UUID>" before the extension: drop it so the names are the ones the walk gave.
        name = re.sub(r"_\d+_[0-9A-Fa-f-]{36}(?=\.[A-Za-z]+$)", "", suggested)
        shutil.copyfile(os.path.join(src, exported), os.path.join(dst, name))
        count += 1
        print(name)
print(f"{count} screenshots in {dst}")
PY

rm -rf "$export_dir"
exit "$status"
