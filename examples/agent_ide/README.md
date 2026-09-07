# agent_ide

Agent IDE demo: projects sidebar, tabbed workspace (Agent chat / real
terminal / code editor / web browser), inspector panel.

The Terminal tab is a real terminal emulator built on `libghostty-vt`
(parsing, state, key encoding) + `portable-pty` (shell process), rendered
with a custom GPUI element. See `src/terminal.rs`.

## Build requirements

`libghostty-vt-sys` compiles Ghostty from source with **Zig 0.15.2** (the
0.16 compiler is rejected by Ghostty's build system). Two env vars are
needed:

```bash
# 1. Zig 0.15.2 first on PATH (0.16 from Homebrew will NOT work)
export PATH="$HOME/tools/zig-aarch64-macos-0.15.2:$PATH"

# 2. A patched Ghostty source dir. The stock mirror URLs fail behind a
#    proxy (zig does not honor proxy env vars), so build.zig.zon points its
#    deps at local tarballs instead.
export GHOSTTY_SOURCE_DIR="$HOME/tools/ghostty-src-patched"
```

Then:

```bash
cargo run -p agent_ide
```

Recreating the patched source dir from scratch:

```bash
# clone ghostty at the commit pinned by libghostty-vt-sys (see its build.rs)
git clone https://github.com/ghostty-org/ghostty /tmp/ghostty-src
cd /tmp/ghostty-src
# rewrite https://deps.files.ghostty.org/... tarball URLs to file:// paths,
# download each with curl (proxy-aware), then:
zig build --fetch   # seeds the global zig package cache
```

## Known limitations

- No IME (Chinese input lands in other panes, not the terminal)
- Full-screen TUI apps (vim/htop) render but mouse reporting is not wired
- Static cursor (no blink); requires a Nerd Font to display powerline glyphs
  from prompts like powerlevel10k
