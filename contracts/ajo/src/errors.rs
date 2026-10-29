use soroban_sdk::contracterror;

#[contracterror]
#[derive(Clone, Debug, PartialEq)]
pub enum AjoError {
    // ── Group errors (1-10) ──────────────────────────────────────────────────
    /// Caller is not authorised for this action.
    Unauthorized = 1,
    /// Requested group / vault does not exist.
    GroupNotFound = 2,
    /// Group is not in the required state for this operation.
    InvalidGroupStatus = 3,
    /// Group has reached its participant cap.
    GroupFull = 4,
    /// Caller is already a member of this group.
    AlreadyJoined = 5,
    /// Caller is not a member of this group.
    NotAMember = 6,
    /// Caller has already contributed for the current cycle.
    AlreadyPaid = 7,
    /// Not all participants have contributed; payout cannot proceed.
    CycleIncomplete = 8,
    /// The cycle interval has not elapsed yet.
    CycleNotReady = 9,
    /// Arithmetic overflow.
    Overflow = 10,
    // ── Payout errors (11-13) ────────────────────────────────────────────────
    /// `claim_payout` called but no payout has been triggered yet.
    NoPendingPayout = 11,
    /// `contribute` called while a payout claim is still outstanding.
    PayoutAlreadyPending = 12,
    // ── Vault errors (13-16) ─────────────────────────────────────────────────
    /// Requested vault does not exist.
    VaultNotFound = 13,
    /// Vault has already been claimed.
    VaultAlreadyClaimed = 14,
    /// Vault maturity ledger has not been reached yet.
    VaultLocked = 15,
    /// Deposit amount is below the tier's minimum.
    InsufficientDeposit = 16,
    // ── Treasury & Receipt errors (17-19) ──────────────────────────────────
    /// Treasury reserve is insufficient to back this deposit at the target reserve ratio.
    InsufficientReserve = 17,
    /// Receipt token contract is not configured for this tier.
    ReceiptTokenNotSet = 18,
    /// Burning receipt token failed.
    ReceiptBurnFailed = 19,
}
