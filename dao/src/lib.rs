use std::collections::HashSet;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, Gas, PanicOnDefault, Promise};

const TGAS_GET_NFT_TOKENS: u64 = 100;

pub fn goblins_id() -> AccountId {
    if cfg!(feature = "mainnet") {
        "" // TODO insert our contract address
    } else {
        "dev-1673361753463-54922973878649" // Test NFT contract with 2k owners
    }
    .parse()
    .unwrap()
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    owner_id: AccountId,

    nft_owners: HashSet<AccountId>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            owner_id,
            nft_owners: HashSet::new(),
        }
    }

    /// Synchronizing NFT owners. Removes those ones who don't have NFT anymore,
    /// insert new owners. Return `true` if NFT owners are fully synchronized.
    pub fn sync_nft_owners(&mut self) -> Promise {
        assert_eq!(self.owner_id, env::predecessor_account_id());

        let ext_self = Self::ext(env::current_account_id());
        let gas = Gas::ONE_TERA * TGAS_GET_NFT_TOKENS;

        Promise::new(goblins_id())
            .function_call("nft_owners".into(), vec![], 0, gas)
            .then(ext_self.handle_owners_sync())
    }

    #[private]
    pub fn handle_owners_sync(&mut self, #[callback] owners: HashSet<AccountId>) -> bool {
        let owners_to_remove: Vec<AccountId> = self
            .nft_owners
            .difference(&owners)
            .map(Clone::clone)
            .collect();

        self.remove_goblins(owners_to_remove);

        let owners_to_add: Vec<AccountId> = owners
            .difference(&self.nft_owners)
            .map(Clone::clone)
            .collect();

        self.nft_owners.extend(owners_to_add);

        self.nft_owners.len() == self.nft_owners.len()
    }

    pub fn clear_nft_owners(&mut self) -> usize {
        assert_eq!(self.owner_id, env::predecessor_account_id());
        self.nft_owners.clear();
        self.nft_owners.len()
    }

    pub fn get_nft_owners(&self) -> HashSet<AccountId> {
        self.nft_owners.clone()
    }

    pub fn owners_len(&self) -> usize {
        self.nft_owners.len()
    }

    fn remove_goblins(&mut self, users: Vec<AccountId>) {
        assert_eq!(self.owner_id, env::predecessor_account_id());
        for user in users {
            self.nft_owners.remove(&user);
        }
    }
}
