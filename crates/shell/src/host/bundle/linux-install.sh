#!/bin/sh
# Installs @NAME@ for the current user. Written by `gpui-shell bundle`.
#
#   sh install.sh                   install from @ARCHIVE@ beside this script,
#                                   or download it; also updates an installed copy
#   sh install.sh <archive or URL>  install from that archive
#   sh install.sh --uninstall       remove it; its saved data and gpui-shell stay
#   curl -fsSL <url>/install.sh | sh
#
# No sudo: everything goes under ~/.local. The application runs on the
# gpui-shell in ~/.local/bin, which is installed when no suitable one exists.
# Whatever the source, the archive must match the checksum written here.
set -eu

id=@ID@
name=@NAME@
command_name=@COMMAND@
icon_size=@ICON_SIZE@
archive_name=@ARCHIVE@
archive_sha256=@ARCHIVE_SHA256@
download_url=@DOWNLOAD_URL@
stem=@STEM@

# Tilde resolves the current account; no environment overrides.
data_home=~/.local/share
bin_dir=~/.local/bin
install_dir="$data_home/$id"
bin_link="$bin_dir/$command_name"
desktop_dir="$data_home/applications"
desktop_file="$desktop_dir/$id.desktop"
icon_dir="$data_home/icons/hicolor/${icon_size}x${icon_size}/apps"

runtime_notice() { echo "$1"; }
runtime_fail() { echo "$1" >&2; exit 1; }

@RUNTIME_FUNCTIONS@

refresh_launcher() {
  if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$desktop_dir" 2>/dev/null || true
  fi
  if [ -n "$icon_size" ] && command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q -t "$data_home/icons/hicolor" 2>/dev/null || true
  fi
}

# Only a link this script made is removed or replaced.
owns_link() {
  [ -L "$bin_link" ] && [ "$(readlink "$bin_link")" = "$install_dir/$command_name" ]
}

usage() {
  echo "Usage: sh install.sh [<archive or URL>]"
  echo "       sh install.sh --uninstall"
  echo "Without an argument it installs $archive_name from beside this script,"
  if [ -n "$download_url" ]; then
    echo "or downloads it from $download_url."
  else
    echo 'so keep the two together, or name the archive.'
  fi
}

source_arg=''
case "${1:-}" in
  '') ;;
  --uninstall)
    [ "$#" -eq 1 ] || { echo 'Unexpected arguments' >&2; exit 2; }
    if [ -L "$install_dir" ]; then
      echo "Refusing to remove the linked application: $install_dir" >&2
      exit 1
    fi
    if owns_link; then rm -f "$bin_link"; fi
    rm -f "$desktop_file"
    [ -z "$icon_size" ] || rm -f "$icon_dir/$id.png"
    rm -rf "$install_dir"
    refresh_launcher
    echo "$name is uninstalled. Its saved data and gpui-shell are kept."
    exit 0 ;;
  --help | -h) usage; exit 0 ;;
  -*) echo "Unknown option: $1" >&2; exit 2 ;;
  *) source_arg=$1 ;;
esac
[ "$#" -le 1 ] || { echo 'Unexpected arguments' >&2; exit 2; }

for utility in install mktemp readlink tar; do
  command -v "$utility" >/dev/null 2>&1 || { echo "Missing dependency: $utility" >&2; exit 1; }
done
if [ -L "$install_dir" ]; then
  echo "Refusing to replace the linked application: $install_dir" >&2
  exit 1
fi
if { [ -e "$bin_link" ] || [ -L "$bin_link" ]; } && ! owns_link; then
  echo "Refusing to replace $bin_link, which this application did not install." >&2
  exit 1
fi

