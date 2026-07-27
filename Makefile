# One command surface for the whole project. Run `make help`.
.DEFAULT_GOAL := help
.PHONY: help build test lint fmt fmt-check check install clean qr-demo

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
	  awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

build: ## Build the release binary
	cargo build --release

test: ## Run the full test suite (offline, deterministic)
	cargo test --workspace

test-live: ## Also run network tests against Solana devnet
	cargo test --workspace -- --ignored

lint: ## Clippy with warnings denied
	cargo clippy --workspace --all-targets -- -D warnings

fmt: ## Format the code
	cargo fmt

fmt-check: ## Verify formatting (CI)
	cargo fmt --check

check: fmt-check lint test ## Everything CI runs

install: ## Install `solpay` to ~/.cargo/bin
	cargo install --path crates/solpay --locked

clean: ## Remove build artifacts
	cargo clean
