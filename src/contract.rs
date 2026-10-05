use crate::{
    errors::VoteError, events, storage, CANONICAL_ALLOCATION_DIGEST, CANONICAL_ELIGIBLE_HOLDERS,
    CANONICAL_SNAPSHOT_SHA256, CANONICAL_TOTAL_ELIGIBLE_SHARES, MAX_OPTIONS, MAX_OPTION_BYTES,
    MAX_PROPOSAL_BYTES, MAX_VOTER_PAGE, PERCENT_7DP_SCALE, SNAPSHOT_DIGEST_DOMAIN,
};
use soroban_sdk::{
    contract, contractimpl, contracttype, panic_with_error, Address, Bytes, BytesN, Env, Map,
    String, Vec,
};

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Config {
    pub proposal: String,
    pub options: Vec<String>,
    pub snapshot_sha256: BytesN<32>,
    pub snapshot_allocation_digest: BytesN<32>,
    pub total_eligible_shares: i128,
    pub eligible_holder_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct VoteRecord {
    pub option: u32,
    pub shares: i128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct OptionResult {
    pub option: u32,
    pub label: String,
    pub shares: i128,
    pub percent_of_cast_7dp: i128,
    pub percent_of_eligible_7dp: i128,
    pub voter_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct VoteResults {
    pub total_eligible_shares: i128,
    pub total_voted_shares: i128,
    pub participation_percent_7dp: i128,
    pub total_voters: u32,
    pub options: Vec<OptionResult>,
}

#[contract]
pub struct BlendVoteContract;

fn percent(e: &Env, numerator: i128, denominator: i128) -> i128 {
    if denominator == 0 {
        return 0;
    }
    numerator
        .checked_mul(PERCENT_7DP_SCALE)
        .and_then(|value| value.checked_div(denominator))
        .unwrap_or_else(|| panic_with_error!(e, VoteError::Overflow))
}

fn require_option(e: &Env, option: u32, count: u32) {
    if option >= count {
        panic_with_error!(e, VoteError::InvalidOptionIndex);
    }
}

fn record_vote(e: &Env, voter: Address, option: u32) -> VoteRecord {
    storage::extend_instance(e);
    let config = storage::get_config(e);
    require_option(e, option, config.options.len());
    voter.require_auth();

    let allocations = storage::get_allocations(e);
    let shares = allocations
        .get(voter.clone())
        .unwrap_or_else(|| panic_with_error!(e, VoteError::IneligibleVoter));
    let mut votes = storage::get_votes(e);
    if votes.contains_key(voter.clone()) {
        panic_with_error!(e, VoteError::AlreadyVoted);
    }

    let record = VoteRecord { option, shares };
    votes.set(voter.clone(), record.clone());

    let mut option_shares = storage::get_option_shares(e);
    let new_option_shares = option_shares
        .get(option)
        .unwrap_or_else(|| panic_with_error!(e, VoteError::InvalidOptionIndex))
        .checked_add(shares)
        .unwrap_or_else(|| panic_with_error!(e, VoteError::Overflow));
    option_shares.set(option, new_option_shares);

    let mut option_voter_counts = storage::get_option_voter_counts(e);
    let new_option_voter_count = option_voter_counts
        .get(option)
        .unwrap_or_else(|| panic_with_error!(e, VoteError::InvalidOptionIndex))
        .checked_add(1)
        .unwrap_or_else(|| panic_with_error!(e, VoteError::Overflow));
    option_voter_counts.set(option, new_option_voter_count);

    let total_voted_shares = storage::get_total_voted_shares(e)
        .checked_add(shares)
        .filter(|total| *total <= config.total_eligible_shares)
        .unwrap_or_else(|| panic_with_error!(e, VoteError::Overflow));
    let total_voters = storage::get_total_voters(e)
        .checked_add(1)
        .filter(|total| *total <= config.eligible_holder_count)
        .unwrap_or_else(|| panic_with_error!(e, VoteError::Overflow));

    let mut option_voters = storage::get_option_voters(e, option);
    option_voters.push_back(voter);

    storage::set_votes(e, &votes);
    storage::set_option_shares(e, &option_shares);
    storage::set_option_voter_counts(e, &option_voter_counts);
    storage::set_option_voters(e, option, &option_voters);
    storage::set_total_voted_shares(e, total_voted_shares);
    storage::set_total_voters(e, total_voters);

    record
}

#[contractimpl]
impl BlendVoteContract {
    pub fn __constructor(
        e: Env,
        proposal: String,
        options: Vec<String>,
        eligible_voters: Vec<(Address, i128)>,
        initial_votes: Vec<(Address, u32)>,
    ) {
        if proposal.is_empty() || proposal.len() > MAX_PROPOSAL_BYTES {
            panic_with_error!(&e, VoteError::InvalidProposal);
        }
        if options.len() < 2 || options.len() > MAX_OPTIONS {
            panic_with_error!(&e, VoteError::InvalidOptionCount);
        }

        let mut checked_options = Vec::new(&e);
        for option in options.iter() {
            if option.is_empty() || option.len() > MAX_OPTION_BYTES {
                panic_with_error!(&e, VoteError::InvalidOption);
            }
            for prior in checked_options.iter() {
                if prior == option {
                    panic_with_error!(&e, VoteError::DuplicateOption);
                }
            }
            checked_options.push_back(option);
        }

        if eligible_voters.len() != CANONICAL_ELIGIBLE_HOLDERS {
            panic_with_error!(&e, VoteError::InvalidEligibleHolderCount);
        }
        let mut allocations = Map::new(&e);
        let mut total_eligible_shares = 0_i128;
        let mut digest_input = Bytes::from_slice(&e, SNAPSHOT_DIGEST_DOMAIN);
        for (holder, shares) in eligible_voters.iter() {
            if shares <= 0 {
                panic_with_error!(&e, VoteError::InvalidAllocation);
            }
            if allocations.contains_key(holder.clone()) {
                panic_with_error!(&e, VoteError::DuplicateEligibleHolder);
            }
            total_eligible_shares = total_eligible_shares
                .checked_add(shares)
                .filter(|total| *total <= i128::MAX / PERCENT_7DP_SCALE)
                .unwrap_or_else(|| panic_with_error!(&e, VoteError::Overflow));
            digest_input.append(&holder.to_string().to_bytes());
            digest_input.extend_from_array(&shares.to_be_bytes());
            allocations.set(holder, shares);
        }
        let snapshot_allocation_digest = e.crypto().sha256(&digest_input).to_bytes();
        if total_eligible_shares != CANONICAL_TOTAL_ELIGIBLE_SHARES
            || snapshot_allocation_digest != BytesN::from_array(&e, &CANONICAL_ALLOCATION_DIGEST)
        {
            panic_with_error!(&e, VoteError::InvalidSnapshot);
        }

        let config = Config {
            proposal,
            options: checked_options,
            snapshot_sha256: BytesN::from_array(&e, &CANONICAL_SNAPSHOT_SHA256),
            snapshot_allocation_digest,
            total_eligible_shares,
            eligible_holder_count: allocations.len(),
        };
        let mut option_shares = Vec::<i128>::new(&e);
        let mut option_voter_counts = Vec::<u32>::new(&e);
        let mut option_voters = Map::<u32, Vec<Address>>::new(&e);
        for option_index in 0..config.options.len() {
            option_shares.push_back(0);
            option_voter_counts.push_back(0);
            option_voters.set(option_index, Vec::new(&e));
        }

        let mut votes = Map::new(&e);
        let mut total_voted_shares = 0_i128;
        for (voter, option) in initial_votes.iter() {
            require_option(&e, option, config.options.len());
            let shares = allocations
                .get(voter.clone())
                .unwrap_or_else(|| panic_with_error!(&e, VoteError::IneligibleVoter));
            if votes.contains_key(voter.clone()) {
                panic_with_error!(&e, VoteError::AlreadyVoted);
            }
            votes.set(voter.clone(), VoteRecord { option, shares });

            let shares_for_option = option_shares
                .get(option)
                .unwrap_or_else(|| panic_with_error!(&e, VoteError::InvalidOptionIndex))
                .checked_add(shares)
                .unwrap_or_else(|| panic_with_error!(&e, VoteError::Overflow));
            option_shares.set(option, shares_for_option);
            let voters_for_option = option_voter_counts
                .get(option)
                .unwrap_or_else(|| panic_with_error!(&e, VoteError::InvalidOptionIndex))
                .checked_add(1)
                .unwrap_or_else(|| panic_with_error!(&e, VoteError::Overflow));
            option_voter_counts.set(option, voters_for_option);
            total_voted_shares = total_voted_shares
                .checked_add(shares)
                .filter(|total| *total <= config.total_eligible_shares)
                .unwrap_or_else(|| panic_with_error!(&e, VoteError::Overflow));

            let mut voters = option_voters
                .get(option)
                .unwrap_or_else(|| panic_with_error!(&e, VoteError::InvalidOptionIndex));
            voters.push_back(voter);
            option_voters.set(option, voters);
        }

        storage::set_config(&e, &config);
        storage::set_allocations(&e, &allocations);
        storage::set_votes(&e, &votes);
        storage::set_option_shares(&e, &option_shares);
        storage::set_option_voter_counts(&e, &option_voter_counts);
        storage::set_total_voted_shares(&e, total_voted_shares);
        storage::set_total_voters(&e, initial_votes.len());
        for option in 0..config.options.len() {
            storage::set_option_voters(
                &e,
                option,
                &option_voters
                    .get(option)
                    .unwrap_or_else(|| panic_with_error!(&e, VoteError::InvalidOptionIndex)),
            );
        }
        storage::extend_instance(&e);
    }

    /// Casts all of `voter`'s immutable snapshot shares for one option.
    pub fn vote(e: Env, voter: Address, option: u32) -> VoteRecord {
        let record = record_vote(&e, voter.clone(), option);
        events::vote(&e, voter, option, record.shares);
        record
    }

    pub fn get_config(e: Env) -> Config {
        storage::extend_instance(&e);
        storage::get_config(&e)
    }

    pub fn get_allocation(e: Env, holder: Address) -> i128 {
        storage::extend_instance(&e);
        storage::get_allocations(&e).get(holder).unwrap_or(0)
    }

    pub fn get_vote(e: Env, holder: Address) -> Option<VoteRecord> {
        storage::extend_instance(&e);
        storage::get_votes(&e).get(holder)
    }

    pub fn get_results(e: Env) -> VoteResults {
        storage::extend_instance(&e);
        let config = storage::get_config(&e);
        let total_voted_shares = storage::get_total_voted_shares(&e);
        let total_voters = storage::get_total_voters(&e);
        let option_shares = storage::get_option_shares(&e);
        let option_voter_counts = storage::get_option_voter_counts(&e);
        let mut results = Vec::new(&e);

        for option in 0..config.options.len() {
            let shares = option_shares
                .get(option)
                .unwrap_or_else(|| panic_with_error!(&e, VoteError::InvalidOptionIndex));
            results.push_back(OptionResult {
                option,
                label: config
                    .options
                    .get(option)
                    .unwrap_or_else(|| panic_with_error!(&e, VoteError::InvalidOptionIndex)),
                shares,
                percent_of_cast_7dp: percent(&e, shares, total_voted_shares),
                percent_of_eligible_7dp: percent(&e, shares, config.total_eligible_shares),
                voter_count: option_voter_counts
                    .get(option)
                    .unwrap_or_else(|| panic_with_error!(&e, VoteError::InvalidOptionIndex)),
            });
        }

        VoteResults {
            total_eligible_shares: config.total_eligible_shares,
            total_voted_shares,
            participation_percent_7dp: percent(
                &e,
                total_voted_shares,
                config.total_eligible_shares,
            ),
            total_voters,
            options: results,
        }
    }

    pub fn get_voter_count(e: Env, option: u32) -> u32 {
        storage::extend_instance(&e);
        let config = storage::get_config(&e);
        require_option(&e, option, config.options.len());
        storage::get_option_voter_counts(&e)
            .get(option)
            .unwrap_or_else(|| panic_with_error!(&e, VoteError::InvalidOptionIndex))
    }

    /// Extends every contract-owned storage entry. Submit this as a transaction;
    /// an RPC simulation alone does not persist TTL changes.
    pub fn extend_state(e: Env) {
        storage::extend_instance(&e);
        let config = storage::get_config(&e);
        let _allocations = storage::get_allocations(&e);
        let _votes = storage::get_votes(&e);
        for option in 0..config.options.len() {
            let _voters = storage::get_option_voters(&e, option);
        }
    }

    /// Returns voters in cast order. `limit` must be between 1 and 100.
    pub fn get_voters(e: Env, option: u32, start: u32, limit: u32) -> Vec<Address> {
        storage::extend_instance(&e);
        let config = storage::get_config(&e);
        require_option(&e, option, config.options.len());
        if limit == 0 || limit > MAX_VOTER_PAGE {
            panic_with_error!(&e, VoteError::InvalidPage);
        }

        let voters = storage::get_option_voters(&e, option);
        let mut page = Vec::new(&e);
        if start >= voters.len() {
            return page;
        }
        let end = start.saturating_add(limit).min(voters.len());
        for index in start..end {
            page.push_back(
                voters
                    .get(index)
                    .unwrap_or_else(|| panic_with_error!(&e, VoteError::InvalidPage)),
            );
        }
        page
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, vec};

    const SNAPSHOT: &str =
        include_str!("../snapshot/comet_cpal_flattened_ownership_before_41c898a1.csv");

    fn canonical_allocations(e: &Env) -> Vec<(Address, i128)> {
        let mut allocations = Vec::new(e);
        for line in SNAPSHOT.lines().skip(1) {
            let fields: std::vec::Vec<&str> = line.split(',').collect();
            let shares: i128 = fields[11].parse().unwrap();
            if shares > 0 {
                allocations.push_back((Address::from_str(e, fields[1]), shares));
            }
        }
        allocations
    }

    fn expected_percent(numerator: i128, denominator: i128) -> i128 {
        numerator * PERCENT_7DP_SCALE / denominator
    }

    struct Fixture {
        e: Env,
        contract: Address,
        alice: Address,
        alice_shares: i128,
        bob: Address,
        bob_shares: i128,
        carol: Address,
    }

    impl Fixture {
        fn create(mock_auth: bool) -> Self {
            let e = Env::default();
            if mock_auth {
                e.mock_all_auths();
            }
            let allocations = canonical_allocations(&e);
            let (alice, alice_shares) = allocations.get(0).unwrap();
            let (bob, bob_shares) = allocations.get(1).unwrap();
            let (carol, _) = allocations.get(2).unwrap();
            let contract = e.register(
                BlendVoteContract,
                (
                    String::from_str(&e, "Which upgrade path should Blend follow?"),
                    vec![&e, String::from_str(&e, "V2.1"), String::from_str(&e, "V3")],
                    allocations,
                    Vec::<(Address, u32)>::new(&e),
                ),
            );
            Self {
                e,
                contract,
                alice,
                alice_shares,
                bob,
                bob_shares,
                carol,
            }
        }

        fn client(&self) -> BlendVoteContractClient<'_> {
            BlendVoteContractClient::new(&self.e, &self.contract)
        }
    }

    #[test]
    fn votes_with_the_full_snapshot_allocation_and_reports_results() {
        let fixture = Fixture::create(true);
        let client = fixture.client();

        assert_eq!(
            client.vote(&fixture.alice, &1),
            VoteRecord {
                option: 1,
                shares: fixture.alice_shares
            }
        );
        client.vote(&fixture.bob, &0);

        let results = client.get_results();
        let voted = fixture.alice_shares + fixture.bob_shares;
        assert_eq!(
            results.total_eligible_shares,
            CANONICAL_TOTAL_ELIGIBLE_SHARES
        );
        assert_eq!(results.total_voted_shares, voted);
        assert_eq!(
            results.participation_percent_7dp,
            expected_percent(voted, CANONICAL_TOTAL_ELIGIBLE_SHARES)
        );
        assert_eq!(results.total_voters, 2);
        assert_eq!(results.options.get(0).unwrap().shares, fixture.bob_shares);
        assert_eq!(
            results.options.get(0).unwrap().percent_of_cast_7dp,
            expected_percent(fixture.bob_shares, voted)
        );
        assert_eq!(results.options.get(1).unwrap().shares, fixture.alice_shares);
        assert_eq!(
            results.options.get(1).unwrap().percent_of_cast_7dp,
            expected_percent(fixture.alice_shares, voted)
        );
        assert_eq!(
            results.options.get(1).unwrap().percent_of_eligible_7dp,
            expected_percent(fixture.alice_shares, CANONICAL_TOTAL_ELIGIBLE_SHARES)
        );
        assert_eq!(client.get_allocation(&fixture.alice), fixture.alice_shares);
        assert_eq!(client.get_vote(&fixture.alice).unwrap().option, 1);
    }

    #[test]
    fn each_holder_can_vote_only_once() {
        let fixture = Fixture::create(true);
        let client = fixture.client();
        client.vote(&fixture.alice, &0);
        assert!(client.try_vote(&fixture.alice, &1).is_err());
        assert_eq!(
            client.get_results().total_voted_shares,
            fixture.alice_shares
        );
    }

    #[test]
    fn constructor_imports_votes_without_auth_and_blocks_a_second_vote() {
        let e = Env::default();
        let allocations = canonical_allocations(&e);
        let (alice, alice_shares) = allocations.get(0).unwrap();
        let (bob, bob_shares) = allocations.get(1).unwrap();
        let contract = e.register(
            BlendVoteContract,
            (
                String::from_str(&e, "Migrated poll"),
                vec![&e, String::from_str(&e, "Yes"), String::from_str(&e, "No")],
                allocations,
                vec![&e, (alice.clone(), 0_u32), (bob.clone(), 1_u32)],
            ),
        );
        let client = BlendVoteContractClient::new(&e, &contract);

        let results = client.get_results();
        assert_eq!(results.total_voters, 2);
        assert_eq!(results.total_voted_shares, alice_shares + bob_shares);
        assert_eq!(results.options.get(0).unwrap().shares, alice_shares);
        assert_eq!(results.options.get(1).unwrap().shares, bob_shares);
        assert_eq!(client.get_vote(&alice).unwrap().option, 0);
        assert_eq!(client.get_vote(&bob).unwrap().option, 1);
        assert_eq!(client.get_voters(&0, &0, &100), vec![&e, alice.clone()]);
        assert_eq!(client.get_voters(&1, &0, &100), vec![&e, bob]);

        e.mock_all_auths();
        assert!(client.try_vote(&alice, &1).is_err());
    }

    #[test]
    fn constructor_rejects_invalid_initial_votes() {
        let duplicate_env = Env::default();
        let duplicate_allocations = canonical_allocations(&duplicate_env);
        let (duplicate_voter, _) = duplicate_allocations.get(0).unwrap();
        let duplicate = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            duplicate_env.register(
                BlendVoteContract,
                (
                    String::from_str(&duplicate_env, "Proposal"),
                    vec![
                        &duplicate_env,
                        String::from_str(&duplicate_env, "Yes"),
                        String::from_str(&duplicate_env, "No"),
                    ],
                    duplicate_allocations,
                    vec![
                        &duplicate_env,
                        (duplicate_voter.clone(), 0_u32),
                        (duplicate_voter, 1_u32),
                    ],
                ),
            );
        }));
        assert!(duplicate.is_err());

        let ineligible_env = Env::default();
        let outsider = Address::generate(&ineligible_env);
        let ineligible = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ineligible_env.register(
                BlendVoteContract,
                (
                    String::from_str(&ineligible_env, "Proposal"),
                    vec![
                        &ineligible_env,
                        String::from_str(&ineligible_env, "Yes"),
                        String::from_str(&ineligible_env, "No"),
                    ],
                    canonical_allocations(&ineligible_env),
                    vec![&ineligible_env, (outsider, 0_u32)],
                ),
            );
        }));
        assert!(ineligible.is_err());

        let option_env = Env::default();
        let option_allocations = canonical_allocations(&option_env);
        let (voter, _) = option_allocations.get(0).unwrap();
        let invalid_option = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            option_env.register(
                BlendVoteContract,
                (
                    String::from_str(&option_env, "Proposal"),
                    vec![
                        &option_env,
                        String::from_str(&option_env, "Yes"),
                        String::from_str(&option_env, "No"),
                    ],
                    option_allocations,
                    vec![&option_env, (voter, 2_u32)],
                ),
            );
        }));
        assert!(invalid_option.is_err());
    }

    #[test]
    fn rejects_ineligible_voters_and_invalid_options() {
        let fixture = Fixture::create(true);
        let client = fixture.client();
        let outsider = Address::generate(&fixture.e);
        assert!(client.try_vote(&outsider, &0).is_err());
        assert!(client.try_vote(&fixture.alice, &2).is_err());
        assert_eq!(client.get_results().total_voters, 0);
    }

    #[test]
    fn vote_requires_holder_authorization() {
        let fixture = Fixture::create(false);
        assert!(fixture.client().try_vote(&fixture.alice, &0).is_err());
        assert_eq!(fixture.client().get_results().total_voters, 0);
    }

    #[test]
    fn voters_are_public_and_paginated_in_cast_order() {
        let fixture = Fixture::create(true);
        let client = fixture.client();
        client.vote(&fixture.alice, &1);
        client.vote(&fixture.carol, &1);

        assert_eq!(client.get_voter_count(&1), 2);
        assert_eq!(
            client.get_voters(&1, &0, &1),
            vec![&fixture.e, fixture.alice.clone()]
        );
        assert_eq!(
            client.get_voters(&1, &1, &100),
            vec![&fixture.e, fixture.carol.clone()]
        );
        assert!(client.try_get_voters(&1, &0, &0).is_err());
    }

    #[test]
    fn constructor_rejects_duplicate_holders_and_options() {
        let e = Env::default();
        let mut duplicate_allocations = canonical_allocations(&e);
        let first = duplicate_allocations.get(0).unwrap();
        let second = duplicate_allocations.get(1).unwrap();
        duplicate_allocations.set(1, (first.0, second.1));
        let duplicate_holder = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            e.register(
                BlendVoteContract,
                (
                    String::from_str(&e, "Proposal"),
                    vec![&e, String::from_str(&e, "A"), String::from_str(&e, "B")],
                    duplicate_allocations,
                    Vec::<(Address, u32)>::new(&e),
                ),
            );
        }));
        assert!(duplicate_holder.is_err());

        let duplicate_option = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            e.register(
                BlendVoteContract,
                (
                    String::from_str(&e, "Proposal"),
                    vec![&e, String::from_str(&e, "A"), String::from_str(&e, "A")],
                    canonical_allocations(&e),
                    Vec::<(Address, u32)>::new(&e),
                ),
            );
        }));
        assert!(duplicate_option.is_err());
    }

    #[test]
    fn canonical_snapshot_constructs_with_exact_count_and_total() {
        let e = Env::default();
        let contract = e.register(
            BlendVoteContract,
            (
                String::from_str(&e, "Canonical snapshot construction test"),
                vec![&e, String::from_str(&e, "Yes"), String::from_str(&e, "No")],
                canonical_allocations(&e),
                Vec::<(Address, u32)>::new(&e),
            ),
        );
        let config = BlendVoteContractClient::new(&e, &contract).get_config();
        assert_eq!(config.eligible_holder_count, CANONICAL_ELIGIBLE_HOLDERS);
        assert_eq!(
            config.total_eligible_shares,
            CANONICAL_TOTAL_ELIGIBLE_SHARES
        );
        assert_eq!(
            config.snapshot_sha256,
            BytesN::from_array(&e, &CANONICAL_SNAPSHOT_SHA256)
        );
        assert_eq!(
            config.snapshot_allocation_digest,
            BytesN::from_array(&e, &CANONICAL_ALLOCATION_DIGEST)
        );
    }

    #[test]
    fn constructor_rejects_a_modified_canonical_snapshot() {
        let e = Env::default();
        let mut allocations = canonical_allocations(&e);
        let first = allocations.get(0).unwrap();
        let second = allocations.get(1).unwrap();
        allocations.set(0, (first.0, first.1 - 1));
        allocations.set(1, (second.0, second.1 + 1));

        let modified = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            e.register(
                BlendVoteContract,
                (
                    String::from_str(&e, "Proposal"),
                    vec![&e, String::from_str(&e, "A"), String::from_str(&e, "B")],
                    allocations,
                    Vec::<(Address, u32)>::new(&e),
                ),
            );
        }));
        assert!(modified.is_err());
    }
}
