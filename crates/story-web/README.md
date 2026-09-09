# GPUI Component Story Web

Web-based component gallery for GPUI Component library.

**Live Demo**: https://gpui-kit.com/gallery/

## Prerequisites

- Rust toolchain with `wasm32-unknown-unknown` target
- [Bun](https://bun.sh) (recommended) or Node.js
- wasm-bindgen-cli

### Install Dependencies

```bash
# Add WASM target
rustup target add wasm32-unknown-unknown

# Install wasm-bindgen-cli
cargo install wasm-bindgen-cli

# Install Bun (macOS/Linux)
curl -fsSL https://bun.sh/install | bash
```

## Development

### Start Development Server

```bash
make dev
```

This will:
1. Build WASM in debug mode
2. Generate JavaScript bindings
3. Start Vite dev server on http://localhost:3000

## Production Build

### Build for Production

```bash
make build-prod
```

This builds the project with:
- Release mode WASM
- Production optimizations
- Base path set to `/gallery/` for GitHub Pages

The output will be in `www/dist/` directory.

## Deployment

The gallery is automatically deployed to GitHub Pages at `/gallery/` when docs are released.

The deployment is handled by `.github/workflows/release-docs.yml` which:
1. Builds WASM in release mode
2. Builds frontend with production settings
3. Copies output to `docs/.vitepress/dist/gallery/`
4. Deploys to GitHub Pages
