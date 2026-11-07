#!/bin/sh
# Starts @NAME@ with an installed gpui-shell. Written by `gpui-shell bundle`.
set -eu

[ "$#" -eq 0 ] || { echo 'This launcher takes no arguments.' >&2; exit 2; }

# A desktop launch has no terminal, so tell the user through a notification.
runtime_notice() {
  if [ -t 2 ]; then
    echo "$1" >&2
  elif command -v notify-send >/dev/null 2>&1; then
    notify-send -- @NAME@ "$1" 2>/dev/null || true
  fi
}

runtime_fail() {
  if [ -t 2 ]; then
    echo "$1" >&2
  elif command -v notify-send >/dev/null 2>&1; then
    notify-send --urgency=critical -- @NAME@ "$1" 2>/dev/null || true
  fi
  exit 1
}

@RUNTIME_FUNCTIONS@

# Resolved through the ~/.local/bin link to the installed directory.
dir=$(dirname "$(readlink -f "$0")")
ensure_runtime
exec "$runtime" run --dir "$dir/app" --packaged
