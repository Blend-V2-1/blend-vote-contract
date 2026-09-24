.PHONY: check test build snapshot verify-snapshot

check:
	cargo fmt --check
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test

build:
	cargo build --release --target wasm32v1-none

snapshot:
	node scripts/generate-snapshot.mjs

verify-snapshot:
	node scripts/generate-snapshot.mjs
	git diff --exit-code -- snapshot/manifest.json
