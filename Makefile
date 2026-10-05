.PHONY: check test build snapshot verify-snapshot verify-migrations

check: verify-migrations
	cargo fmt --check
	cargo clippy --all-targets --all-features -- -D warnings

test: verify-migrations
	cargo test

build:
	cargo build --release --target wasm32v1-none

snapshot:
	node scripts/generate-snapshot.mjs
	node scripts/verify-migration-votes.mjs

verify-snapshot:
	node scripts/generate-snapshot.mjs
	node scripts/verify-migration-votes.mjs
	git diff --exit-code -- snapshot/manifest.json

verify-migrations:
	node scripts/verify-migration-votes.mjs
