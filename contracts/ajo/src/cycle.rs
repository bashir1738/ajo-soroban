use soroban_sdk::{token, Address, Env, Symbol, Vec};

use crate::errors::AjoError;
use crate::storage::{
    index_group_by_status, load_config, load_participant, load_state,
    remove_group_from_status_index, save_participant, save_state,
};
use crate::types::{GroupStatus, ParticipantRecord};

/// Deposit the cycle contribution from `participant`.
///
/// The participant must have pre-approved this contract via token `approve`.
/// When the last participant contributes, `payout_pending` is automatically
/// set to `true` — the recipient then calls `claim_payout` to receive funds.
pub fn contribute(env: &Env, group_id: u32, participant: Address) -> Result<(), AjoError> {
    participant.require_auth();

    let config = load_config(env, group_id)?;
    let mut state = load_state(env, group_id)?;

    if state.status != GroupStatus::Active {
        return Err(AjoError::InvalidGroupStatus);
    }
    // Block new contributions while the previous round's payout hasn't been claimed.
    if state.payout_pending {
        return Err(AjoError::PayoutAlreadyPending);
    }
    if !state.participants.contains(&participant) {
        return Err(AjoError::NotAMember);
    }
    if state.paid_this_cycle.contains(&participant) {
        return Err(AjoError::AlreadyPaid);
    }

    // Pull tokens from participant into this contract.
    let token_client = token::Client::new(env, &config.token);
    token_client.transfer(
        &participant,
        &env.current_contract_address(),
        &config.contribution_amount,
    );

    state.paid_this_cycle.push_back(participant.clone());

    // Auto-trigger payout when every participant has contributed.
    if state.paid_this_cycle.len() >= state.participants.len() {
        state.payout_pending = true;
    }

    save_state(env, group_id, &state);
    Ok(())
}

/// Recipient explicitly claims their pending payout (two-step model).
///
/// Only callable by the address that is next in the rotation.  Funds are
/// transferred, missed-cycle counts are updated for absent contributors, and
/// the group advances to the next cycle.
pub fn claim_payout(env: &Env, group_id: u32) -> Result<Address, AjoError> {
    let config = load_config(env, group_id)?;
    let mut state = load_state(env, group_id)?;

    if state.status != GroupStatus::Active {
        return Err(AjoError::InvalidGroupStatus);
    }
    if !state.payout_pending {
        return Err(AjoError::NoPendingPayout);
    }

    let recipient = state
        .participants
        .get(state.current_cycle)
        .ok_or(AjoError::GroupNotFound)?;

    // Only the designated recipient may claim.
    recipient.require_auth();

    // Transfer the full pool to the recipient.
    let pool = config
        .contribution_amount
        .checked_mul(state.participants.len() as i128)
        .ok_or(AjoError::Overflow)?;

    let token_client = token::Client::new(env, &config.token);
    token_client.transfer(&env.current_contract_address(), &recipient, &pool);

    // Mark recipient's record.
    let mut record = load_participant(env, group_id, &recipient)
        .unwrap_or(ParticipantRecord { missed_cycles: 0, received_payout: false });
    record.received_payout = true;
    save_participant(env, group_id, &recipient, &record);

    // Increment missed_cycles for anyone who did not contribute this round.
    for addr in state.participants.iter() {
        if !state.paid_this_cycle.contains(&addr) {
            let mut r = load_participant(env, group_id, &addr)
                .unwrap_or(ParticipantRecord { missed_cycles: 0, received_payout: false });
            r.missed_cycles += 1;
            save_participant(env, group_id, &addr, &r);
        }
    }

    // Advance to the next cycle.
    state.current_cycle += 1;
    state.payout_pending = false;
    state.paid_this_cycle = Vec::new(env);
    state.cycle_start_ledger = env.ledger().sequence();
    state.next_payout_ledger = env.ledger().sequence() + config.cycle_interval_ledgers;

    // Mark the rotation complete when all participants have been paid out.
    if state.current_cycle >= state.participants.len() {
        state.status = GroupStatus::Complete;
        
        remove_group_from_status_index(env, group_id, &GroupStatus::Active);
        index_group_by_status(env, group_id, &GroupStatus::Complete);
        env.events().publish((Symbol::new(env, "circle_completed"), group_id), ());
    }

    save_state(env, group_id, &state);
    Ok(recipient)
}
