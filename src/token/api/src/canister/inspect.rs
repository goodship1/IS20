use candid::{Nat, Principal};
use ic_storage::IcStorage;

use crate::state::CanisterState;

static PUBLIC_METHODS: &[&str] = &[
    "icrc1_balance_of",
    "decimals",
    "get_holders",
    "get_metadata",
    "get_token_info",
    "get_transaction",
    "get_transactions",
    "get_user_transaction_amount",
    "get_user_transactions",
    "history_size",
    "logo",
    "icrc1_name",
    "owner",
    "icrc1_symbol",
    "icrc1_total_supply",
    "is_test_token",
];

static OWNER_METHODS: &[&str] = &[
    "icrc1_mint",
    "set_auction_period",
    "set_fee",
    "set_fee_to",
    "set_logo",
    "set_min_cycles",
    "set_name",
    "set_symbol",
    "set_owner",
];

static TRANSACTION_METHODS: &[&str] = &["burn", "icrc1_transfer", "transferIncludeFee"];

/// Reason why the method may be accepted.
#[derive(Debug, Clone, Copy)]
pub enum AcceptReason {
    /// The call is a part of the IS20 API and can be performed.
    Valid,
    /// The method isn't a part of the IS20 API, and may require further validation.
    NotIS20Method,
}

/// This function checks if the canister should accept ingress message or not. We allow query
/// calls for anyone, but update calls have different checks to see, if it's reasonable to spend
/// canister cycles on accepting this call. Check the comments in this method for details on
/// the checks for different methods.
pub fn inspect_message(
    state: &CanisterState,
    method: &str,
    caller: Principal,
) -> Result<AcceptReason, &'static str> {
    match method {
        // These are query methods, so no checks are needed.
        #[cfg(feature = "mint_burn")]
        "mint" if state.stats.is_test_token => Ok(AcceptReason::Valid),
        m if PUBLIC_METHODS.contains(&m) => Ok(AcceptReason::Valid),
        // Owner
        m if OWNER_METHODS.contains(&m) && caller == state.stats.owner => Ok(AcceptReason::Valid),
        // Not owner
        m if OWNER_METHODS.contains(&m) => {
            Err("Owner method is called not by an owner. Rejecting.")
        }
        #[cfg(any(feature = "transfer", feature = "mint_burn"))]
        m if TRANSACTION_METHODS.contains(&m) => {
            // These methods requires that the caller have tokens.
            let state = CanisterState::get();
            let state = state.borrow();
            let balances = &state.balances;
            if !balances.0.contains_key(&caller) {
                return Err("Transaction method is not called by a stakeholder. Rejecting.");
            }

            // Anything but the `burn` method
            if caller == state.stats.owner || m != "burn" {
                return Ok(AcceptReason::Valid);
            }

            // It's the `burn` method and the caller isn't the owner.
            let from = ic_cdk::api::call::arg_data::<(Option<Principal>, Nat)>().0;
            if from.is_some() {
                return Err("Only the owner can burn other's tokens. Rejecting.");
            }

            Ok(AcceptReason::Valid)
        }
        "bidCycles" => {
            // We reject this message, because a call with cycles cannot be made through ingress,
            // only from the wallet canister.
            Err("Call with cycles cannot be made through ingress.")
        }
        _ => Ok(AcceptReason::NotIS20Method),
    }
}
