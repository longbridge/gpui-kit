.PHONY: help dev build-wasm build-web build clean install

help: ## Show help information
	@echo "GPUI Component Story Web - Available commands:"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

install: ## Install all dependencies
	@echo "Checking Rust WASM target..."
	@rustup target add wasm32-unknown-unknown || true
	@echo "Checking wasm-bindgen-cli..."
	@cargo install wasm-bindgen-cli || true
	@echo "Installing frontend dependencies..."
	@cd www && bun install

build-wasm: ## Build WASM (release mode)
	@./scripts/build-wasm.sh --release

build-wasm-dev: ## Build WASM (debug mode)
	@./scripts/build-wasm.sh

build-web: ## Build frontend
	@cd www && bun install && bun run build

build-web-prod: ## Build frontend for production (GitHub Pages)
	@cd www && bun install && NODE_ENV=production bun run build

build: build-wasm build-web ## Build complete project (WASM + frontend)

build-prod: build-wasm build-web-prod ## Build complete project for production

dev: build-wasm-dev ## Start development server
	@cd www && bun install && bun run dev

preview: ## Preview production build
	@cd www && bun run preview

clean: ## Clean build artifacts
	@echo "Cleaning build artifacts..."
	@rm -rf www/dist
	@rm -rf www/src/wasm/*.js www/src/wasm/*.wasm
	@cargo clean

watch-wasm: ## Watch Rust code changes and auto-rebuild WASM
	@echo "Watching WASM changes..."
	@cargo watch -x 'build --target wasm32-unknown-unknown' -s './scripts/build-wasm.sh'
