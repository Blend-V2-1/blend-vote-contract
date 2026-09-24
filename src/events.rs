use soroban_sdk::{contractevent, Address, Env};

#[contractevent(topics = ["vote"], data_format = "vec")]
struct VoteEvent {
    #[topic]
    voter: Address,
    #[topic]
    option: u32,
    shares: i128,
}

pub fn vote(e: &Env, voter: Address, option: u32, shares: i128) {
    VoteEvent {
        voter,
        option,
        shares,
    }
    .publish(e);
}
