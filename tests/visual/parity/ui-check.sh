#!/usr/bin/env bash
# Usage: ui-check.sh [--theme light|dark|both] [--only NAME ...] [--list]
#
# Drives the real Inkuno desktop app on a virtual X screen (Xvfb) into many UI states, in the
# light theme (Quiet Light) and the dark theme (Quiet Dark), and screenshots each state.
#
# Env:
#   BIN     the app binary (default: <repo>/target/debug/inkuno). Never built here.
#   OUT     output root (default: the scratchpad's app/ folder); PNGs go to $OUT/<light|dark>/.
#
# Each state runs in a fresh temporary HOME and XDG_CONFIG_HOME (the app restores its layout from
# there), so no state leaks into the next. The user's configuration and vault are never touched.
# Linux only: Xvfb, xdotool, ImageMagick's import and convert. Software Vulkan (lavapipe) when
# installed. The window draws nothing until it gets input, so the mouse is nudged before a shot.
set -uo pipefail

root=$(cd "$(dirname "$0")/../../.." && pwd)
BIN=${BIN:-$root/target/debug/inkuno}
OUT=${OUT:-$root/target/visual/app}
FIXTURES=$root/tests/visual/fixtures
W=1300
H=940
WIN_X=50
WIN_Y=20

theme_arg=both
only=()
while [ $# -gt 0 ]; do
  case $1 in
    --theme) theme_arg=$2; shift ;;
    --only) shift; while [ $# -gt 0 ] && [[ $1 != --* ]]; do only+=("$1"); shift; continue; done; continue ;;
    -h|--help) sed -n '2,15p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

for tool in Xvfb xdotool import convert; do
  command -v "$tool" >/dev/null 2>&1 || { echo "missing tool: $tool" >&2; exit 1; }
done
[ -x "$BIN" ] || { echo "no binary at $BIN" >&2; exit 1; }

case $theme_arg in
  light) themes=(light) ;;
  dark) themes=(dark) ;;
  both) themes=(light dark) ;;
  *) echo "--theme takes light, dark or both" >&2; exit 2 ;;
esac

theme_value() { case $1 in light) echo "Quiet Light" ;; dark) echo "Quiet Dark" ;; esac; }

# ---------------------------------------------------------------------------------------------
# X server and app lifecycle
# ---------------------------------------------------------------------------------------------
xvfb_pid=""
app_pid=""
tmp_root=$(mktemp -d)
cleanup() {
  [ -n "$app_pid" ] && kill "$app_pid" 2>/dev/null
  [ -n "$xvfb_pid" ] && kill "$xvfb_pid" 2>/dev/null
  wait 2>/dev/null
  rm -rf "$tmp_root"
}
trap cleanup EXIT

start_xvfb() {
  local n=99
  while [ -e "/tmp/.X$n-lock" ] || [ -S "/tmp/.X11-unix/X$n" ]; do n=$((n + 1)); done
  export DISPLAY=:$n
  Xvfb "$DISPLAY" -screen 0 1400x1000x24 -nolisten tcp >"$tmp_root/xvfb.log" 2>&1 &
  xvfb_pid=$!
  for _ in $(seq 100); do
    xdotool getdisplaygeometry >/dev/null 2>&1 && return 0
    sleep 0.2
  done
  echo "Xvfb did not start; log: $tmp_root/xvfb.log" >&2
  exit 1
}

# Software Vulkan when the lavapipe driver is installed.
if [ -f /usr/share/vulkan/icd.d/lvp_icd.json ]; then
  export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json
fi

# The window: found by its title, moved and sized as the visual check does.
wid=""
find_window() {
  wid=""
  for _ in $(seq 150); do
    wid=$(xdotool search --name Inkuno 2>/dev/null | tail -n 1 || true)
    [ -n "$wid" ] && return 0
    kill -0 "$app_pid" 2>/dev/null || return 1
    sleep 0.2
  done
  return 1
}

nudge() {
  xdotool mousemove 500 400; sleep 0.4
  xdotool mousemove 600 300; sleep 0.4
}

