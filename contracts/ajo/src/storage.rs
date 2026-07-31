use soroban_sdk::{Address, Env, Vec};

use crate::errors::AjoError;
use crate::types::{DataKey, GroupConfig, GroupState, GroupStatus, ParticipantRecord, VaultDeposit};

// Storage rent bump constants.
// Soroban charges rent per ledger entry; bumping TTL keeps entries alive.
// 1 day ≈ 17,280 ledgers (5-second ledger time).
const DAY_LEDGERS: u32 = 17_280;
const BUMP_AMOUNT: u32 = 60 * DAY_LEDGERS; // extend by ~60 days on access
const MIN_TTL: u32 = 30 * DAY_LEDGERS; // only bump when TTL drops below 30 days

// ── NextGroupId ────────────────────────────────────────────────────────────

pub fn get_next_group_id(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get::<_, u32>(&DataKey::NextGroupId)
        .unwrap_or(1)
}

pub fn increment_group_id(env: &Env) -> u32 {
    let id = get_next_group_id(env);
    env.storage()
        .instance()
        .set(&DataKey::NextGroupId, &(id + 1));
    id
}

// ── TotalGroups ────────────────────────────────────────────────────────────

pub fn get_total_groups(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get::<_, u32>(&DataKey::TotalGroups)
        .unwrap_or(0)
}

pub fn increment_total_groups(env: &Env) -> u32 {
    let count = get_total_groups(env) + 1;
    env.storage()
        .instance()
        .set(&DataKey::TotalGroups, &count);
    count
}

// ── GroupConfig ────────────────────────────────────────────────────────────

pub fn save_config(env: &Env, group_id: u32, config: &GroupConfig) {
    let key = DataKey::Config(group_id);
    env.storage().persistent().set(&key, config);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, BUMP_AMOUNT);
}

pub fn load_config(env: &Env, group_id: u32) -> Result<GroupConfig, AjoError> {
    let key = DataKey::Config(group_id);
    let config = env
        .storage()
        .persistent()
        .get::<_, GroupConfig>(&key)
        .ok_or(AjoError::GroupNotFound)?;
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, BUMP_AMOUNT);
    Ok(config)
}

// ── GroupState ─────────────────────────────────────────────────────────────

pub fn save_state(env: &Env, group_id: u32, state: &GroupState) {
    let key = DataKey::State(group_id);
    env.storage().persistent().set(&key, state);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, BUMP_AMOUNT);
}

pub fn load_state(env: &Env, group_id: u32) -> Result<GroupState, AjoError> {
    let key = DataKey::State(group_id);
    let state = env
        .storage()
        .persistent()
        .get::<_, GroupState>(&key)
        .ok_or(AjoError::GroupNotFound)?;
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, BUMP_AMOUNT);
    Ok(state)
}

// ── ParticipantRecord ──────────────────────────────────────────────────────

pub fn save_participant(
    env: &Env,
    group_id: u32,
    address: &Address,
    record: &ParticipantRecord,
) {
    let key = DataKey::Participant(group_id, address.clone());
    env.storage().persistent().set(&key, record);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, BUMP_AMOUNT);
}

pub fn load_participant(
    env: &Env,
    group_id: u32,
    address: &Address,
) -> Option<ParticipantRecord> {
    let key = DataKey::Participant(group_id, address.clone());
    let record = env
        .storage()
        .persistent()
        .get::<_, ParticipantRecord>(&key)?;
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, BUMP_AMOUNT);
    Some(record)
}

// ── Participant groups reverse index ───────────────────────────────────────

/// Return every group_id this address belongs to.
pub fn get_participant_groups(env: &Env, participant: &Address) -> Vec<u32> {
    let key = DataKey::ParticipantGroups(participant.clone());
    if let Some(groups) = env
        .storage()
        .persistent()
        .get::<_, Vec<u32>>(&key)
    {
        env.storage()
            .persistent()
            .extend_ttl(&key, MIN_TTL, BUMP_AMOUNT);
        groups
    } else {
        Vec::new(env)
    }
}

/// Append a group_id to the participant's reverse index.
pub fn add_participant_group(env: &Env, participant: &Address, group_id: u32) {
    let key = DataKey::ParticipantGroups(participant.clone());
    let mut groups = get_participant_groups(env, participant);
    groups.push_back(group_id);
    env.storage().persistent().set(&key, &groups);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, BUMP_AMOUNT);
}

// ── Status Groups reverse index ────────────────────────────────────────────

pub fn get_groups_by_status(env: &Env, status: &GroupStatus) -> Vec<u32> {
    let key = DataKey::GroupsByStatus(status.clone());
    if let Some(groups) = env
        .storage()
        .persistent()
        .get::<_, Vec<u32>>(&key)
    {
        env.storage()
            .persistent()
            .extend_ttl(&key, MIN_TTL, BUMP_AMOUNT);
        groups
    } else {
        Vec::new(env)
    }
}

pub fn index_group_by_status(env: &Env, group_id: u32, status: &GroupStatus) {
    let key = DataKey::GroupsByStatus(status.clone());
    let mut groups = get_groups_by_status(env, status);
    if !groups.contains(&group_id) {
        groups.push_back(group_id);
        env.storage().persistent().set(&key, &groups);
    }
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, BUMP_AMOUNT);
}

pub fn remove_group_from_status_index(env: &Env, group_id: u32, status: &GroupStatus) {
    let key = DataKey::GroupsByStatus(status.clone());
    let mut groups = get_groups_by_status(env, status);
    
    if let Some(index) = groups.first_index_of(group_id) {
        groups.remove(index);
        env.storage().persistent().set(&key, &groups);
    }
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, BUMP_AMOUNT);
}

// ── NextVaultId ────────────────────────────────────────────────────────────

pub fn get_next_vault_id(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get::<_, u32>(&DataKey::NextVaultId)
        .unwrap_or(1)
}

pub fn increment_vault_id(env: &Env) -> u32 {
    let id = get_next_vault_id(env);
    env.storage()
        .instance()
        .set(&DataKey::NextVaultId, &(id + 1));
    id
}

// ── VaultDeposit ───────────────────────────────────────────────────────────

pub fn save_vault(env: &Env, vault_id: u32, vault: &VaultDeposit) {
    let key = DataKey::Vault(vault_id);
    env.storage().persistent().set(&key, vault);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, BUMP_AMOUNT);
}

pub fn load_vault(env: &Env, vault_id: u32) -> Result<VaultDeposit, AjoError> {
    let key = DataKey::Vault(vault_id);
    let vault = env
        .storage()
        .persistent()
        .get::<_, VaultDeposit>(&key)
        .ok_or(AjoError::VaultNotFound)?;
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, BUMP_AMOUNT);
    Ok(vault)
}
