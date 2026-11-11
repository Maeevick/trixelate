SHELL := bash
.SHELLFLAGS := -euo pipefail -c
.DEFAULT_GOAL := help

.PHONY: help test check mutants sync-version install web web-build lighthouse

help: ## list the targets
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk -F ':.*## ' '{printf "  %-13s %s\n", $$1, $$2}'

test: ## every test, the command line's included
	cargo test

check: ## formatting, lints, tests and no uncovered line but the command line's and the page's wiring, as before a commit
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo clippy -p trixelate-web --target wasm32-unknown-unknown -- -D warnings
	cargo llvm-cov --workspace --ignore-filename-regex '(trixelate-cli/src/main|trixelate-web/src/(app|main))\.rs' --fail-uncovered-lines 0 --summary-only

mutants: ## whether the tests bite, at the end of a slice
	cargo mutants

sync-version: ## every crate's version set to the most recent git tag's, Cargo.lock included
	@version=$$(git describe --tags --abbrev=0 | sed 's/^v//'); \
	VERSION="$$version" perl -0pi -e 's/(\[workspace\.package\]\nversion = )"[^"]*"/$$1"$$ENV{VERSION}"/; s/(trixelate = \{ path = "crates\/trixelate", version = )"[^"]*"/$$1"$$ENV{VERSION}"/' Cargo.toml; \
	cargo update --workspace --offline --quiet; \
	echo "every crate at $$version"

install: ## the command line, on this machine
	cargo install --path crates/trixelate-cli

web: ## the page, served at http://127.0.0.1:8080 and rebuilt on every change
	cd crates/trixelate-web && trunk serve --cargo-profile web

web-build: ## the page, built for any static host in crates/trixelate-web/dist
	cd crates/trixelate-web && trunk build --release --cargo-profile web-release

LIGHTHOUSE_PORT := 8090
LIGHTHOUSE_PAGE := http://127.0.0.1:$(LIGHTHOUSE_PORT)/

lighthouse: web-build ## the built page audited on a phone and a desktop, served gzipped as a host would, reports in lighthouse/
	mkdir -p lighthouse
	npx -y serve@14 crates/trixelate-web/dist -l $(LIGHTHOUSE_PORT) --no-clipboard > /dev/null 2>&1 & server=$$!; \
	trap 'kill $$server' EXIT; \
	for try in $$(seq 30); do curl -s -o /dev/null $(LIGHTHOUSE_PAGE) && break; sleep 1; done; \
	npx -y lighthouse@12 $(LIGHTHOUSE_PAGE) --quiet --chrome-flags=--headless=new \
		--output=html --output=json --output-path=lighthouse/mobile; \
	npx -y lighthouse@12 $(LIGHTHOUSE_PAGE) --quiet --chrome-flags=--headless=new --preset=desktop \
		--output=html --output=json --output-path=lighthouse/desktop
	@for form in mobile desktop; do \
		node -e 'const r = require(process.argv[1]); console.log(process.argv[2].padEnd(8), Object.values(r.categories).map(c => `$${c.title} $${Math.round(c.score * 100)}`).join(" · "))' \
			"$$PWD/lighthouse/$$form.report.json" $$form; \
	done
