#!/bin/sh
# Starts @NAME@ with an installed gpui-shell. Written by `gpui-shell bundle`.
#
# Finder starts this with PATH=/usr/bin:/bin:/usr/sbin:/sbin, which holds
# everything used here: curl, shasum, tar and osascript.
set -eu

# Messages reach osascript as arguments, never as script text.
runtime_notice() {
  osascript - "$1" @NAME@ >/dev/null 2>&1 <<'APPLESCRIPT' || true
on run argv
  display notification (item 1 of argv) with title (item 2 of argv)
end run
APPLESCRIPT
}

runtime_fail() {
  osascript - "$1" @NAME@ >/dev/null 2>&1 <<'APPLESCRIPT' || true
on run argv
  display alert ((item 2 of argv) & " cannot start") message (item 1 of argv) as critical
end run
APPLESCRIPT
  echo "$1" >&2
  exit 1
}

@RUNTIME_FUNCTIONS@

# Contents/MacOS/<name> beside Contents/Resources/app.
dir=$(cd "$(dirname "$0")" && pwd -P)
ensure_runtime
exec "$runtime" run --dir "$dir/../Resources/app" --packaged
