#![no_std]

mod cycle;
mod errors;
mod group;
mod storage;
mod types;
mod vault;

pub use errors::AjoError;
pub use types::{GroupConfig, GroupState, GroupStatus, ParticipantRecord, VaultDeposit, VaultTier};

use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

#[contract]
pub struct AjoContract;

#[contractimpl]
impl AjoContract {
    // ── Group lifecycle ──────────────────────────────────────────────────────

    /// Create a new Ajo savings group.
    ///
    /// Admin is auto-enrolled in slot 0.  Returns the new `group_id`.
    pub fn create_group(
        env: Env,
        admin: Address,
        name: String,
        token: Address,
        contribution_amount: i128,
        cycle_interval_ledgers: u32,
        max_participants: u32,
    ) -> u32 {
        group::create_group(
            &env,
            admin,
            name,
            token,
            contribution_amount,
            cycle_interval_ledgers,
            max_participants,
        )
    }

    /// Join an open group.  Fails when the group is full or already active.
    pub fn join_group(env: Env, group_id: u32, participant: Address) -> Result<(), AjoError> {
        group::join_group(&env, group_id, participant)
    }

    // ── Contribution & two-step payout ───────────────────────────────────────

    /// Deposit one cycle contribution.
    ///
    /// Caller must have approved this contract to transfer `contribution_amount`
    /// tokens beforehand.  When the last participant contributes, the contract
    /// automatically sets `payout_pending = true`.
    pub fn contribute(env: Env, group_id: u32, participant: Address) -> Result<(), AjoError> {
        cycle::contribute(&env, group_id, participant)
    }

    /// Recipient explicitly claims their pending payout.
    ///
    /// Must be called by the address at the current rotation slot.  Transfers
    /// the pooled funds, records missed cycles for non-contributors, and
    /// advances the rotation.
    pub fn claim_payout(env: Env, group_id: u32) -> Result<Address, AjoError> {
        cycle::claim_payout(&env, group_id)
    }

    // ── Read-only queries ────────────────────────────────────────────────────

    pub fn get_config(env: Env, group_id: u32) -> Result<GroupConfig, AjoError> {
        group::get_config(&env, group_id)
    }

    pub fn get_state(env: Env, group_id: u32) -> Result<GroupState, AjoError> {
        group::get_state(&env, group_id)
    }

    /// Return every group ID that `participant` has joined (reverse index).
    pub fn get_participant_groups(env: Env, participant: Address) -> Vec<u32> {
        storage::get_participant_groups(&env, &participant)
    }

    /// Returns up to limit group IDs for the given status, starting at offset.
    /// Pass None for status to list across all statuses.
    pub fn list_groups(env: Env, status: Option<GroupStatus>, offset: u32, limit: u32) -> Vec<u32> {
        let capped_limit = limit.min(50);
        let mut result = Vec::new(&env);
        
        if let Some(s) = status {
            let groups = storage::get_groups_by_status(&env, &s);
            let len = groups.len();
            let start = offset.min(len);
            let end = (start + capped_limit).min(len);
            for i in start..end {
                result.push_back(groups.get(i).unwrap());
            }
        } else {
            let total = storage::get_total_groups(&env);
            let start = offset + 1;
            let end = start + capped_limit;
            for id in start..end {
                if id <= total {
                    result.push_back(id);
                }
            }
        }
        
        result
    }

    /// Total count of groups in a given status (or all groups if None).
    pub fn count_groups(env: Env, status: Option<GroupStatus>) -> u32 {
        if let Some(s) = status {
            storage::get_groups_by_status(&env, &s).len()
        } else {
            storage::get_total_groups(&env)
        }
    }

    /// Bulk-resolve configs for a slice of IDs in one call (avoids N round-trips).
    pub fn get_configs(env: Env, group_ids: Vec<u32>) -> Vec<GroupConfig> {
        let mut result = Vec::new(&env);
        for id in group_ids.iter() {
            if let Ok(config) = group::get_config(&env, id) {
                result.push_back(config);
            }
        }
        result
    }

    /// Bulk-resolve states for a slice of IDs in one call.
    pub fn get_states(env: Env, group_ids: Vec<u32>) -> Vec<GroupState> {
        let mut result = Vec::new(&env);
        for id in group_ids.iter() {
            if let Ok(state) = group::get_state(&env, id) {
                result.push_back(state);
            }
        }
        result
    }

    // ── Vault ────────────────────────────────────────────────────────────────

    /// Deposit `amount` of `token` into a tiered savings vault.
    ///
    /// Caller must have approved this contract for the transfer.
    /// Returns the new `vault_id`.
    pub fn deposit_vault(
        env: Env,
        owner: Address,
        token: Address,
        amount: i128,
        tier: VaultTier,
    ) -> Result<u32, AjoError> {
        vault::deposit(&env, owner, token, amount, tier)
    }

    /// Claim principal plus accrued yield from a matured vault.
    ///
    /// Returns the total amount transferred.
    pub fn claim_vault(env: Env, vault_id: u32, owner: Address) -> Result<i128, AjoError> {
        vault::claim_vault(&env, vault_id, owner)
    }

    /// Return stored details for a vault.
    pub fn get_vault(env: Env, vault_id: u32) -> Result<VaultDeposit, AjoError> {
        vault::get_vault_info(&env, vault_id)
    }

    /// Preview accrued yield for a vault at the current ledger.
    pub fn preview_yield(env: Env, vault_id: u32) -> Result<i128, AjoError> {
        vault::preview_yield(&env, vault_id)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests;
