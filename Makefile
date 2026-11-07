SHELL := bash
.SHELLFLAGS := -euo pipefail -c
.DEFAULT_GOAL := help

.PHONY: help test check mutants install web web-build

help: ## list the targets
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk -F ':.*## ' '{printf "  %-10s %s\n", $$1, $$2}'

test: ## every test, the command line's included
	cargo test

check: ## formatting, lints, tests and no uncovered line but the command line's and the page's wiring, as before a commit
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo clippy -p trixelate-web --target wasm32-unknown-unknown -- -D warnings
	cargo llvm-cov --workspace --exclude trixelate-web --ignore-filename-regex 'trixelate-cli/src/main\.rs' --fail-uncovered-lines 0 --summary-only

mutants: ## whether the tests bite, at the end of a slice
	cargo mutants

install: ## the command line, on this machine
	cargo install --path crates/trixelate-cli

web: ## the page, served at http://127.0.0.1:8080 and rebuilt on every change
	cd crates/trixelate-web && trunk serve

web-build: ## the page, built for any static host in crates/trixelate-web/dist
	cd crates/trixelate-web && trunk build --release
