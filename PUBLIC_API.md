# Public API

## Constructor

```rust
__constructor(
    proposal: String,
    options: Vec<String>,
    eligible_voters: Vec<(Address, i128)>,
)
```

Option indexes are zero-based and follow constructor order. The ordered voter list must exactly match the canonical snapshot committed to this repository.

## Methods

```rust
vote(voter: Address, option: u32) -> VoteRecord
get_config() -> Config
get_allocation(holder: Address) -> i128
get_vote(holder: Address) -> Option<VoteRecord>
get_results() -> VoteResults
get_voter_count(option: u32) -> u32
get_voters(option: u32, start: u32, limit: u32) -> Vec<Address>
extend_state()
```

`get_allocation` returns zero for an ineligible address. `get_voters` returns an empty vector when `start` is at or beyond the list length; `limit` must be 1–100.

## Return types

```rust
struct Config {
    proposal: String,
    options: Vec<String>,
    snapshot_sha256: BytesN<32>,
    snapshot_allocation_digest: BytesN<32>,
    total_eligible_shares: i128,
    eligible_holder_count: u32,
}

struct VoteRecord {
    option: u32,
    shares: i128,
}

struct OptionResult {
    option: u32,
    label: String,
    shares: i128,
    percent_of_cast_7dp: i128,
    percent_of_eligible_7dp: i128,
    voter_count: u32,
}

struct VoteResults {
    total_eligible_shares: i128,
    total_voted_shares: i128,
    participation_percent_7dp: i128,
    total_voters: u32,
    options: Vec<OptionResult>,
}
```

## Contract errors

| Code | Name | Meaning |
| ---: | --- | --- |
| 1400 | `InvalidProposal` | Empty or oversized proposal |
| 1401 | `InvalidOptionCount` | Fewer than 2 or more than 16 choices |
| 1402 | `InvalidOption` | Empty or oversized choice label |
| 1403 | `DuplicateOption` | Repeated choice label |
| 1404 | `InvalidEligibleHolderCount` | Snapshot does not contain exactly 434 positive entries |
| 1405 | `InvalidAllocation` | Non-positive voting weight |
| 1406 | `DuplicateEligibleHolder` | Address occurs more than once |
| 1407 | `IneligibleVoter` | Address has no positive snapshot allocation |
| 1408 | `AlreadyVoted` | Address already cast its one vote |
| 1409 | `InvalidOptionIndex` | Choice index is out of bounds |
| 1410 | `InvalidPage` | Voter page limit is zero or over 100 |
| 1411 | `Overflow` | Checked arithmetic failed or an invariant bound was exceeded |
| 1412 | `InvalidSnapshot` | Total or allocation digest differs from the canonical snapshot |
