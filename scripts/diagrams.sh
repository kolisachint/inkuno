#!/usr/bin/env bash
# Renders each docs/diagrams/*.d2 to docs/user/src/img/<name>.svg with d2 (install it from https://d2lang.com).
set -euo pipefail

if ! command -v d2 >/dev/null 2>&1; then
  echo "diagrams.sh: d2 is not installed. Install it from https://d2lang.com, then run this script again." >&2
  exit 1
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
for src in "$root"/docs/diagrams/*.d2; do
  name="$(basename "${src%.d2}")"
  d2 --theme 0 --dark-theme 200 --pad 24 "$src" "$root/docs/user/src/img/$name.svg"
done
