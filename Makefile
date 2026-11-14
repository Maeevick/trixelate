SHELL := bash
.SHELLFLAGS := -euo pipefail -c
.DEFAULT_GOAL := help

.PHONY: help test check mutants bump install web web-build web-smoke lighthouse

help: ## list the targets
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk -F ':.*## ' '{printf "  %-11s %s\n", $$1, $$2}'

test: ## every test, the command line's included
	cargo test

check: ## formatting, lints, tests and no uncovered line but the command line's and the page's wiring, as before a commit
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo clippy -p trixelate-web --target wasm32-unknown-unknown -- -D warnings
	cargo llvm-cov --workspace --ignore-filename-regex '(trixelate-cli/src/main|trixelate-csp/src/main|trixelate-web/src/(app|main))\.rs' --fail-uncovered-lines 0 --summary-only

mutants: ## whether the tests bite, at the end of a slice
	cargo mutants

bump: ## every crate at VERSION, committed then tagged vVERSION, never pushed: make bump VERSION=0.4.0
	@test -n "$(VERSION)" || { echo "which version? make bump VERSION=0.4.0"; exit 2; }
	@[[ "$(VERSION)" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$$ ]] || { echo "$(VERSION) is no version: three numbers, as in 0.4.0"; exit 2; }
	@! git rev-parse --quiet --verify "refs/tags/v$(VERSION)" > /dev/null || { echo "v$(VERSION) is already tagged"; exit 2; }
	@git diff --quiet && git diff --cached --quiet || { echo "commit or stash your changes first: the bump commits the version alone"; exit 2; }
	@VERSION="$(VERSION)" perl -0pi -e 's/(\[workspace\.package\]\nversion = )"[^"]*"/$$1"$$ENV{VERSION}"/; s/(trixelate = \{ path = "crates\/trixelate", version = )"[^"]*"/$$1"$$ENV{VERSION}"/' Cargo.toml
	@[ "$$(grep -c '"$(VERSION)"' Cargo.toml)" -eq 2 ] || { git checkout -- Cargo.toml; echo "Cargo.toml has moved: its workspace version or the library's requirement was not found, nothing changed"; exit 1; }
	@cargo update --workspace --offline --quiet || { git checkout -- Cargo.toml Cargo.lock; echo "cargo could not update Cargo.lock, nothing changed"; exit 1; }
	@git add Cargo.toml Cargo.lock
	@git commit --quiet -m "chore: every crate at $(VERSION)"
	@git tag -a "v$(VERSION)" -m "trixelate $(VERSION)"
	@echo "every crate at $(VERSION), tagged v$(VERSION); push both with: git push --follow-tags"

install: ## the command line, on this machine
	cargo install --path crates/trixelate-cli

web: ## the page, served at http://127.0.0.1:8080 and rebuilt on every change
	cd crates/trixelate-web && trunk serve --cargo-profile web

web-build: ## the page, built for any static host in crates/trixelate-web/dist, failing without a Content-Security-Policy that hashes its script and style
	cd crates/trixelate-web && trunk build --release --cargo-profile web-release
	@page=crates/trixelate-web/dist/index.html; \
	policy=$$(grep -n -o '<meta http-equiv="Content-Security-Policy" content="[^"]*"' $$page | head -1); \
	script=$$(grep -n -m1 '<script' $$page | cut -d: -f1); \
	[ -n "$$policy" ] && [ -n "$$script" ] && [ "$${policy%%:*}" -lt "$$script" ] \
		&& grep -Eq "script-src [^;]*'sha256-" <<< "$$policy" && grep -Eq "style-src [^;]*'sha256-" <<< "$$policy" \
		|| { echo "the built page carries no Content-Security-Policy before its first script, hashing its script and style: see crates/trixelate-csp"; exit 1; }

SMOKE_PORT := 8091
SMOKE_PAGE := http://127.0.0.1:$(SMOKE_PORT)/

web-smoke: ## the built page opened in a headless Chrome, failing unless it mounts with its policy blocking nothing: after make web-build
	@test -f crates/trixelate-web/dist/index.html || { echo "no page to open: make web-build first"; exit 2; }
	@chrome="$${CHROME:-$$(command -v google-chrome || command -v chromium || echo '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome')}"; \
	scratch=$$(mktemp -d); \
	npx -y serve@14.2.8 crates/trixelate-web/dist -l $(SMOKE_PORT) --no-clipboard > /dev/null 2>&1 & server=$$!; \
	trap 'kill $$server; rm -rf "$$scratch"' EXIT; \
	for try in $$(seq 30); do curl -s -o /dev/null $(SMOKE_PAGE) && break; sleep 1; done; \
	"$$chrome" --headless=new --disable-gpu --no-first-run $(CHROME_FLAGS) --user-data-dir="$$scratch/profile" \
		--enable-logging=stderr --v=0 --virtual-time-budget=10000 --dump-dom $(SMOKE_PAGE) \
		> "$$scratch/page.html" 2> "$$scratch/console.log" || true; \
	if grep 'Content Security Policy' "$$scratch/console.log"; then echo "the built page's policy blocks part of it"; exit 1; fi; \
	grep -q 'type="file"' "$$scratch/page.html" || { cat "$$scratch/console.log"; echo "the built page did not mount"; exit 1; }; \
	echo "the built page mounts, its policy blocking nothing"

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
