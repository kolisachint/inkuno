#!/usr/bin/env bash
# Usage: scripts/visual-check.sh [--no-build] [fixture-name ...]
#
# Runs the real Inkuno app on a virtual X screen, opens one fixture note per Markdown component
# (tests/visual/fixtures/*.md) and screenshots it, so a human or an agent can look at every
# component after a change to the editor's layout, paint or Markdown classification.
# Output: target/visual/<timestamp>/ with one PNG per page of each note, and an index.html.
#
# Linux only (Xvfb, xdotool, ImageMagick's import). Rendering is software Vulkan (lavapipe), so no
# GPU is needed. The window draws nothing until it gets an input event: the script nudges the
# mouse after every step for that reason.
#
# The app runs with a temporary HOME and XDG_CONFIG_HOME: the user's configuration and vault are
# never read or written. The fixtures are copied into a fresh vault at ~/Documents/Notes.
set -euo pipefail
cd "$(dirname "$0")/.."
root=$PWD

usage() { sed -n '2,/^set -euo/p' "$0" | sed '$d; s/^# \{0,1\}//'; }

if [ "$(uname -s)" != Linux ]; then
  echo "scripts/visual-check.sh drives the app on Xvfb, which exists only on Linux."
  echo "On Linux, run it there:  scripts/visual-check.sh [--no-build] [fixture-name ...]"
  exit 0
fi

build=1
names=()
for arg in "$@"; do
  case $arg in
    --no-build) build=0 ;;
    -h|--help) usage; exit 0 ;;
    -*) echo "unknown option: $arg" >&2; usage >&2; exit 2 ;;
    *) names+=("${arg%.md}") ;;
  esac
done

missing=()
for tool in Xvfb xdotool import; do
  command -v "$tool" >/dev/null 2>&1 || missing+=("$tool")
