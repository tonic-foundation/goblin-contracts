use near_contract_standards::non_fungible_token::{
    core::NonFungibleTokenResolver, events::NftTransfer, refund_approved_account_ids,
};
use near_sdk::{assert_one_yocto, json_types::U128, PromiseResult};

use crate::*;

#[derive(Serialize, Deserialize, Default)]
#[serde(crate = "near_sdk::serde")]
pub struct Payout {
    pub payout: HashMap<AccountId, U128>,
}

#[near_bindgen]
impl NonFungibleTokenCore for Contract {
    #[payable]
    fn nft_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
    ) {
        self.tokens
            .nft_transfer(receiver_id.clone(), token_id, approval_id, memo);
        self.update_owners_map(&env::predecessor_account_id(), receiver_id);
    }

    #[payable]
    fn nft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<bool> {
        self.tokens
            .nft_transfer_call(receiver_id, token_id, approval_id, memo, msg)
    }

    fn nft_token(&self, token_id: TokenId) -> Option<Token> {
        self.tokens.nft_token(token_id)
    }
}

#[near_bindgen]
impl NonFungibleTokenResolver for Contract {
    #[private]
    /// Returns true if token was successfully transferred to `receiver_id`.
    fn nft_resolve_transfer(
        &mut self,
        previous_owner_id: AccountId,
        receiver_id: AccountId,
        token_id: TokenId,
        approved_account_ids: Option<HashMap<AccountId, u64>>,
    ) -> bool {
        // Get whether token should be returned
        let must_revert = match env::promise_result(0) {
            PromiseResult::NotReady => env::abort(),
            PromiseResult::Successful(value) => {
                if let Ok(yes_or_no) = near_sdk::serde_json::from_slice::<bool>(&value) {
                    yes_or_no
                } else {
                    true
                }
            }
            PromiseResult::Failed => true,
        };

        // if call succeeded, return early
        if !must_revert {
            self.update_owners_map(&previous_owner_id, receiver_id.clone());
            return true;
        }

        // OTHERWISE, try to set owner back to previous_owner_id and restore approved_account_ids

        // Check that receiver didn't already transfer it away or burn it.
        if let Some(current_owner) = self.tokens.owner_by_id.get(&token_id) {
            if current_owner != receiver_id {
                self.check_old_owner_in_map(&previous_owner_id);
                self.check_old_owner_in_map(&receiver_id);
                // The token is not owned by the receiver anymore. Can't return it.
                return true;
            }
        } else {
            // The token was burned and doesn't exist anymore.
            // Refund storage cost for storing approvals to original owner and return early.
            if let Some(approved_account_ids) = approved_account_ids {
                refund_approved_account_ids(previous_owner_id.clone(), &approved_account_ids);
            }
            self.check_old_owner_in_map(&previous_owner_id);
            self.check_old_owner_in_map(&receiver_id);
            return true;
        };

        self.tokens
            .internal_transfer_unguarded(&token_id, &receiver_id, &previous_owner_id);

        // If using Approval Management extension,
        // 1. revert any approvals receiver already set, refunding storage costs
        // 2. reset approvals to what previous owner had set before call to nft_transfer_call
        if let Some(by_id) = &mut self.tokens.approvals_by_id {
            if let Some(receiver_approvals) = by_id.get(&token_id) {
                refund_approved_account_ids(receiver_id.clone(), &receiver_approvals);
            }
            if let Some(previous_owner_approvals) = approved_account_ids {
                by_id.insert(&token_id, &previous_owner_approvals);
            }
        }

        emit_transfer(&receiver_id, &previous_owner_id, &token_id, None, None);
        false
    }
}

#[near_bindgen]
#[allow(unused_variables)]
impl Contract {
    pub fn nft_payout(&self, token_id: TokenId, balance: U128, max_len_payout: u32) -> Payout {
        let owner_id = self.tokens.owner_by_id.get(&token_id).expect("No token id");

        let mut payout = Payout::default();
        payout.payout.insert(owner_id, balance);
        payout
    }

    #[payable]
    pub fn nft_transfer_payout(
        &mut self,
        receiver_id: AccountId,
        token_id: String,
        approval_id: u64,
        balance: U128,
        max_len_payout: u32,
    ) -> HashMap<AccountId, U128> {
        assert_one_yocto();

        let owner_id = self.tokens.owner_by_id.get(&token_id).expect("No token id");
        self.tokens.nft_transfer(
            receiver_id.clone(),
            token_id.clone(),
            Some(approval_id),
            None,
        );

        let mut result = HashMap::new();
        result.insert(owner_id, balance);
        result
    }
}

/// Used when an NFT is transferred using `nft_transfer_call`. This trait is implemented on the receiving contract, not on the NFT contract.
#[ext_contract(ext_nft_receiver)]
pub trait NonFungibleTokenReceiver {
    /// Take some action after receiving a non-fungible token
    ///
    /// Requirements:
    /// * Contract MUST restrict calls to this function to a set of whitelisted NFT
    ///   contracts
    ///
    /// Arguments:
    /// * `sender_id`: the sender of `nft_transfer_call`
    /// * `previous_owner_id`: the account that owned the NFT prior to it being
    ///   transferred to this contract, which can differ from `sender_id` if using
    ///   Approval Management extension
    /// * `token_id`: the `token_id` argument given to `nft_transfer_call`
    /// * `msg`: information necessary for this contract to know how to process the
    ///   request. This may include method names and/or arguments.
    ///
    /// Returns true if token should be returned to `sender_id`
    fn nft_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_id: AccountId,
        token_id: TokenId,
        msg: String,
    ) -> PromiseOrValue<bool>;
}

fn emit_transfer(
    owner_id: &AccountId,
    receiver_id: &AccountId,
    token_id: &str,
    sender_id: Option<&AccountId>,
    memo: Option<String>,
) {
    NftTransfer {
        old_owner_id: owner_id,
        new_owner_id: receiver_id,
        token_ids: &[token_id],
        authorized_id: sender_id.filter(|sender_id| *sender_id == owner_id),
        memo: memo.as_deref(),
    }
    .emit();
}
