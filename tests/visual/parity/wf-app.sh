#!/usr/bin/env bash
# Drives the real Inkuno app (built binary, never cargo) on Xvfb through workflows W1-W8.
#
# Usage:
#   wf-app.sh start              start Xvfb + app with a seeded temp vault, click welcome
#   wf-app.sh stop               kill app + Xvfb, remove temp home
#   wf-app.sh key <keys...>      xdotool key (e.g. ctrl+n), then sleep and nudge
#   wf-app.sh type <text>        xdotool type
#   wf-app.sh click <x> <y>      click at window-relative (x,y), hover first
#   wf-app.sh dclick <x> <y>     double click at window-relative (x,y)
#   wf-app.sh shot <name>        screenshot the window to wf-app/<name>.png (+ 50% copy in wf-small/)
#   wf-app.sh W<n>               run a whole scenario (only used for final regression, see below)
#
# Screens are window-relative: the window sits at (50,20) and is 1300x940.
# Coordinates in this file come from the screenshots; the header of each scenario block says which.
set -u

root=$(cd "$(dirname "$0")/../../.." && pwd)
S=${WORK:-$root/target/visual}
OUT=$S/wf-app
SMALL=$S/wf-small
BIN=${BIN:-$root/target/debug/inkuno}
FIX=$root/tests/visual/fixtures/mixed.md
WX=50; WY=20; WW=1300; WH=940
STATE=$OUT/.state
mkdir -p "$OUT" "$SMALL"

log() { printf '%s\n' "$*" >>"$OUT/steps.txt"; }

load_state() { [ -f "$STATE" ] && . "$STATE"; }

nudge() {
  xdotool mousemove 500 400; sleep 0.4
  xdotool mousemove 600 300; sleep 0.4
}

start() {
  load_state
  if [ -n "${APP_PID:-}" ] && kill -0 "$APP_PID" 2>/dev/null; then echo "already running"; return 0; fi
  local tmp n
  tmp=$(mktemp -d "$S/wf-home.XXXX")
  mkdir -p "$tmp/home/Documents/Notes" "$tmp/.config" "$tmp/run"
  chmod 700 "$tmp/run"
  cp "$FIX" "$tmp/home/Documents/Notes/week-plan.md"
  cat >"$tmp/home/Documents/Notes/roadmap.md" <<'EOF'
# Roadmap

## Q1 goals

- Ship the editor preview
- Stabilise sync

## Later

Reading and the calendar view.
EOF
  cat >"$tmp/home/Documents/Notes/reading-list.md" <<'EOF'
# Reading list

- Designing Data-Intensive Applications
- A Pattern Language

## Notes

Read one chapter per week.
EOF
  cat >"$tmp/home/Documents/Notes/ideas.md" <<'EOF'
# Ideas

## Small things

- A better empty state
- Keyboard-first sorting

## Big things

Offline mode for the tree.
EOF
  mkdir -p "$tmp/home/Documents/Notes/Projects"
  printf '# Alpha\n\nFirst project note.\n' >"$tmp/home/Documents/Notes/Projects/alpha.md"
  printf '# Beta\n\nSecond project note.\n' >"$tmp/home/Documents/Notes/Projects/beta.md"

  n=120
  while [ -e "/tmp/.X$n-lock" ] || [ -S "/tmp/.X11-unix/X$n" ]; do n=$((n + 1)); done
  export HOME=$tmp/home XDG_CONFIG_HOME=$tmp/.config XDG_RUNTIME_DIR=$tmp/run
  export DISPLAY=:$n
  [ -f /usr/share/vulkan/icd.d/lvp_icd.json ] && export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json
  setsid Xvfb "$DISPLAY" -screen 0 1400x1000x24 -nolisten tcp >"$OUT/xvfb.log" 2>&1 &
  local xpid=$!
  for _ in $(seq 100); do xdotool getdisplaygeometry >/dev/null 2>&1 && break; sleep 0.2; done
  setsid "$BIN" >>"$OUT/app.log" 2>&1 &
  local apid=$!
  local wid=""
  for _ in $(seq 150); do
    wid=$(xdotool search --name Inkuno 2>/dev/null | tail -n 1 || true)
    [ -n "$wid" ] && break
    kill -0 "$apid" 2>/dev/null || { echo "app exited early; see $OUT/app.log"; return 1; }
    sleep 0.2
  done
  [ -n "$wid" ] || { echo "no window"; return 1; }
  nudge
  xdotool windowsize "$wid" $WW $WH windowmove "$wid" $WX $WY
  sleep 1; nudge
  # Welcome screen: "Start in ~/Documents/Notes" (absolute 731,505 as in scripts/visual-check.sh)
  xdotool mousemove 731 505; sleep 0.5; xdotool click 1; sleep 3; nudge
  cat >"$STATE" <<EOF
DISPLAY=$DISPLAY
HOME=$HOME
XDG_CONFIG_HOME=$XDG_CONFIG_HOME
XDG_RUNTIME_DIR=$XDG_RUNTIME_DIR
VK_ICD_FILENAMES=${VK_ICD_FILENAMES:-}
APP_PID=$apid
XVFB_PID=$xpid
WID=$wid
TMP=$tmp
EOF
  log "START display=$DISPLAY wid=$wid tmp=$tmp"
  echo "started on $DISPLAY wid=$wid"
}

