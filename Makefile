dev-web:
	$(MAKE) -C crates/story-web build-wasm-dev build-web
	$(MAKE) dev\:website

dev-gallery:
	$(MAKE) -C crates/story-web dev

# `--cwd` moves only the spawned process, so the site's own package.json and
# astro.config stay inside website/ and the repository root stays Rust-only.
dev\:website:
	bun run --cwd website dev

build\:website:
	bun run --cwd website build
