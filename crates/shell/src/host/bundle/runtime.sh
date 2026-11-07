# Finds gpui-shell, or installs the release this bundle was made with.
# Included by the launchers and the installer; the includer defines
# `runtime_notice <message>` and `runtime_fail <message>` (which must not return).

runtime_version=@RUNTIME_VERSION@
runtime_repository=longbridge/gpui-kit

# Whether version $1 is at least $2. Numeric X.Y.Z; a pre-release suffix on an
# otherwise equal version is lower, as semver orders it.
runtime_at_least() {
  awk -v have="$1" -v want="$2" 'BEGIN {
    split(have, h, "-"); split(want, w, "-")
    n = split(h[1], a, "."); m = split(w[1], b, ".")
    if (n != 3 || m != 3) exit 1
    for (i = 1; i <= 3; i++) {
      if (a[i] !~ /^[0-9]+$/) exit 1
      if (a[i] + 0 > b[i] + 0) exit 0
      if (a[i] + 0 < b[i] + 0) exit 1
    }
    # Equal numbers: a release beats any pre-release of itself.
    exit (h[2] != "" && w[2] == "") ? 1 : 0
  }'
}

# Whether $1 is a gpui-shell this bundle can run on.
runtime_suits() {
  [ -x "$1" ] || return 1
  candidate_version=$("$1" --version 2>/dev/null | awk '$1 == "gpui-shell" { print $2; exit }') || return 1
  [ -n "$candidate_version" ] && runtime_at_least "$candidate_version" "$runtime_version"
}

# Sets `runtime` to a suitable gpui-shell, downloading one into ~/.local/bin
# when none is installed. ~/.local/bin is where the official installer puts it.
ensure_runtime() {
  for candidate in ~/.local/bin/gpui-shell "$(command -v gpui-shell 2>/dev/null || true)" \
    /opt/homebrew/bin/gpui-shell /usr/local/bin/gpui-shell ~/.cargo/bin/gpui-shell; do
    if [ -n "$candidate" ] && runtime_suits "$candidate"; then
      runtime=$candidate
      return 0
    fi
  done
  runtime_install
}

runtime_install() {
  case "$(uname -s)" in
    Darwin) runtime_os=apple-darwin ;;
    Linux) runtime_os=unknown-linux-gnu ;;
    *) runtime_fail "gpui-shell has no release for $(uname -s)." ;;
  esac
  case "$(uname -m)" in
    arm64 | aarch64) runtime_arch=aarch64 ;;
    x86_64 | amd64) runtime_arch=x86_64 ;;
    *) runtime_fail "gpui-shell has no release for $(uname -m)." ;;
  esac
  runtime_target="$runtime_arch-$runtime_os"
  if [ "$runtime_target" = aarch64-unknown-linux-gnu ]; then
    runtime_fail "gpui-shell has no release for Linux on ARM yet; build it with \`cargo build -p gpui-shell --release\` and put it in ~/.local/bin."
  fi
  for utility in curl tar mktemp; do
    command -v "$utility" >/dev/null 2>&1 || runtime_fail "Installing gpui-shell needs $utility."
  done

  runtime_asset="gpui-shell-$runtime_target.tar.gz"
  runtime_url="https://github.com/$runtime_repository/releases/download/v$runtime_version/$runtime_asset"
  runtime_bin=~/.local/bin
  mkdir -p "$runtime_bin" || runtime_fail "Cannot create $runtime_bin."
  runtime_work=$(mktemp -d) || runtime_fail 'Cannot create a temporary directory.'
  runtime_notice "Downloading gpui-shell $runtime_version…"
  if ! runtime_fetch; then
    rm -rf "$runtime_work"
    runtime_fail "Could not install gpui-shell $runtime_version from $runtime_url."
  fi
  rm -rf "$runtime_work"
  runtime="$runtime_bin/gpui-shell"
  runtime_notice "Installed gpui-shell $runtime_version in $runtime_bin."
}

# Downloads, verifies and installs into $runtime_bin. Fails without touching
# an installed copy: the new binary replaces it only after it checks out.
# Every step is checked explicitly: `set -e` does not apply inside a function
# whose status is tested.
runtime_fetch() (
  cd "$runtime_work" || exit 1
  curl --fail --silent --show-error --location --retry 3 "$runtime_url" -o "$runtime_asset" || exit 1
  curl --fail --silent --show-error --location --retry 3 "$runtime_url.sha256" -o "$runtime_asset.sha256" || exit 1
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 -c "$runtime_asset.sha256" >/dev/null || exit 1
  elif command -v sha256sum >/dev/null 2>&1; then
    sha256sum -c "$runtime_asset.sha256" >/dev/null || exit 1
  else
    echo 'Missing a SHA-256 verifier (shasum or sha256sum).' >&2
    exit 1
  fi
  tar -xzf "$runtime_asset" || exit 1
  staged="$runtime_bin/.gpui-shell.$$"
  if ! cp "gpui-shell-$runtime_target/gpui-shell" "$staged" || ! chmod 755 "$staged" ||
    ! runtime_suits "$staged"; then
    rm -f "$staged"
    echo "The downloaded gpui-shell is not version $runtime_version or later." >&2
    exit 1
  fi
  mv -f "$staged" "$runtime_bin/gpui-shell" || { rm -f "$staged"; exit 1; }
)
