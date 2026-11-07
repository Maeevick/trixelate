SHELL := bash
.SHELLFLAGS := -euo pipefail -c
.DEFAULT_GOAL := help

.PHONY: help test check mutants install

help: ## list the targets
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk -F ':.*## ' '{printf "  %-10s %s\n", $$1, $$2}'

test: ## every test, the command line's included
	cargo test

check: ## formatting, lints, tests and no uncovered line but the command line's wiring, as before a commit
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo llvm-cov --workspace --ignore-filename-regex 'trixelate-cli/src/main\.rs' --fail-uncovered-lines 0 --summary-only

mutants: ## whether the tests bite, at the end of a slice
	cargo mutants

install: ## the command line, on this machine
	cargo install --path crates/trixelate-cli
