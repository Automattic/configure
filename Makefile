# Makefile for a8c-secrets - Rust CLI tool for managing encrypted secrets
.PHONY: help build build-release clean test lint fmt check install run dev all

# Default target
help: ## Show this help message
	@echo "Available commands:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

# Build targets
build: ## Build the project in debug mode
	cargo build

build-release: ## Build the project in release mode (optimized)
	cargo build --release

install: build-release ## Install the binary to ~/.cargo/bin
	cargo install --path .

# Development targets
dev: fmt lint test ## Run all development checks (format, lint, test)

check: ## Quick compile check without building
	cargo check

test: ## Run all tests
	cargo test

# Code quality targets
lint: ## Run clippy linter with comprehensive checks
	cargo clippy -- -D warnings

lint-fix: ## Run clippy and automatically fix issues where possible
	cargo clippy --fix --allow-no-vcs -- -D warnings

fmt: ## Format code using rustfmt
	cargo fmt

fmt-check: ## Check if code is properly formatted
	cargo fmt --check

# Comprehensive quality check
all: fmt lint test build ## Run format, lint, test, and build

# Utility targets
clean: ## Clean build artifacts
	cargo clean

run: build ## Build and run the CLI tool (use ARGS="..." for arguments)
	./target/debug/a8c-secrets $(ARGS)

run-release: build-release ## Build and run the optimized CLI tool (use ARGS="..." for arguments)
	./target/release/a8c-secrets $(ARGS)

# Documentation targets
doc: ## Generate and open documentation
	cargo doc --open

doc-private: ## Generate documentation including private items
	cargo doc --document-private-items --open

# Security and dependency checks
audit: ## Run security audit on dependencies
	cargo audit

outdated: ## Check for outdated dependencies
	cargo outdated

# Development workflow shortcuts
quick: fmt lint ## Quick development check (format + lint)
	@echo "✅ Quick checks passed!"

ci: fmt-check lint test build ## CI-like check (what CI would run)
	@echo "✅ All CI checks passed!"

# Binary size analysis
size: build-release ## Show binary size information
	@echo "Binary sizes:"
	@ls -lh target/release/a8c-secrets
	@echo "Stripped size:"
	@strip target/release/a8c-secrets
	@ls -lh target/release/a8c-secrets
