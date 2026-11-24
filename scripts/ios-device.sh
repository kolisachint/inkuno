#!/usr/bin/env bash
# Usage: scripts/ios-device.sh [debug|release]
#
# Builds the iOS app for the connected iPhone, installs it with devicectl and
# launches it. Run it on macOS with Xcode installed. Requirements:
#   - ios/Local.xcconfig (untracked) with DEVELOPMENT_TEAM = <your Apple team id>
#   - one paired, unlocked iPhone (or IOS_DEVICE_ID=<udid or CoreDevice id>)
#   - xcodegen (brew install xcodegen)
# DEVELOPER_DIR defaults to /Applications/Xcode.app/Contents/Developer.
set -euo pipefail
cd "$(dirname "$0")/.."

config=${1:-debug}
case "$config" in
debug) xcode_config=Debug ;;
release) xcode_config=Release ;;
*)
    echo "usage: scripts/ios-device.sh [debug|release]" >&2
    exit 2
    ;;
esac
bundle_id=com.inkuno.Inkuno

export DEVELOPER_DIR="${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}"
if [ ! -d "$DEVELOPER_DIR" ]; then
    echo "ios-device: Xcode not found at $DEVELOPER_DIR (set DEVELOPER_DIR)" >&2
    exit 1
fi

if [ ! -f ios/Local.xcconfig ]; then
    echo "ios-device: ios/Local.xcconfig is missing. Create it (it is gitignored) with:" >&2
    echo "  DEVELOPMENT_TEAM = <your 10-character Apple team id>" >&2
    exit 1
fi
team=$(sed -n 's/^[[:space:]]*DEVELOPMENT_TEAM[[:space:]]*=[[:space:]]*\([A-Za-z0-9]*\).*/\1/p' ios/Local.xcconfig | tail -1 || true)
if [ -z "$team" ]; then
    echo "ios-device: ios/Local.xcconfig has no DEVELOPMENT_TEAM value" >&2
    exit 1
fi

if ! command -v xcodegen >/dev/null; then
    echo "ios-device: xcodegen is missing (brew install xcodegen)" >&2
    exit 1
fi

if [ -n "${IOS_DEVICE_ID:-}" ]; then
    device=$IOS_DEVICE_ID
else
    devices_json=$(mktemp)
    trap 'rm -f "$devices_json"' EXIT
    xcrun devicectl list devices --json-output "$devices_json" >/dev/null
    device=$(python3 -c '
import json, sys


def flatten(node, out):
    # devicectl nests fields under "properties" categories and under the older
    # flat dictionaries. Collect leaves by name, so the lookup does not depend
    # on the nesting. The first occurrence wins.
    if isinstance(node, dict):
        for key, value in node.items():
            if isinstance(value, (dict, list)):
                flatten(value, out)
            else:
                out.setdefault(key, value)
    elif isinstance(node, list):
        for item in node:
            flatten(item, out)
    return out


found = []
for device in json.load(open(sys.argv[1]))["result"]["devices"]:
    fields = flatten(device, {})
    kind = str(fields.get("deviceType", "")).lower()
    if "iphone" not in kind:
        continue
    if fields.get("reality", "physical") != "physical":
        continue
    if fields.get("pairingState", "paired") != "paired":
        continue
    if fields.get("tunnelState") == "unavailable":
        continue
    found.append((fields.get("name", "unnamed"), fields.get("identifier") or fields.get("udid")))

if not found:
    sys.exit("ios-device: no connected iPhone. Plug it in, unlock it, tap Trust, then retry.")
if len(found) > 1:
    names = ", ".join("%s (%s)" % (name, identifier) for name, identifier in found)
    sys.exit("ios-device: more than one iPhone connected: %s. Set IOS_DEVICE_ID to choose one." % names)
print(found[0][1])
' "$devices_json")
fi

echo "ios-device: building $xcode_config for device $device"
scripts/build-ios-core.sh "$config"
(cd ios && xcodegen generate)
xcodebuild -project ios/Inkuno.xcodeproj -scheme Inkuno -configuration "$xcode_config" \
    -destination 'generic/platform=iOS' -derivedDataPath ios/build \
    -allowProvisioningUpdates DEVELOPMENT_TEAM="$team" build

app="ios/build/Build/Products/$xcode_config-iphoneos/Inkuno.app"
if [ ! -d "$app" ]; then
    echo "ios-device: build did not produce $app" >&2
    exit 1
fi

echo "ios-device: installing $app"
xcrun devicectl device install app --device "$device" "$app"
echo "ios-device: launching $bundle_id"
xcrun devicectl device process launch --device "$device" --terminate-existing "$bundle_id"
