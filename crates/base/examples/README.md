# gpui-base examples

`components.rs` is the single Cargo example entrypoint for every documented `gpui-base`
component. It selects one component from the shared `showcase` implementation, so native and
WebAssembly previews exercise the same Rust code without producing one binary per component.

Run an individual component natively:

```bash
cargo run -p gpui-base-examples -- button
cargo run -p gpui-base-examples -- alert-dialog
cargo run -p gpui-base-examples -- virtual-list
```

Run without a component slug to show the overview:

```bash
cargo run -p gpui-base-examples
```

Motion has a separate example because it demonstrates continuous behavior rather than a component catalog entry. It contains focused pages for transitions, springs, keyframes, presence, and stagger:

```bash
cargo run -p gpui-base-examples --bin motion
```

The website builds `examples/wasm`, which imports the same `showcase/mod.rs` and selects the
component using the `?component=<slug>` query parameter.

## dock

`dock` is a showcase component like the rest, but a larger one: a dockable workspace — nested
splits, tab groups, and a bottom dock — built on `gpui-base` alone. Because the base dock draws
nothing, `showcase/components/dock.rs` implements the `DockAreaRenderer`, `TabGroupRenderer`, and
`TilesRenderer` traits itself, which is what makes it worth reading: it is the smallest complete
skin over the dock's renderer seam.

```bash
cargo run -p gpui-base dock
```
