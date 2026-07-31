use soroban_sdk::{contracttype, Address, String, Vec};

/// Current lifecycle state of a savings group.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum GroupStatus {
    /// Accepting new participants; no contributions yet.
    Open,
    /// All slots filled; contributions accepted.
    Active,
    /// Every participant has received one payout.
    Complete,
}

/// Immutable configuration set at group creation time.
#[contracttype]
#[derive(Clone, Debug)]
pub struct GroupConfig {
    /// Human-readable name (e.g. "Lagos Savers").
    pub name: String,
    /// Token contract address (USDC or any SEP-41 token).
    pub token: Address,
    /// Amount each participant must contribute per cycle, in token stroops.
    pub contribution_amount: i128,
    /// Minimum ledger gap between the start of consecutive cycles.
    pub cycle_interval_ledgers: u32,
    /// Maximum (and exact) number of participants.
    pub max_participants: u32,
    /// Account that created the group.
    pub admin: Address,
}

/// Mutable runtime state of a savings group.
#[contracttype]
#[derive(Clone, Debug)]
pub struct GroupState {
    pub status: GroupStatus,
    /// Ordered list of participants; index determines payout rotation.
    pub participants: Vec<Address>,
    /// Index into `participants` for the current cycle's payout recipient.
    pub current_cycle: u32,
    /// Ledger number when the current cycle started.
    pub cycle_start_ledger: u32,
    /// Ledger number when the next payout is expected (cycle_start + interval).
    pub next_payout_ledger: u32,
    /// Addresses that have contributed in the current cycle.
    pub paid_this_cycle: Vec<Address>,
    /// True once all participants have contributed; recipient must call
    /// `claim_payout` to receive funds (two-step model).
    pub payout_pending: bool,
}

/// Per-participant record used for missed-cycle and reputation tracking.
#[contracttype]
#[derive(Clone, Debug)]
pub struct ParticipantRecord {
    /// Number of cycles where this participant did not contribute in time.
    pub missed_cycles: u32,
    pub received_payout: bool,
}

// ── Vault types ────────────────────────────────────────────────────────────

/// Tiered savings vault (mirrors Rolla's Flex / Growth / Power tiers).
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum VaultTier {
    /// No lock period.  APR: 4.50 % (450 bps).  Min deposit: 10 USDC.
    Flex,
    /// ~90-day lock period.  APR: 9.20 % (920 bps).  Min deposit: 100 USDC.
    Growth,
    /// ~365-day lock period.  APR: 14.80 % (1 480 bps).  Min deposit: 500 USDC.
    Power,
}

/// A single vault deposit record.
#[contracttype]
#[derive(Clone, Debug)]
pub struct VaultDeposit {
    pub owner: Address,
    pub token: Address,
    /// Original deposited amount.
    pub principal: i128,
    pub tier: VaultTier,
    /// Ledger at which the deposit was made.
    pub deposit_ledger: u32,
    /// Ledger at which funds become claimable; 0 for Flex (always claimable).
    pub maturity_ledger: u32,
    pub claimed: bool,
}

// ── Storage key namespacing ────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    /// GroupConfig keyed by group_id.
    Config(u32),
    /// GroupState keyed by group_id.
    State(u32),
    /// ParticipantRecord keyed by (group_id, address).
    Participant(u32, Address),
    /// Monotonically increasing counter for group IDs.
    NextGroupId,
    /// Vec<u32> of group IDs this address belongs to (reverse index).
    ParticipantGroups(Address),
    /// Monotonically increasing counter for vault IDs.
    NextVaultId,
    /// VaultDeposit keyed by vault_id.
    Vault(u32),
    /// Reverse index of group IDs by their GroupStatus.
    GroupsByStatus(GroupStatus),
    /// Monotonically increasing counter for all groups ever created.
    TotalGroups,
}
