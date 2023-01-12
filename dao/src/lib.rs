mod dao_impl;

use dao_impl::*;
use near_sdk::serde::Serialize;
use near_sdk::serde_json::json;
use std::collections::{HashMap, HashSet};

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, Gas, PanicOnDefault, Promise};

const TGAS_GET_NFT_TOKENS: u64 = 20;
const TGAS_GET_DAO_POLICY: u64 = 20;
const TGAS_ADD_PROPOSAL: u64 = 10;
const MAX_PROPOSALS_PER_CALL: usize = 15;

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
    nft_owners: HashSet<AccountId>,

    dao_account_id: AccountId,
    dao_owners_role: String,

    members_to_update: HashMap<AccountId, MembershipType>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId, nft_contract_id: AccountId, dao_account_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            owner_id,
            nft_contract_id,
            nft_owners: HashSet::new(),
            dao_account_id,
            dao_owners_role: String::new(),
            members_to_update: HashMap::new(),
        }
    }

    /// Synchronize NFT owners. Removes those ones who don't have NFT anymore,
    /// insert new owners. Return `true` if NFT owners are fully synchronized.
    pub fn sync_nft_owners(&mut self) -> Promise {
        self.assert_owner();

        let ext_self = Self::ext(env::current_account_id());
        let gas = Gas::ONE_TERA * TGAS_GET_NFT_TOKENS;

        Promise::new(self.nft_contract_id.clone())
            .function_call("nft_owners".into(), vec![], 0, gas)
            .then(ext_self.handle_nft_owners_sync())
    }

    #[private]
    pub fn handle_nft_owners_sync(&mut self, #[callback] owners: HashSet<AccountId>) -> bool {
        let owners_to_remove = difference(&self.nft_owners, &owners);
        let owners_to_add = difference(&owners, &self.nft_owners);

        self.remove_owners(owners_to_remove);
        self.nft_owners.extend(owners_to_add);

        owners.len() == self.nft_owners.len()
    }

    /// Synchronize DAO members and NFT owners. Save those ones who need to be added or removed.
    pub fn sync_dao_owners(&mut self) -> Promise {
        self.assert_owner();

        let ext_self = Self::ext(env::current_account_id());
        let gas = Gas::ONE_TERA * TGAS_GET_DAO_POLICY;

        Promise::new(self.dao_account_id.clone())
            .function_call("get_policy".into(), vec![], 0, gas)
            .then(ext_self.handle_dao_owners_sync())
    }

    #[private]
    pub fn handle_dao_owners_sync(&mut self, #[callback] policy: Policy) {
        let role_kind = &policy
            .roles
            .iter()
            .find(|role| role.name == self.dao_owners_role)
            .expect("Role not found")
            .kind;

        let members = match role_kind {
            RoleKind::Group(members) => members,
            _ => env::panic_str("Wrong role kind"),
        };

        let owners_to_add = difference(&self.nft_owners, &members);
        let owners_to_remove = difference(&members, &self.nft_owners);

        self.extend_owners_for_proposal(owners_to_add, MembershipType::Add);
        self.extend_owners_for_proposal(owners_to_remove, MembershipType::Remove);
    }

    /// Create proposals for owners that should be added to or removed from DAO members
    pub fn create_proposals(&mut self) {
        self.assert_owner();

        let members = self.members_to_update.clone();
        let owners_to_update = members.iter().take(MAX_PROPOSALS_PER_CALL);

        for (owner_id, status) in owners_to_update {
            self.members_to_update.remove(owner_id);

            let (description, kind) = match status {
                MembershipType::Add => {
                    if !self.nft_owners.contains(owner_id) {
                        continue;
                    }
                    ("Add DAO member", "AddMemberToRole")
                }
                MembershipType::Remove => {
                    if self.nft_owners.contains(owner_id) {
                        continue;
                    }
                    ("Remove DAO member", "RemoveMemberFromRole")
                }
            };

            let gas = Gas::ONE_TERA * TGAS_ADD_PROPOSAL;
            let args = self.proposal_args(description, owner_id, kind);

            Promise::new(self.dao_account_id.clone()).function_call(
                "add_proposal".into(),
                args,
                0,
                gas,
            );
        }
    }

    pub fn clear_nft_owners(&mut self) -> usize {
        self.assert_owner();
        self.nft_owners.clear();
        self.nft_owners.len()
    }

    pub fn get_nft_owners(&self) -> HashSet<AccountId> {
        self.nft_owners.clone()
    }

    pub fn owners_len(&self) -> usize {
        self.nft_owners.len()
    }

    pub fn set_dao_role(&mut self, role: String) {
        self.assert_owner();
        self.dao_owners_role = role;
    }

    pub fn remove_owners(&mut self, users: Vec<AccountId>) {
        self.assert_owner();
        for user in users {
            self.nft_owners.remove(&user);
        }
    }

    pub fn get_members_to_update(&self) -> HashMap<AccountId, MembershipType> {
        self.members_to_update.clone()
    }

    pub fn members_to_update_len(&self) -> usize {
        self.members_to_update.len()
    }

    fn extend_owners_for_proposal(
        &mut self,
        owners: Vec<AccountId>,
        membership_type: MembershipType,
    ) {
        self.members_to_update.extend(
            owners
                .iter()
                .map(|owner| (owner.clone(), membership_type.clone())),
        );
    }

    fn assert_owner(&self) {
        assert_eq!(self.owner_id, env::predecessor_account_id());
    }

    fn proposal_args(&self, description: &str, owner_id: &AccountId, kind: &str) -> Vec<u8> {
        json!({
          "proposal": {
            "description": description,
            "kind": {
                kind: {
                "member_id": owner_id,
                "role": self.dao_owners_role
              }
            }
          }
        })
        .to_string()
        .into_bytes()
    }
}

/// Returns the accounts which are presented in first set and are absent in second one
fn difference(first_set: &HashSet<AccountId>, second_set: &HashSet<AccountId>) -> Vec<AccountId> {
    first_set
        .difference(&second_set)
        .map(Clone::clone)
        .collect()
}
