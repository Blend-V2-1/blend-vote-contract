# Contract specification

## Purpose

Each deployment records one non-binding, multiple-choice poll weighted by the pre-incident BLND:USDC Comet V1 LP ownership snapshot. It is intended to make LP-holder intent independently observable, not to execute upgrades or control protocol contracts.

## Construction

Construction succeeds only when:

- the proposal is 1–2,048 bytes;
- there are 2–16 non-empty, unique option labels, each at most 256 bytes;
- there are exactly 434 unique eligible addresses in canonical CSV order;
- every eligible allocation is a positive `i128`;
- their sum is exactly `146100619813817`; and
- the SHA-256 allocation digest matches `16ef4a50d5899fb5ecc18f72f38b2cb903d009f9bf116904c465c76b52fece06`.

The digest is calculated over the UTF-8 domain `blend-vote-snapshot-v1`, followed by each positive CSV row's 56-byte ASCII address and signed i128 big-endian raw share count. The proposal, options, snapshot hashes, allocation map, eligible total, and eligible holder count are immutable after construction.

## Vote transition

For `vote(voter, option)` to succeed:

1. `option` must identify a configured choice.
2. `voter.require_auth()` must succeed.
3. `voter` must have a positive constructor allocation.
4. `voter` must not already have a vote record.

The transition atomically creates the vote record, adds the full immutable allocation to the option and global totals, increments voter counts, stores the address at the next index in the option's voter list, and publishes a `vote` event. Any failure rolls back the entire transition.

## Invariants

- Each address has at most one vote record.
- A vote record's weight equals the address's constructor allocation.
- `total_voted_shares` equals the sum of all option share totals.
- `total_voters` equals the sum of all option voter counts.
- Voted shares never exceed total eligible shares.
- Voter count never exceeds eligible holder count.
- Option voter lists preserve successful vote order.
- There are no external contract calls other than Soroban authorization handling.

## Result arithmetic

The integer `1_000_000_000` represents `100.0000000%`.

```text
percent_of_cast_7dp     = option_shares * 1_000_000_000 / total_voted_shares
percent_of_eligible_7dp = option_shares * 1_000_000_000 / total_eligible_shares
participation_7dp       = total_voted_shares * 1_000_000_000 / total_eligible_shares
```

Integer division truncates. When no votes have been cast, every percentage of votes cast is zero.

## Storage lifetime

Instance and persistent entries are extended when relevant contract methods execute, using a 90-day threshold and a 180-day target based on five-second ledgers. `extend_state()` touches every contract-owned entry and is permissionless. Read-only RPC simulations do not submit TTL extensions to the network, so maintenance must be submitted as a transaction. Operators should monitor and restore archived state when necessary.
