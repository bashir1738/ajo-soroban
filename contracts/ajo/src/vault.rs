/// Tiered savings vault (mirrors Rolla's Flex / Growth / Power tiers).
///
/// Users deposit a SEP-41 token and earn a dynamic APR based on their chosen
/// lock tier and TVL utilization against the treasury reserve. The contract
/// holds the principal; yield is backed and paid from the protocol's
/// treasury reserve.
use soroban_sdk::{token, Address, Env};

use crate::errors::AjoError;
use crate::storage::{
    get_admin, get_treasury, get_vault_token, increment_vault_id, load_vault, save_treasury,
    save_vault, set_admin, set_vault_token as store_vault_token,
};
use crate::types::{TreasuryState, VaultDeposit, VaultTier};

// 1 day in ledgers at ~5 s per ledger.
const DAY_LEDGERS: u32 = 17_280;
// Approximate ledgers in one year.
const YEAR_LEDGERS: i128 = 365 * 17_280;

/// Tier-specific parameters: (lock_ledgers, base_apr_bps, tier_ceiling_bps, min_deposit_stroops).
///
/// Min deposits assume a 6-decimal token (USDC):
///   10 USDC  = 10_000_000 stroops
///  100 USDC  = 100_000_000 stroops
///  500 USDC  = 500_000_000 stroops
pub fn tier_params(tier: &VaultTier) -> (u32, u32, u32, i128) {
    match tier {
        VaultTier::Flex => (
            0,                    // no lock
            450,                  // 4.50 % base APR
            800,                  // 8.00 % ceiling APR
            10_000_000,           // min 10 USDC
        ),
        VaultTier::Growth => (
            90 * DAY_LEDGERS,     // ~90-day lock
            920,                  // 9.20 % base APR
            1_400,                // 14.00 % ceiling APR
            100_000_000,          // min 100 USDC
        ),
        VaultTier::Power => (
            365 * DAY_LEDGERS,    // ~365-day lock
            1_480,                // 14.80 % base APR
            2_200,                // 22.00 % ceiling APR
            500_000_000,          // min 500 USDC
        ),
    }
}

/// Compute dynamic APR based on utilization curve:
/// `utilization = total_principal / reserve_balance` (clamped 0–10_000 bps)
/// `effective_apr = base_apr * (10_000 - utilization) / 10_000 + tier_ceiling * utilization / 10_000`
pub fn calculate_dynamic_apr(total_principal: i128, reserve_balance: i128, tier: &VaultTier) -> u32 {
    let (_lock, base_apr, tier_ceiling, _min) = tier_params(tier);
    if reserve_balance <= 0 {
        return tier_ceiling;
    }
    let utilization = if total_principal <= 0 {
        0u32
    } else {
        let util = (total_principal.saturating_mul(10_000) / reserve_balance) as u32;
        util.min(10_000)
    };
    let effective_apr = ((base_apr as u64 * (10_000 - utilization) as u64)
        + (tier_ceiling as u64 * utilization as u64))
        / 10_000;
    effective_apr as u32
}

/// Returns the current dynamic APR in basis points for the given tier based on active TVL.
pub fn get_current_apr(env: &Env, tier: VaultTier) -> u32 {
    let treasury = get_treasury(env);
    calculate_dynamic_apr(treasury.total_principal, treasury.reserve_balance, &tier)
}

/// Superadmin deposits yield reserve funds into the treasury contract.
pub fn admin_deposit_reserve(
    env: &Env,
    admin: Address,
    token: Address,
    amount: i128,
) -> Result<(), AjoError> {
    admin.require_auth();
    if let Some(configured_admin) = get_admin(env) {
        if admin != configured_admin {
            return Err(AjoError::Unauthorized);
        }
    } else {
        set_admin(env, &admin);
    }
    if amount <= 0 {
        return Err(AjoError::InsufficientDeposit);
    }

    let token_client = token::Client::new(env, &token);
    token_client.transfer(&admin, &env.current_contract_address(), &amount);

    let mut treasury = get_treasury(env);
    treasury.reserve_balance = treasury.reserve_balance.saturating_add(amount);
    save_treasury(env, &treasury);

    Ok(())
}

/// Read-only: returns the current global treasury state.
pub fn get_treasury_state(env: &Env) -> TreasuryState {
    get_treasury(env)
}

/// Read-only: returns `reserve_balance * 10_000 / total_principal` in bps.
/// Clients warn if below 500 bps (5%). If `total_principal == 0`, returns 10_000 bps.
pub fn reserve_health(env: &Env) -> u32 {
    let treasury = get_treasury(env);
    if treasury.total_principal <= 0 {
        return 10_000;
    }
    let health = (treasury.reserve_balance.saturating_mul(10_000) / treasury.total_principal) as u32;
    health
}

/// Superadmin registers a SEP-41 receipt token contract for a given tier.
pub fn set_vault_token(
    env: &Env,
    admin: Address,
    tier: VaultTier,
    token_address: Address,
) -> Result<(), AjoError> {
    admin.require_auth();
    if let Some(configured_admin) = get_admin(env) {
        if admin != configured_admin {
            return Err(AjoError::Unauthorized);
        }
    } else {
        set_admin(env, &admin);
    }
    store_vault_token(env, &tier, &token_address);
    Ok(())
}

