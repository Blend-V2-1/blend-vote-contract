use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VoteError {
    InvalidProposal = 1400,
    InvalidOptionCount = 1401,
    InvalidOption = 1402,
    DuplicateOption = 1403,
    InvalidEligibleHolderCount = 1404,
    InvalidAllocation = 1405,
    DuplicateEligibleHolder = 1406,
    IneligibleVoter = 1407,
    AlreadyVoted = 1408,
    InvalidOptionIndex = 1409,
    InvalidPage = 1410,
    Overflow = 1411,
    InvalidSnapshot = 1412,
}