# Where the archive comes from: the argument, the file beside this script, or
# the release. Piped into `sh`, $0 is the shell, so there is no "beside".
source=''
case "$source_arg" in
  http://* | https://*) source=$source_arg ;;
  '')
    case "$0" in
      *.sh)
        if [ -f "$0" ] && [ -f "$(dirname "$0")/$archive_name" ]; then
          source="$(cd "$(dirname "$0")" && pwd -P)/$archive_name"
        fi ;;
    esac
    if [ -z "$source" ] && [ -n "$download_url" ]; then
      source="${download_url%/}/$archive_name"
    fi
    if [ -z "$source" ]; then
      echo "Cannot find $archive_name." >&2
      usage >&2
      exit 1
    fi ;;
  *)
    [ -f "$source_arg" ] || { echo "No such archive: $source_arg" >&2; exit 1; }
    source=$source_arg ;;
esac

# Now rather than at the first launch, where a download has no terminal.
# Before any file changes, so a failed download leaves an installation as it was.
ensure_runtime
echo "Using $("$runtime" --version) at $runtime"

mkdir -p "$data_home" "$bin_dir" "$desktop_dir"
stage=$(mktemp -d "$data_home/.$id.XXXXXX")
trap 'rm -rf "$stage"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

case "$source" in
  http://* | https://*)
    command -v curl >/dev/null 2>&1 || { echo 'Missing dependency: curl' >&2; exit 1; }
    echo "Downloading $source"
    curl --fail --silent --show-error --location --retry 3 "$source" -o "$stage/$archive_name" ;;
  *) cp "$source" "$stage/$archive_name" ;;
esac

if command -v sha256sum >/dev/null 2>&1; then
  actual=$(sha256sum "$stage/$archive_name" | awk '{ print $1 }')
elif command -v shasum >/dev/null 2>&1; then
  actual=$(shasum -a 256 "$stage/$archive_name" | awk '{ print $1 }')
else
  echo 'Missing a SHA-256 verifier (sha256sum or shasum).' >&2
  exit 1
fi
if [ "$actual" != "$archive_sha256" ]; then
  echo "$source does not match the $archive_name this script was written for." >&2
  exit 1
fi

mkdir "$stage/unpacked"
tar -xzf "$stage/$archive_name" -C "$stage/unpacked"
here="$stage/unpacked/$stem"
if [ ! -f "$here/$command_name" ] || [ ! -f "$here/app/gpui-shell.json" ]; then
  echo "$archive_name does not hold $stem/$command_name and its app." >&2
  exit 1
fi

# Copy first, then swap: a failed copy leaves the installed version untouched.
mkdir "$stage/new"
install -m 755 "$here/$command_name" "$stage/new/$command_name"
cp -R "$here/app" "$stage/new/app"
if [ -e "$install_dir" ]; then mv "$install_dir" "$stage/previous"; fi
if ! mv "$stage/new" "$install_dir"; then
  [ ! -e "$stage/previous" ] || mv "$stage/previous" "$install_dir"
  exit 1
fi

ln -sfn "$install_dir/$command_name" "$bin_link"

icon=application-x-executable
if [ -n "$icon_size" ] && [ -f "$here/$command_name.png" ]; then
  mkdir -p "$icon_dir"
  install -m 644 "$here/$command_name.png" "$icon_dir/$id.png"
  icon=$id
fi

# Desktop Exec quoting has its own grammar; escape the account path literally.
desktop_exec=$(printf '%s' "$install_dir/$command_name" | sed 's/\\/\\\\/g; s/"/\\"/g; s/`/\\`/g; s/\$/\\$/g; s/%/%%/g')
cat > "$stage/$id.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=$name
Exec="$desktop_exec"
Icon=$icon
Terminal=false
Categories=Utility;
StartupWMClass=$id
StartupNotify=true
DESKTOP
install -m 644 "$stage/$id.desktop" "$desktop_file"
refresh_launcher

echo "$name is installed in $install_dir."
echo 'Open it from your application launcher, or run:'
echo "  $bin_link"
case ":${PATH:-}:" in
  *":$bin_dir:"*) ;;
  *) echo "Add $bin_dir to PATH to run it as \`$command_name\`." ;;
esac