/// Compute accrued yield.
///
/// Formula: `principal * apr_bps * ledgers_held / (10_000 * YEAR_LEDGERS)`
fn compute_yield(principal: i128, apr_bps: i128, ledgers_held: u32) -> i128 {
    principal
        .saturating_mul(apr_bps)
        .saturating_mul(ledgers_held as i128)
        / (10_000 * YEAR_LEDGERS)
}

/// Deposit `amount` of `token` into a new vault with the given `tier`.
///
/// Checks reserve solvency before accepting deposit (`InsufficientReserve = 17`).
/// Locks in current dynamic APR, increments total principal, and mints SEP-41 receipt tokens.
pub fn deposit(
    env: &Env,
    owner: Address,
    token: Address,
    amount: i128,
    tier: VaultTier,
) -> Result<u32, AjoError> {
    owner.require_auth();

    let (lock_ledgers, _base_apr, _ceiling, min_deposit) = tier_params(&tier);

    if amount < min_deposit {
        return Err(AjoError::InsufficientDeposit);
    }

    // Solvency check: reserve_balance >= (total_principal + amount) * reserve_ratio_bps / 10_000
    let mut treasury = get_treasury(env);
    let new_total_principal = treasury.total_principal.saturating_add(amount);
    let required_reserve = new_total_principal.saturating_mul(treasury.reserve_ratio_bps as i128) / 10_000;
    if treasury.reserve_balance < required_reserve {
        return Err(AjoError::InsufficientReserve);
    }

    // Pull tokens from the owner into this contract.
    let token_client = token::Client::new(env, &token);
    token_client.transfer(&owner, &env.current_contract_address(), &amount);

    // Compute and lock in dynamic APR at deposit time
    let apr_bps = calculate_dynamic_apr(treasury.total_principal, treasury.reserve_balance, &tier);
    treasury.total_principal = new_total_principal;
    save_treasury(env, &treasury);

    // If SEP-41 receipt token is configured for this tier, mint amount units to depositor
    if let Some(receipt_token_addr) = get_vault_token(env, &tier) {
        let receipt_client = token::StellarAssetClient::new(env, &receipt_token_addr);
        receipt_client.mint(&owner, &amount);
    }

    let deposit_ledger = env.ledger().sequence();
    let maturity_ledger = if lock_ledgers == 0 {
        0 // Flex: always claimable
    } else {
        deposit_ledger + lock_ledgers
    };

    let vault_id = increment_vault_id(env);
    save_vault(
        env,
        vault_id,
        &VaultDeposit {
            owner,
            token,
            principal: amount,
            tier,
            deposit_ledger,
            maturity_ledger,
            claimed: false,
            apr_bps,
        },
    );

    Ok(vault_id)
}

/// Claim principal plus accrued yield from vault `vault_id`.
///
/// If a receipt token contract is registered, burns `principal` receipt tokens from caller
/// (allowing third-party claims by anyone holding the receipt token).
/// Yield is calculated using the locked-in dynamic APR. Principal & yield are deducted from treasury.
pub fn claim_vault(env: &Env, vault_id: u32, caller: Address) -> Result<i128, AjoError> {
    caller.require_auth();

    let mut vault = load_vault(env, vault_id)?;

    if vault.claimed {
        return Err(AjoError::VaultAlreadyClaimed);
    }

    let current_ledger = env.ledger().sequence();

    // Flex vaults (maturity_ledger == 0) are immediately claimable.
    if vault.maturity_ledger > 0 && current_ledger < vault.maturity_ledger {
        return Err(AjoError::VaultLocked);
    }

    // Receipt token handling: if receipt token is set, burn vault.principal from caller
    if let Some(receipt_token_addr) = get_vault_token(env, &vault.tier) {
        let receipt_client = token::Client::new(env, &receipt_token_addr);
        let balance = receipt_client.balance(&caller);
        if balance < vault.principal {
            return Err(AjoError::ReceiptBurnFailed);
        }
        receipt_client.burn(&caller, &vault.principal);
    } else {
        // When receipt token is not configured, caller must be original owner
        if vault.owner != caller {
            return Err(AjoError::Unauthorized);
        }
    }

    let ledgers_held = current_ledger.saturating_sub(vault.deposit_ledger);
    let yield_earned = compute_yield(vault.principal, vault.apr_bps as i128, ledgers_held);
    let total = vault.principal + yield_earned;

    // Deduct payout from treasury
    let mut treasury = get_treasury(env);
    treasury.total_principal = treasury.total_principal.saturating_sub(vault.principal);
    treasury.reserve_balance = treasury.reserve_balance.saturating_sub(yield_earned);
    save_treasury(env, &treasury);

    let token_client = token::Client::new(env, &vault.token);
    token_client.transfer(&env.current_contract_address(), &caller, &total);

    vault.claimed = true;
    save_vault(env, vault_id, &vault);

    Ok(total)
}

/// Return the stored details for vault `vault_id`.
pub fn get_vault_info(env: &Env, vault_id: u32) -> Result<VaultDeposit, AjoError> {
    load_vault(env, vault_id)
}

/// Preview the yield that would be credited if `vault_id` were claimed now.
pub fn preview_yield(env: &Env, vault_id: u32) -> Result<i128, AjoError> {
    let vault = load_vault(env, vault_id)?;
    if vault.claimed {
        return Ok(0);
    }
    let current_ledger = env.ledger().sequence();
    let ledgers_held = current_ledger.saturating_sub(vault.deposit_ledger);
    Ok(compute_yield(vault.principal, vault.apr_bps as i128, ledgers_held))
}