done
if [ ${#missing[@]} -gt 0 ]; then
  echo "missing tools: ${missing[*]}" >&2
  echo "install them with:" >&2
  echo "  sudo apt-get install -y xvfb xdotool imagemagick mesa-vulkan-drivers libxkbcommon-dev libxkbcommon-x11-dev" >&2
  exit 1
fi

# The fixtures to show: the named ones, or all of them, in name order.
fixtures_dir=$root/tests/visual/fixtures
fixtures=()
if [ ${#names[@]} -eq 0 ]; then
  for f in "$fixtures_dir"/*.md; do
    fixtures+=("$(basename "$f" .md)")
  done
else
  for n in "${names[@]}"; do
    [ -f "$fixtures_dir/$n.md" ] || { echo "no fixture named $n (see $fixtures_dir)" >&2; exit 1; }
    fixtures+=("$n")
  done
fi

if [ "$build" -eq 1 ]; then
  cargo build -p inkuno
fi
binary=${CARGO_TARGET_DIR:-$root/target}/debug/inkuno
[ -x "$binary" ] || { echo "no binary at $binary (drop --no-build or build first)" >&2; exit 1; }

out=${CARGO_TARGET_DIR:-$root/target}/visual/$(date +%Y%m%d-%H%M%S)
mkdir -p "$out"

# A temporary home: the real configuration and vault are never touched.
tmp=$(mktemp -d)
app_pid=""
xvfb_pid=""
cleanup() {
  [ -n "$app_pid" ] && kill "$app_pid" 2>/dev/null || true
  [ -n "$xvfb_pid" ] && kill "$xvfb_pid" 2>/dev/null || true
  wait 2>/dev/null || true
  rm -rf "$tmp"
}
trap cleanup EXIT

export HOME=$tmp/home
export XDG_CONFIG_HOME=$tmp/.config
export XDG_RUNTIME_DIR=$tmp/run
mkdir -p "$HOME" "$XDG_CONFIG_HOME" "$XDG_RUNTIME_DIR" "$HOME/Documents/Notes"
chmod 700 "$XDG_RUNTIME_DIR"
cp "$fixtures_dir"/*.md "$HOME/Documents/Notes/"
# The pictures the notes show, in a folder of their own (media/).
cp -r "$fixtures_dir"/media "$HOME/Documents/Notes/"

# The first free display number, from 99 up.
n=99
while [ -e "/tmp/.X$n-lock" ] || [ -S "/tmp/.X11-unix/X$n" ]; do n=$((n + 1)); done
export DISPLAY=:$n
Xvfb "$DISPLAY" -screen 0 1400x1000x24 -nolisten tcp >"$out/xvfb.log" 2>&1 &
xvfb_pid=$!
for _ in $(seq 100); do
  xdotool getdisplaygeometry >/dev/null 2>&1 && break
  sleep 0.2
done

# Software Vulkan when the lavapipe driver is installed.
if [ -f /usr/share/vulkan/icd.d/lvp_icd.json ]; then
  export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json
fi
"$binary" >"$out/app.log" 2>&1 &
app_pid=$!

# The window appears a moment after the process starts.
wid=""
for _ in $(seq 150); do
  wid=$(xdotool search --name Inkuno 2>/dev/null | tail -n 1 || true)
  [ -n "$wid" ] && break
  if ! kill -0 "$app_pid" 2>/dev/null; then
    echo "the app exited before its window appeared; log: $out/app.log" >&2
    tail -n 20 "$out/app.log" >&2
    exit 1
  fi
  sleep 0.2
done
if [ -z "$wid" ]; then
  echo "no Inkuno window after 30 s; log: $out/app.log" >&2
  exit 1
fi

# An input event makes the window draw.
nudge() {
  xdotool mousemove 500 400
  sleep 0.5
  xdotool mousemove 600 300
  sleep 0.5
}

nudge
xdotool windowsize "$wid" 1300 940 windowmove "$wid" 50 20
sleep 1
nudge

# First run: the welcome screen. Its "Start in ~/Documents/Notes" button is centred at about
# (731, 505) on the 1300x940 window placed at (50, 20). Hover first: GPUI sets the hover state on
# a move, and a click alone can miss.
xdotool mousemove 731 505
sleep 0.5
xdotool click 1
sleep 3
nudge

pages=()
capture() {
  local name=$1 page=1 file prev=""
  xdotool key ctrl+p
  sleep 1
  xdotool type --delay 30 -- "$name"
  sleep 1
  xdotool key Return
  sleep 2
  # The line revealed at load keeps its source: a keyboard move does not hide it again, so the
  # first list item would show raw Markdown. A click on the H1 (a tall title line, hit with or
  # without a tab bar) makes the H1 the revealed line instead, and the lines below render.
  xdotool key ctrl+Home
  xdotool mousemove 420 92
  sleep 0.5
  xdotool click 1
  sleep 1
  xdotool mousemove 1250 900
  sleep 1
  while :; do
    file="$out/$name.png"
    [ "$page" -gt 1 ] && file="$out/$name-$page.png"
    xdotool mousemove 1250 900
    sleep 0.5
    import -window root "$file"
    # A page that looks like the one before it is past the end of the note.
    if [ -n "$prev" ] && cmp -s "$prev" "$file"; then
      rm -f "$file"
      break
    fi
    pages+=("$(basename "$file")")
    if [ "$page" -ge 3 ]; then break; fi
    prev=$file
    xdotool key Page_Down
    sleep 1
    page=$((page + 1))
  done
}

for name in "${fixtures[@]}"; do
  echo "capturing $name"
  pages=()
  capture "$name"
  printf '%s\n' "${pages[@]}" >>"$out/.pages"
done

# An index to review the screenshots in a browser.
{
  printf '<!doctype html>\n<meta charset="utf-8">\n<title>Inkuno visual check</title>\n'
  printf '<style>body{font-family:sans-serif;margin:24px;color:#222}img{max-width:100%%;border:1px solid #bbb;display:block;margin-bottom:24px}h2{margin:36px 0 8px}</style>\n'
  printf '<h1>Inkuno visual check</h1>\n<p>%s</p>\n' "$(date '+%Y-%m-%d %H:%M:%S')"
  while IFS= read -r png; do
    [ -n "$png" ] || continue
    label=${png%.png}
    printf '<h2>%s</h2>\n<img src="%s" alt="%s">\n' "$label" "$png" "$label"
  done <"$out/.pages"
} >"$out/index.html"
rm -f "$out/.pages"

echo
echo "screenshots: $out"
ls "$out"/*.png
echo "review: $out/index.html"
