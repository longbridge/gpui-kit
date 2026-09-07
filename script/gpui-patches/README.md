# Runtime window visibility

`window-visibility.patch` adds `Window::set_visible(bool)` to the GPUI snapshot.
Tray applications can hide and show a window without destroying its view or
rebuilding its state. To focus a restored window, call `activate_window()` after
`set_visible(true)`; window managers can also apply their own focus policy.

The patch covers macOS, Windows, X11, Wayland, web, and the headless/test
implementations. On Wayland, hiding unmaps the surface and stops frame retries;
showing restores the toplevel properties and waits for a fresh configure before
presenting. Initially hidden windows also honor `WindowOptions::show` on Linux
and web.

Source: [the original implementation](https://github.com/domenkozar/zed-gpui/commit/8d58cb6c5d5bbc24361c9b3aaf2905b7956b942f),
rebased onto Zed `db10a8dd733e64682638b98232c190e8babecd5a`, with an additional
guard against frame retries after hiding during a callback. The patch also
applies to `69164008341295ad481bb11c0334a712ca8c23e3` (gpui-pre 0.3.4).

`script/bump-gpui.ts` applies the patch to the copied workspace before its source
transformations and license audit. It checks every hunk first and stops on
upstream drift. It does not modify the Zed checkout. Modified source files carry
the same redistribution notice as the publisher's existing transformations;
crate metadata and `gpui-pre.json` record the patch name.

When refreshing GPUI, resolve any patch conflict against the new platform
implementation before publishing. If Zed adds equivalent support, remove the
patch and staging hook together. Use `--force` for a release that changes only
this patch, since the scheduled publisher compares upstream Zed revisions.

To stage without publishing:

```sh
bun script/bump-gpui.ts 0.3.5 --rev db10a8dd733e64682638b98232c190e8babecd5a --stage-only
cargo test --manifest-path target/gpui-pre/workspace/Cargo.toml -p gpui-pre --features test-support --test window_visibility
```

The publisher runs the regression test before publishing (unless verification
is explicitly skipped with `--no-verify`). It covers initial visibility,
repeated hide/show, and independence between windows. Native verification should
also exercise retained view state, close-to-tray, and restoration with explicit
activation on each desktop platform.
