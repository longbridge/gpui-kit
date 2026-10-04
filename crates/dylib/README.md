# gpui-kit-dylib

Forces dynamic linking of the **GPUI Kit** engine for significantly faster incremental build times.

## How it works

In large Rust applications using GUI engines (such as GPUI, WGPU, and Cosmic-Text), the linker must process hundreds of static libraries (`.rlib` files) on every incremental code change, causing sluggish linking times during rapid iteration.

`gpui-kit-dylib` compiles the entire engine and its dependencies into a single shared dynamic library (`.so` on Linux, `.dylib` on macOS, `.dll` on Windows). On subsequent compilations, the linker only needs to link against the pre-compiled shared library rather than re-linking all static libraries.

## Usage

In your application's `Cargo.toml`:

```toml
[dependencies]
gpui-kit = "0.7.0"

[features]
dev = ["gpui-kit/dynlib"]
```

Run in development mode:

```bash
cargo run --features dev
```

For production/release builds, simply omit the `dev` (or `dynlib`) feature:

```bash
cargo build --release
```

## Optional layers

Dynamic linking preserves Kit's feature selection. With `default-features = false`,
only GPUI and Base are included; enabling Kit's `component` or `assets` feature
also includes that layer in the shared library.

## Running and distributing binaries

Use `cargo run` or `cargo test` during development. Cargo supplies the
[dynamic library search path](https://doc.rust-lang.org/cargo/reference/environment-variables.html#dynamic-library-paths)
for processes it launches. Running the executable directly is different: the
loader must also locate the Kit dynamic library and the matching Rust toolchain's
shared libraries. Copying only the executable is not sufficient.

Cargo does **not** enable RPATH by default. Its
[`rpath` profile setting](https://doc.rust-lang.org/cargo/reference/profiles.html#rpath)
is opt-in on supported platforms, and is not a portable packaging solution.
Keep `dynlib` disabled for production builds unless you deliberately
package all required shared libraries. `--release` alone does not disable features.

## Platform considerations

- **Linux and macOS**: Prefer Cargo-managed execution during development. Direct
  execution needs a suitable loader search path or an explicitly configured RPATH.
- **Windows**: Enable optimized development dependencies as described below.
  Unoptimized builds can exceed the DLL import-library member limit (MSVC
  `LNK1189`). If your dependency graph still exceeds it, disable `dynlib`.
- **WebAssembly**: The dynamic dependency is excluded on Wasm targets.

### Windows development profile

In your application's root `Cargo.toml`, optimize dependencies while leaving
application code in the normal development profile:

```toml
[profile.dev.package."*"]
opt-level = 3
```

Optimization reduces exported generic instances and takes longer on the first
build. Cargo's wildcard excludes workspace members. If Kit is in your workspace,
optimize its framework packages explicitly too. This repository's Windows smoke
check uses the matching configuration:

```sh
cargo run -p dynlib --features dynlib --config script/dynamic-linking-windows.toml
```

See Cargo's [profile overrides and generics](https://doc.rust-lang.org/cargo/reference/profiles.html#overrides-and-generics).

## Measuring iteration time

Measure your application on the same machine, toolchain, profile and linker.
Use separate target directories for static and dynamic builds, warm each with
`cargo build`, then make the same small application-source edit before each timed
rebuild. Repeat and compare medians. Do not use a no-op build or `cargo check` as
a linking benchmark; neither measures an application relink. The first dynamic
build can be slower, and speedups depend on the application's dependency graph.