# Hover, then click: GPUI sets the hover state on a move, and a click alone can miss.
click_at() {
  xdotool mousemove "$1" "$2"; sleep 0.4
  xdotool click 1; sleep 0.6
}

right_click_at() {
  xdotool mousemove "$1" "$2"; sleep 0.4
  xdotool click 3; sleep 0.8
}

key() { xdotool key "$@"; sleep 0.6; }
typ() { xdotool type --delay 30 -- "$1"; sleep 0.6; }

# Opens a note by name through the Find palette (Ctrl+P), as the visual check does.
open_note() {
  key ctrl+p; sleep 0.6
  typ "$1"; sleep 0.6
  key Return; sleep 1.5
}

# The note's H1 is a tall line, hit with or without a tab bar; a click there makes it the
# revealed line, so the lines below render (the same trick as the visual check).
settle_note() {
  key ctrl+Home
  xdotool mousemove 420 92; sleep 0.5
  xdotool click 1; sleep 0.8
}

# Writes the screenshot: the window's own rectangle, cropped from the root.
shot() {
  local file=$1 X Y WIDTH HEIGHT
  eval "$(xdotool getwindowgeometry --shell "$wid")"
  xdotool mousemove 1290 930; sleep 0.6
  import -window root "$tmp_root/full.png"
  mkdir -p "$(dirname "$file")"
  convert "$tmp_root/full.png" -crop "${WIDTH}x${HEIGHT}+${X}+${Y}" +repage "$file"
  # The crop must be the window size the states were designed for.
  [ "$WIDTH" = "$W" ] && [ "$HEIGHT" = "$H" ] || echo "  warning: window is ${WIDTH}x${HEIGHT}, expected ${W}x${H}" >&2
}