env_for_app() {
  load_state
  export DISPLAY HOME XDG_CONFIG_HOME XDG_RUNTIME_DIR
  [ -n "${VK_ICD_FILENAMES:-}" ] && export VK_ICD_FILENAMES
}

stop() {
  load_state
  [ -n "${APP_PID:-}" ] && kill "$APP_PID" 2>/dev/null
  sleep 1
  [ -n "${XVFB_PID:-}" ] && kill "$XVFB_PID" 2>/dev/null
  wait 2>/dev/null
  [ -n "${TMP:-}" ] && rm -rf "$TMP"
  rm -f "$STATE"
  log "STOP"
  echo stopped
}

key() { env_for_app; xdotool key -- "$@"; sleep 1; log "key $*"; nudge; }
type_() { env_for_app; xdotool type --delay 40 -- "$1"; sleep 1; log "type $1"; nudge; }
click() { env_for_app; xdotool mousemove $((WX + $1)) $((WY + $2)); sleep 0.5; xdotool click 1; sleep 1; log "click $1,$2"; nudge; }
dclick() { env_for_app; xdotool mousemove $((WX + $1)) $((WY + $2)); sleep 0.5; xdotool click --repeat 2 --delay 120 1; sleep 1; log "dclick $1,$2"; nudge; }
rclick() { env_for_app; xdotool mousemove $((WX + $1)) $((WY + $2)); sleep 0.5; xdotool click 3; sleep 1; log "rclick $1,$2"; nudge; }
hover() { env_for_app; xdotool mousemove $((WX + $1)) $((WY + $2)); sleep 1; log "hover $1,$2"; }
shot() {
  local name=$1
  env_for_app
  nudge
  sleep 1
  local f=$OUT/$name.png
  import -window root -crop ${WW}x${WH}+${WX}+${WY} +repage "$f"
  convert "$f" -resize 50% "$SMALL/$name.png"
  log "shot $name"
  echo "$f"
}

cmd=${1:-}
shift || true
case $cmd in
  start) start ;;
  stop) stop ;;
  key) key "$@" ;;
  type) type_ "$1" ;;
  click) click "$1" "$2" ;;
  dclick) dclick "$1" "$2" ;;
  rclick) rclick "$1" "$2" ;;
  hover) hover "$1" "$2" ;;
  shot) shot "$1" ;;
  *) echo "unknown command: $cmd" >&2; exit 2 ;;
esac
