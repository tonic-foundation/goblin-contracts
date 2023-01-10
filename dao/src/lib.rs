use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedSet;
use near_sdk::serde_json::json;
use near_sdk::{env, near_bindgen, AccountId, BorshStorageKey, Gas, PanicOnDefault, Promise};

const TGAS_GET_NFT_TOKENS: u64 = 150;

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    NftOwners,
    Goblins,
}

pub fn goblins_id() -> AccountId {
    if cfg!(feature = "mainnet") {
        "tonic_goblin.enleap.near"
    } else {
        "dev-1673008864122-15827906125219" // Test NFT contract with 2k owners
    }
    .parse()
    .unwrap()
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    owner_id: AccountId,
    /// Actual owners of the contract
    nft_owners: UnorderedSet<AccountId>,
    /// Goblins that take part in DAO staff
    goblins: UnorderedSet<AccountId>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            owner_id,
            nft_owners: UnorderedSet::new(StorageKey::NftOwners),
            goblins: UnorderedSet::new(StorageKey::Goblins),
        }
    }

    pub fn parse_nft_owners(&mut self, from_index: Option<u64>, limit: Option<u64>) -> Promise {
        assert_eq!(self.owner_id, env::predecessor_account_id());

        let ext_self = Self::ext(env::current_account_id());
        let gas = Gas::ONE_TERA * TGAS_GET_NFT_TOKENS;
        let args = json!({ "from_index": from_index, "limit": limit})
            .to_string()
            .into_bytes();

        Promise::new(goblins_id())
            .function_call("nft_owners".into(), args, 0, gas)
            .then(ext_self.handle_goblins_parse())
    }

    #[private]
    pub fn handle_goblins_parse(&mut self, #[callback] owners: Vec<AccountId>) {
        self.nft_owners.extend(owners);
    }

    pub fn clear_nft_owners(&mut self, from_index: Option<u64>, limit: Option<u64>) {
        assert_eq!(self.owner_id, env::predecessor_account_id());
        self.nft_owners.clear();
    }

    pub fn get_goblins(&self, from_index: Option<u64>, limit: Option<u64>) -> Vec<AccountId> {
        let goblins = self.goblins.as_vector();
        let from_index = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(goblins.len());
        (from_index..std::cmp::min(goblins.len(), limit))
            .map(|index| goblins.get(index).unwrap())
            .collect()
    }

    pub fn get_nft_owners(&self, from_index: Option<u64>, limit: Option<u64>) -> Vec<AccountId> {
        let goblins = self.nft_owners.as_vector();
        let from_index = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(goblins.len());
        (from_index..std::cmp::min(goblins.len(), limit))
            .map(|index| goblins.get(index).unwrap())
            .collect()
    }
}
