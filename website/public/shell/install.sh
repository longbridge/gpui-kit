#!/usr/bin/env sh
# Installs the gpui-shell binary from a GitHub release.
#
#   curl -fsSL https://gpui-kit.com/shell/install.sh | sh
#
# GPUI_SHELL_VERSION  a release tag such as v0.7.1 (default: the latest release)
# GPUI_SHELL_HOME     gpui-shell's own directory (default: ~/.gpui-shell),
#                     holding the `apps` it runs by name and its cache. The
#                     script only creates `apps`; keep the variable exported
#                     when running gpui-shell, which reads it.
#
# The binary goes in ~/.local/bin.
set -eu

repository="longbridge/gpui-kit"
home="${GPUI_SHELL_HOME:-$HOME/.gpui-shell}"
apps="$home/apps"
bin="$HOME/.local/bin"
version="${GPUI_SHELL_VERSION:-latest}"

case "$(uname -s)" in
  Darwin) os="apple-darwin" ;;
  Linux) os="unknown-linux-gnu" ;;
  *)
    echo "gpui-shell: unsupported system $(uname -s); download a release from https://github.com/$repository/releases" >&2
    exit 1
    ;;
esac

case "$(uname -m)" in
  arm64 | aarch64) arch="aarch64" ;;
  x86_64 | amd64) arch="x86_64" ;;
  *)
    echo "gpui-shell: unsupported architecture $(uname -m)" >&2
    exit 1
    ;;
esac

target="$arch-$os"
if [ "$target" = "aarch64-unknown-linux-gnu" ]; then
  echo "gpui-shell: no prebuilt binary for $target yet; build it with \`cargo build -p gpui-shell --release\`" >&2
  exit 1
fi

asset="gpui-shell-$target.tar.gz"
if [ "$version" = "latest" ]; then
  url="https://github.com/$repository/releases/latest/download/$asset"
else
  url="https://github.com/$repository/releases/download/$version/$asset"
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

echo "Downloading $url"
curl -fsSL "$url" -o "$work/$asset"
curl -fsSL "$url.sha256" -o "$work/$asset.sha256"

(
  cd "$work"
  if command -v shasum > /dev/null 2>&1; then
    shasum -a 256 -c "$asset.sha256" > /dev/null
  else
    sha256sum -c "$asset.sha256" > /dev/null
  fi
)

tar -xzf "$work/$asset" -C "$work"
mkdir -p "$bin" "$apps"
install -m 755 "$work/gpui-shell-$target/gpui-shell" "$bin/gpui-shell"

echo "Installed $("$bin/gpui-shell" --version) to $bin/gpui-shell"
case ":$PATH:" in
  *":$bin:"*) ;;
  *) echo "Add it to your PATH: export PATH=\"$bin:\$PATH\"" ;;
esac
if [ -n "${GPUI_SHELL_HOME:-}" ]; then
  echo "Keep GPUI_SHELL_HOME=$home exported so gpui-shell finds its applications."
fi
echo "Applications go in $apps/<name>; run one with \`gpui-shell run <name>\`."
