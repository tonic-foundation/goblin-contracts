mod dao_impl;

use dao_impl::*;
use near_sdk::serde::Serialize;
use near_sdk::serde_json::json;
use std::collections::HashSet;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, Gas, PanicOnDefault, Promise};

const TGAS_GET_NFT_TOKENS: u64 = 20;
const TGAS_GET_DAO_POLICY: u64 = 20;
const TGAS_ADD_PROPOSAL: u64 = 40;

#[derive(BorshDeserialize, BorshSerialize, Clone, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub enum MembershipType {
    Add,
    Remove,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    owner_id: AccountId,
    nft_contract_id: AccountId,
    dao_account_id: AccountId,
    dao_owners_role: String,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId, nft_contract_id: AccountId, dao_account_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            owner_id,
            nft_contract_id,
            dao_account_id,
            dao_owners_role: String::new(),
        }
    }

    /// Synchronize NFT owners. Removes those ones who don't have NFT anymore,
    /// insert new owners. Return `true` if NFT owners are fully synchronized.
    pub fn sync_nft_owners(&mut self) -> Promise {
        self.assert_owner();

        let ext_self = Self::ext(env::current_account_id());
        let gas_get_owners = Gas::ONE_TERA * TGAS_GET_NFT_TOKENS;
        let gas_get_policy = Gas::ONE_TERA * TGAS_GET_DAO_POLICY;

        Promise::new(self.nft_contract_id.clone())
            .function_call("nft_owners".into(), vec![], 0, gas_get_owners)
            .and(Promise::new(self.dao_account_id.clone()).function_call(
                "get_policy".into(),
                vec![],
                0,
                gas_get_policy,
            ))
            .then(ext_self.handle_nft_owners_sync())
    }

    #[private]
    pub fn handle_nft_owners_sync(
        &mut self,
        #[callback] owners: HashSet<AccountId>,
        #[callback] mut policy: Policy,
    ) -> Promise {
        policy.update_group_members(owners, self.dao_owners_role.clone());

        let gas = Gas::ONE_TERA * TGAS_ADD_PROPOSAL;
        let args = json!({
          "proposal": {
            "description": "Update DAO members",
            "kind": {
                "ChangePolicy": {
                    "policy": policy
              }
            }
          }
        })
        .to_string()
        .into_bytes();

        Promise::new(self.dao_account_id.clone()).function_call("add_proposal".into(), args, 0, gas)
    }

    pub fn set_dao_role(&mut self, role: String) {
        self.assert_owner();
        self.dao_owners_role = role;
    }

    fn assert_owner(&self) {
        assert_eq!(self.owner_id, env::predecessor_account_id());
    }
}
