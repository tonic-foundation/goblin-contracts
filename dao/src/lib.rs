mod dao_impl;

use dao_impl::*;
use std::collections::{HashMap, HashSet};

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, Gas, PanicOnDefault, Promise};

const TGAS_GET_NFT_TOKENS: u64 = 50;
const TGAS_GET_DAO_POLICY: u64 = 50;

#[derive(BorshDeserialize, BorshSerialize, Clone)]
enum MembershipType {
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

    proposal_owners: HashMap<AccountId, MembershipType>,
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
            proposal_owners: HashMap::new(),
        }
    }

    /// Synchronize NFT owners. Removes those ones who don't have NFT anymore,
    /// insert new owners. Return `true` if NFT owners are fully synchronized.
    pub fn sync_nft_owners(&mut self) -> Promise {
        assert_eq!(self.owner_id, env::predecessor_account_id());

        let ext_self = Self::ext(env::current_account_id());
        let gas = Gas::ONE_TERA * TGAS_GET_NFT_TOKENS;

        Promise::new(self.nft_contract_id.clone())
            .function_call("nft_owners".into(), vec![], 0, gas)
            .then(ext_self.handle_nft_owners_sync())
    }

    #[private]
    pub fn handle_nft_owners_sync(&mut self, #[callback] owners: HashSet<AccountId>) -> bool {
        let owners_to_remove: Vec<AccountId> = self
            .nft_owners
            .difference(&owners)
            .map(Clone::clone)
            .collect();

        self.remove_owners(owners_to_remove);

        let owners_to_add: Vec<AccountId> = owners
            .difference(&self.nft_owners)
            .map(Clone::clone)
            .collect();

        self.nft_owners.extend(owners_to_add);

        owners.len() == self.nft_owners.len()
    }

    /// Synchronize DAO members and NFT owners. Save those ones who need to be added or removed.
    pub fn sync_dao_owners(&mut self) -> Promise {
        assert_eq!(self.owner_id, env::predecessor_account_id());

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

        let owners_to_add: Vec<AccountId> = self
            .nft_owners
            .difference(&members)
            .map(Clone::clone)
            .collect();

        let owners_to_remove: Vec<AccountId> = members
            .difference(&self.nft_owners)
            .map(Clone::clone)
            .collect();

        self.extend_owners_for_proposal(owners_to_add, MembershipType::Add);
        self.extend_owners_for_proposal(owners_to_remove, MembershipType::Remove);
    }

    pub fn clear_nft_owners(&mut self) -> usize {
        assert_eq!(self.owner_id, env::predecessor_account_id());
        self.nft_owners.clear();
        self.nft_owners.len()
    }

    pub fn get_nft_owners(&self) -> HashSet<AccountId> {
        self.nft_owners.clone()
    }

    pub fn set_dao_role(&mut self, role: String) {
        assert_eq!(self.owner_id, env::predecessor_account_id());
        self.dao_owners_role = role;
    }

    pub fn owners_len(&self) -> usize {
        self.nft_owners.len()
    }

    pub fn remove_owners(&mut self, users: Vec<AccountId>) {
        assert_eq!(self.owner_id, env::predecessor_account_id());
        for user in users {
            self.nft_owners.remove(&user);
        }
    }

    fn extend_owners_for_proposal(
        &mut self,
        owners: Vec<AccountId>,
        membership_type: MembershipType,
    ) {
        self.proposal_owners.extend(
            owners
                .iter()
                .map(|owner| (owner.clone(), membership_type.clone())),
        );
    }
}
