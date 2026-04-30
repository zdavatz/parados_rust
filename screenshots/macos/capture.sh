#!/usr/bin/env bash
# Capture eight Mac App Store screenshots of Parados (1280x800
# logical → 2560x1600 physical on a Retina display, which is the
# resolution Apple requires for Mac App Store listings).
#
# Strategy: launch parados N times with --url <game> (no button
# clicks needed, no AppleScript flakiness), resize the window with
# osascript, sleep ~1.2s for the webview to render, screencapture
# the window, kill parados, repeat.

set -euo pipefail

cd "$(dirname "$0")/../.."
BIN="./target/release/parados"
OUT="screenshots/macos"
[ -x "$BIN" ] || { echo "missing $BIN — run cargo build --release first"; exit 1; }
mkdir -p "$OUT"

# 1280x800 logical (= 2560x1600 physical on Retina) is the Apple-preferred size.
WIN_W=1280
WIN_H=800

# Pairs of (filename suffix without .png, parados://localhost/... URL).
SHOTS=(
  "01_game_list                           parados://localhost/"
  "02_kangaroo                            parados://localhost/games/kangaroo.html"
  "03_capovolto                           parados://localhost/games/capovolto.html"
  "04_divided_loyalties                   parados://localhost/games/divided_loyalties.html"
  "05_democracy                           parados://localhost/games/democracy.html"
  "06_frankenstein                        parados://localhost/games/frankenstein.html"
  "07_rainbow_blackjack                   parados://localhost/games/rainbow_blackjack.html"
  "08_makalaina                           parados://localhost/games/makalaina.html"
)

resize_and_capture() {
  local out_path="$1"
  # Resize + center via System Events.  We don't pin position because
  # the window may be off-screen on small displays; centering keeps
  # it visible regardless.
  osascript <<EOF
tell application "System Events"
  tell process "parados"
    set frontmost to true
    delay 0.1
    if (count of windows) > 0 then
      set position of window 1 to {80, 80}
      set size of window 1 to {${WIN_W}, ${WIN_H}}
    end if
  end tell
end tell
EOF
  sleep 1.2

  # Capture the parados window by id.  -o = no shadow.
  WID=$(osascript -e 'tell application "System Events" to tell process "parados" to id of window 1' 2>/dev/null || true)
  if [ -n "$WID" ]; then
    screencapture -o -l "$WID" "$out_path"
  else
    # Fallback: capture the rectangle we just placed the window at.
    # Retina logical → physical conversion is automatic.
    screencapture -o -R 80,80,${WIN_W},${WIN_H} "$out_path"
  fi
}

for entry in "${SHOTS[@]}"; do
  read -r SUFFIX URL <<<"$entry"
  echo "==> ${SUFFIX} → ${URL}"
  "$BIN" --url "$URL" --screenshot >/dev/null 2>&1 &
  PID=$!
  sleep 1.5  # let tao create the window + wry boot the webview
  resize_and_capture "${OUT}/${SUFFIX}.png"
  kill "$PID" 2>/dev/null || true
  wait "$PID" 2>/dev/null || true
  sleep 0.3
done

echo
echo "=== captured screenshots ==="
ls -la "${OUT}"/*.png 2>&1
echo
echo "=== sizes (physical pixels — should be ${WIN_W}*2 × ${WIN_H}*2 = $((WIN_W*2))x$((WIN_H*2)) on Retina) ==="
for f in "${OUT}"/*.png; do
  sips -g pixelWidth -g pixelHeight "$f" 2>/dev/null | grep -E "pixel(Width|Height)" | tr '\n' ' '
  echo " $(basename "$f")"
done