# Creates a fresh HOME for the state: the fixtures copied into the vault, the Projects folder,
# and the settings file with the theme. The app writes its own config at the welcome screen.
fresh_home() {
  local theme=$1 home=$tmp_root/home
  rm -rf "$tmp_root/home" "$tmp_root/config" "$tmp_root/run"
  mkdir -p "$home/Documents/Notes/Projects" "$tmp_root/config/inkuno" "$tmp_root/run"
  chmod 700 "$tmp_root/run"
  cp "$FIXTURES"/*.md "$home/Documents/Notes/"
  printf '# Week plan\n\n- [ ] Draft the release notes\n- [ ] Review the parser tests\n- [x] Book the review room\n\nSee [[Roadmap]] and [[Projects/Alpha]].\n' >"$home/Documents/Notes/week-plan.md"
  printf '# Alpha\n\nFirst project: the editor rewrite.\n\n- Milestone one\n- Milestone two\n' >"$home/Documents/Notes/Projects/Alpha.md"
  printf '# Beta\n\nSecond project: the sync layer.\n\n- Spike the protocol\n' >"$home/Documents/Notes/Projects/Beta.md"
  printf 'theme=%s\n' "$(theme_value "$theme")" >"$tmp_root/config/inkuno/settings"
  export HOME=$home
  export XDG_CONFIG_HOME=$tmp_root/config
  export XDG_RUNTIME_DIR=$tmp_root/run
}

# Starts the app, waits for its window, sizes it, and gets past the welcome screen.
start_app() {
  "$BIN" >"$tmp_root/app.log" 2>&1 &
  app_pid=$!
  if ! find_window; then
    echo "  no Inkuno window; log: $tmp_root/app.log" >&2
    tail -n 20 "$tmp_root/app.log" >&2
    return 1
  fi
  nudge
  xdotool windowsize "$wid" "$W" "$H" windowmove "$wid" "$WIN_X" "$WIN_Y"; sleep 1
  nudge
  # First run: the welcome screen, its "Start in ~/Documents/Notes" button.
  click_at 731 505
  sleep 3
  nudge
  return 0
}

stop_app() {
  [ -n "$app_pid" ] && kill "$app_pid" 2>/dev/null
  wait "$app_pid" 2>/dev/null
  app_pid=""
}

# ---------------------------------------------------------------------------------------------
# The states. Each is a function that sets the app up; the screenshot is taken after it returns.
# The name is the prototype harness's, so the two sets of PNGs can be compared side by side.
# ---------------------------------------------------------------------------------------------
STATES=(
  01-default
  02-sidebar-hidden
  03-panel-outline
  04-panel-comments
  05-panel-versions
  06-panel-assistant
  07-panel-closed
  08-palette
  09-palette-commands
  10-slash-menu
  11-selection-toolbar
  12-context-menu-file
  13-context-menu-note
  14-settings
  14a-settings-editor
  14b-settings-files
  14c-settings-shortcuts
  14d-settings-about
  15-focus-mode
  16-split
  17-titlebar-hidden
  18-tabs-multiple
  19-empty-pane
  20-toast
  21-line-revealed
  23-scratch-section
)

setup_01-default()        { open_note mixed; settle_note; key ctrl+b; }  # sidebar hidden at start: show it
setup_02-sidebar-hidden() { setup_01-default; key ctrl+b; }  # hide it again
setup_03-panel-outline()  { setup_01-default; key ctrl+alt+1; }
setup_04-panel-comments() { setup_01-default; key ctrl+alt+2; }
setup_05-panel-versions() { setup_01-default; key ctrl+alt+3; }
setup_06-panel-assistant() { setup_01-default; key ctrl+alt+4; }
setup_07-panel-closed()   { setup_01-default; key ctrl+alt+1; key ctrl+shift+o; }  # open, then close
setup_08-palette()        { setup_01-default; key ctrl+p; typ "week"; }
setup_09-palette-commands() { setup_01-default; key ctrl+p; typ ">"; }
setup_10-slash-menu()     { setup_01-default; key ctrl+End; key Return; key Return; typ "/"; }
setup_11-selection-toolbar() { setup_01-default; xdotool mousemove 300 300; sleep 0.4; xdotool click --repeat 2 --delay 80 1; sleep 0.8; }
setup_12-context-menu-file() { setup_01-default; right_click_at 120 300; }
setup_13-context-menu-note() { setup_01-default; right_click_at 500 500; }
setup_14-settings()       { setup_01-default; key ctrl+comma; sleep 1; }  # settings page nav: screen coordinates = window + (50,20)
setup_14a-settings-editor()    { setup_14-settings; click_at 328 136; }
setup_14b-settings-files()     { setup_14-settings; click_at 328 170; }
setup_14c-settings-shortcuts() { setup_14-settings; click_at 328 206; }
setup_14d-settings-about()     { setup_14-settings; click_at 328 242; }
setup_15-focus-mode()     { setup_01-default; key F9; }
setup_16-split()          { setup_01-default; key ctrl+backslash; typ "week-plan"; key Return; sleep 1; }
setup_17-titlebar-hidden() { setup_01-default; key ctrl+shift+s; }
setup_18-tabs-multiple()  { setup_01-default; open_note week-plan; open_note Alpha; }
setup_19-empty-pane()     { setup_01-default; key ctrl+backslash; key Escape; }
setup_20-toast()          { setup_01-default; key ctrl+alt+s; sleep 0.5; key Return; sleep 0.4; }
setup_21-line-revealed()  { setup_01-default; key ctrl+Home; key Down; key Down; key Down; key Down; }
setup_23-scratch-section() { setup_01-default; key ctrl+n; sleep 1; typ "Quick idea: try the outline in focus mode"; sleep 3; }  # a scratch note with text is listed in the sidebar section

run_state() {
  local theme=$1 name=$2
  local file="$OUT/$theme/$name.png"
  fresh_home "$theme"
  start_app || { echo "FAILED start  $theme $name"; stop_app; return 1; }
  "setup_$name" || true
  shot "$file"
  stop_app
  echo "ok  $theme $name -> $file"
}

start_xvfb
for theme in "${themes[@]}"; do
  if [ ${#only[@]} -gt 0 ]; then
    list=("${only[@]}")
  else
    list=("${STATES[@]}")
  fi
  for name in "${list[@]}"; do
    run_state "$theme" "$name"
  done
done
