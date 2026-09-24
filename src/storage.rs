use crate::{Config, VoteRecord};
use soroban_sdk::{contracttype, unwrap::UnwrapOptimized, Address, Env, Map, Vec};

const ONE_DAY_LEDGERS: u32 = 17_280;
const TTL_THRESHOLD: u32 = 90 * ONE_DAY_LEDGERS;
const TTL_BUMP: u32 = 180 * ONE_DAY_LEDGERS;

#[derive(Clone)]
#[contracttype]
enum InstanceKey {
    Config,
    TotalVotedShares,
    TotalVoters,
    OptionShares,
    OptionVoterCounts,
}

#[derive(Clone)]
#[contracttype]
enum PersistentKey {
    Allocations,
    Votes,
    OptionVoters(u32),
}

pub fn extend_instance(e: &Env) {
    e.storage().instance().extend_ttl(TTL_THRESHOLD, TTL_BUMP);
}

pub fn set_config(e: &Env, config: &Config) {
    e.storage().instance().set(&InstanceKey::Config, config);
}

pub fn get_config(e: &Env) -> Config {
    e.storage()
        .instance()
        .get(&InstanceKey::Config)
        .unwrap_optimized()
}

pub fn set_total_voted_shares(e: &Env, value: i128) {
    e.storage()
        .instance()
        .set(&InstanceKey::TotalVotedShares, &value);
}

pub fn get_total_voted_shares(e: &Env) -> i128 {
    e.storage()
        .instance()
        .get(&InstanceKey::TotalVotedShares)
        .unwrap_or(0)
}

pub fn set_total_voters(e: &Env, value: u32) {
    e.storage()
        .instance()
        .set(&InstanceKey::TotalVoters, &value);
}

pub fn get_total_voters(e: &Env) -> u32 {
    e.storage()
        .instance()
        .get(&InstanceKey::TotalVoters)
        .unwrap_or(0)
}

pub fn set_option_shares(e: &Env, values: &Vec<i128>) {
    e.storage()
        .instance()
        .set(&InstanceKey::OptionShares, values);
}

pub fn get_option_shares(e: &Env) -> Vec<i128> {
    e.storage()
        .instance()
        .get(&InstanceKey::OptionShares)
        .unwrap_optimized()
}

pub fn set_option_voter_counts(e: &Env, values: &Vec<u32>) {
    e.storage()
        .instance()
        .set(&InstanceKey::OptionVoterCounts, values);
}

pub fn get_option_voter_counts(e: &Env) -> Vec<u32> {
    e.storage()
        .instance()
        .get(&InstanceKey::OptionVoterCounts)
        .unwrap_optimized()
}

fn bump_persistent(e: &Env, key: &PersistentKey) {
    e.storage()
        .persistent()
        .extend_ttl(key, TTL_THRESHOLD, TTL_BUMP);
}

pub fn set_allocations(e: &Env, values: &Map<Address, i128>) {
    let key = PersistentKey::Allocations;
    e.storage().persistent().set(&key, values);
    bump_persistent(e, &key);
}

pub fn get_allocations(e: &Env) -> Map<Address, i128> {
    let key = PersistentKey::Allocations;
    let values = e.storage().persistent().get(&key).unwrap_optimized();
    bump_persistent(e, &key);
    values
}

pub fn set_votes(e: &Env, values: &Map<Address, VoteRecord>) {
    let key = PersistentKey::Votes;
    e.storage().persistent().set(&key, values);
    bump_persistent(e, &key);
}

pub fn get_votes(e: &Env) -> Map<Address, VoteRecord> {
    let key = PersistentKey::Votes;
    let values = e.storage().persistent().get(&key).unwrap_optimized();
    bump_persistent(e, &key);
    values
}

pub fn set_option_voters(e: &Env, option: u32, values: &Vec<Address>) {
    let key = PersistentKey::OptionVoters(option);
    e.storage().persistent().set(&key, values);
    bump_persistent(e, &key);
}

pub fn get_option_voters(e: &Env, option: u32) -> Vec<Address> {
    let key = PersistentKey::OptionVoters(option);
    let voter = e.storage().persistent().get(&key).unwrap_optimized();
    bump_persistent(e, &key);
    voter
}
