# Blend Vote Contract

A public, snapshot-weighted Soroban voting contract for recording the non-binding intent of pre-incident BLND:USDC Comet V1 LP holders on matters related to Blend V2.1 and V3.

This repository includes the pre-incident ownership snapshot, a deterministic manifest derived from it, and a reusable single-proposal contract. The contract does not transfer or burn LP shares. Snapshot shares are voting weights only.

## Voting model

- A contract instance represents one proposal.
- The proposal text, two or more option labels, and eligible allocations are fixed in the constructor.
- Construction succeeds only when the ordered allocation list exactly matches the committed canonical snapshot digest, holder count, and total.
- An eligible address authenticates one vote for one option.
- The contract automatically uses that address's entire snapshot allocation. Partial voting and vote changes are not supported.
- Votes and voter addresses are public.
- Results report both the percentage of votes cast and the percentage of all eligible shares. Percentages are truncated to seven decimal places.
- There is no administrator, upgrade method, deadline, or binding execution mechanism.

This is a public signaling mechanism, not binding on-chain governance. Contract addresses in the snapshot remain eligible, but can vote only if their authorization design can satisfy Soroban `require_auth`.

## Deployments

- Stellar testnet: [`CBJNVTNSZL6HFCYHXY2ZFXFKRV5K2QBU3FSXBTFPNR6RUOB52OY24XW4`](https://lab.stellar.org/r/testnet/contract/CBJNVTNSZL6HFCYHXY2ZFXFKRV5K2QBU3FSXBTFPNR6RUOB52OY24XW4) — deployment details are recorded in [`deployments/testnet.json`](deployments/testnet.json).
- Stellar testnet, YieldBox Security Council emitter migration poll: [`CBZGH2OXR4UEBU64CL2NLW76G4VX4JKJ7R54ED7M3KOEWJQO653BEQHW`](https://lab.stellar.org/r/testnet/contract/CBZGH2OXR4UEBU64CL2NLW76G4VX4JKJ7R54ED7M3KOEWJQO653BEQHW) — deployment details are recorded in [`deployments/testnet-yieldbox-emitter-migration.json`](deployments/testnet-yieldbox-emitter-migration.json).

## Canonical snapshot

The exact source data is committed at [`snapshot/comet_cpal_flattened_ownership_before_41c898a1.csv`](snapshot/comet_cpal_flattened_ownership_before_41c898a1.csv).

| Property | Value |
| --- | ---: |
| CSV SHA-256 | `30fbbb6c62c8812a94cfb02f1c9d528235a28dcddfb45fa7f5535e39fd9a4cd3` |
| Canonical allocation digest | `16ef4a50d5899fb5ecc18f72f38b2cb903d009f9bf116904c465c76b52fece06` |
| CSV holders | 435 |
| Eligible holders with positive weight | 434 |
| Eligible raw shares | `146100619813817` |
| Eligible shares at 7 decimals | `14610061.9813817` |

One zero-weight CSV row is retained for provenance but excluded from constructor allocations. No positive allocation is redistributed or rounded. Run `make snapshot` to validate the CSV and regenerate [`snapshot/manifest.json`](snapshot/manifest.json).

## Contract interface

The constructor accepts:

1. `proposal: String`
2. `options: Vec<String>`
3. `eligible_voters: Vec<(Address, i128)>`

`eligible_voters` must contain the 434 positive allocations in the exact CSV order. The contract derives its allocation digest and rejects any omission, reordering, address change, or weight change. The source CSV SHA-256 and canonical allocation digest are compiled into the contract and returned by `get_config()`.

The primary methods are:

- `vote(voter, option)` — authenticates `voter` and casts all of its snapshot shares once.
- `get_config()` — proposal, choices, snapshot hash, eligible shares, and holder count.
- `get_allocation(holder)` and `get_vote(holder)` — individual snapshot and vote records.
- `get_results()` — totals, participation, and per-option weighted results.
- `get_voter_count(option)` and `get_voters(option, start, limit)` — public voter discovery with pages of at most 100 addresses.
- `extend_state()` — permissionless storage-lifetime maintenance; it must be submitted as a transaction to persist TTL changes.

See [SPECIFICATION.md](SPECIFICATION.md) for invariants and [PUBLIC_API.md](PUBLIC_API.md) for types and error codes.

## Build and test

Requirements: Rust 1.91.1, the `wasm32v1-none` target, Node.js, and Stellar CLI 27 or later.

```sh
make snapshot
make check
make test
make build
```

The optimized Wasm input is written to `target/wasm32v1-none/release/blend_vote_contract.wasm`. Before deployment, optimize it with Stellar CLI and supply all 434 `eligible_voters` from the manifest in order. The constructor independently rejects a non-canonical allocation list.

## License

GNU Affero General Public License v3.0. See [LICENSE](LICENSE).
