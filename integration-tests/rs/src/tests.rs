use near_units::{parse_gas, parse_near};
use serde_json::json;
use workspaces::{Account, Contract};

const NFT_WASM_FILEPATH: &str = "../../res/non_fungible_token.wasm";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // initiate environemnt
    let worker = workspaces::sandbox().await?;

    // deploy contracts
    let nft_wasm = std::fs::read(NFT_WASM_FILEPATH)?;
    let nft_contract = worker.dev_deploy(&nft_wasm).await?;

    // create accounts
    let owner = worker.root_account().unwrap();
    let alice = owner
        .create_subaccount("alice")
        .initial_balance(parse_near!("30 N"))
        .transact()
        .await?
        .into_result()?;

    // Initialize contracts
    nft_contract
        .call("new")
        .args_json(serde_json::json!({
            "owner_id": owner.id(),
            "metadata": {
                "spec": "nft-1.0.0", 
                "name": "Tonic Greedy Goblins", 
                "symbol": "GGB"
            }
        }))
        .transact()
        .await?;

    // begin tests
    test_simple_approve(&owner, &alice, &nft_contract).await?;
    test_approved_account_transfers_token(&owner, &alice, &nft_contract).await?;
    test_simple_transfer(&owner, &alice, &nft_contract).await?;
    test_enum_total_supply(&nft_contract).await?;
    test_enum_nft_tokens(&nft_contract).await?;
    test_enum_nft_supply_for_owner(&owner, &alice, &nft_contract).await?;
    test_enum_nft_tokens_for_owner(&owner, &alice, &nft_contract).await?;
    Ok(())
}

async fn test_simple_approve(
    owner: &Account,
    user: &Account,
    nft_contract: &Contract,
) -> anyhow::Result<()> {
    owner
        .call(nft_contract.id(), "nft_mint")
        .args_json(json!({
            "token_id": "0",
            "receiver_id": owner.id(),
            "token_metadata": {
                "title": "Olympus Mons",
                "description": "The tallest mountain in the charted solar system",
                "copies": 10000,
            }
        }))
        .deposit(parse_gas!("5950000000000000000000"))
        .transact()
        .await?;

    // root approves alice
    owner
        .call(nft_contract.id(), "nft_approve")
        .args_json(json!({
            "token_id":  "0",
            "account_id": user.id(),
        }))
        .deposit(parse_gas!("5950000000000000000000"))
        .transact()
        .await?;

    let approval_no_id: bool = nft_contract
        .call("nft_is_approved")
        .args_json(json!({
            "token_id":  "0",
            "approved_account_id": user.id()
        }))
        .transact()
        .await?
        .json()?;

    assert!(approval_no_id);

    let approval: bool = nft_contract
        .call("nft_is_approved")
        .args_json(json!({
            "token_id":  "0",
            "approved_account_id": user.id(),
            "approval_id": 1
        }))
        .transact()
        .await?
        .json()?;

    assert!(approval);

    let approval_wrong_id: bool = nft_contract
        .call("nft_is_approved")
        .args_json(json!({
            "token_id":  "0",
            "approved_account_id": user.id(),
            "approval_id": 2
        }))
        .transact()
        .await?
        .json()?;

    assert!(!approval_wrong_id);
    println!("      Passed ✅ test_simple_approve");
    Ok(())
}

async fn test_approved_account_transfers_token(
    owner: &Account,
    user: &Account,
    nft_contract: &Contract,
) -> anyhow::Result<()> {
    use serde_json::Value::String;
    owner
        .call(nft_contract.id(), "nft_transfer")
        .args_json(json!({
            "receiver_id": user.id(),
            "token_id": '0',
            "approval_id": 1,
            "memo": "message for test 3",
        }))
        .deposit(1)
        .transact()
        .await?;

    let token: serde_json::Value = nft_contract
        .call("nft_token")
        .args_json(json!({"token_id": "0"}))
        .transact()
        .await?
        .json()?;
    assert_eq!(token.get("owner_id"), Some(&String(user.id().to_string())));

    println!("      Passed ✅ test_approved_account_transfers_token");
    Ok(())
}

async fn test_simple_transfer(
    owner: &Account,
    user: &Account,
    nft_contract: &Contract,
) -> anyhow::Result<()> {
    use serde_json::Value::String;
    let token: serde_json::Value = nft_contract
        .call("nft_token")
        .args_json(json!({"token_id": "1"}))
        .transact()
        .await?
        .json()?;
    assert_eq!(token.get("owner_id"), Some(&String(owner.id().to_string())));

    owner
        .call(nft_contract.id(), "nft_transfer")
        .args_json(json!({
            "token_id": "1",
            "receiver_id": user.id(),
        }))
        .deposit(1)
        .transact()
        .await?;

    let token: serde_json::Value = nft_contract
        .call("nft_token")
        .args_json(json!({"token_id": "1"}))
        .transact()
        .await?
        .json()?;
    assert_eq!(token.get("owner_id"), Some(&String(user.id().to_string())));

    println!("      Passed ✅ test_simple_transfer");
    Ok(())
}

async fn test_enum_total_supply(
    nft_contract: &Contract,
) -> anyhow::Result<()> {
    let supply: String = nft_contract
        .call("nft_total_supply")
        .args_json(json!({}))
        .transact()
        .await?
        .json()?;
    assert_eq!(supply, "5");

    println!("      Passed ✅ test_enum_total_supply");
    Ok(())
}

async fn test_enum_nft_tokens(
    nft_contract: &Contract,
) -> anyhow::Result<()> {
    let tokens: Vec<serde_json::Value> = nft_contract
        .call("nft_tokens")
        .args_json(json!({}))
        .transact()
        .await?
        .json()?;

    assert_eq!(tokens.len(), 5);

    println!("      Passed ✅ test_enum_nft_tokens");
    Ok(())
}

async fn test_enum_nft_supply_for_owner(
    owner: &Account,
    user: &Account,
    nft_contract: &Contract,
) -> anyhow::Result<()> {
    let owner_tokens: String = nft_contract
        .call("nft_supply_for_owner")
        .args_json(json!({"account_id": owner.id()}))
        .transact()
        .await?
        .json()?;
    assert_eq!(owner_tokens, "1");

    let user_tokens: String = nft_contract
        .call("nft_supply_for_owner")
        .args_json(json!({"account_id": user.id()}))
        .transact()
        .await?
        .json()?;
    assert_eq!(user_tokens, "2");

    println!("      Passed ✅ test_enum_nft_supply_for_owner");
    Ok(())
}

async fn test_enum_nft_tokens_for_owner(
    owner: &Account,
    user: &Account,
    nft_contract: &Contract,
) -> anyhow::Result<()> {
    let tokens: Vec<serde_json::Value> = nft_contract
        .call("nft_tokens_for_owner")
        .args_json(json!({
            "account_id": user.id()
        }))
        .transact()
        .await?
        .json()?;
    assert_eq!(tokens.len(), 2);

    let tokens: Vec<serde_json::Value> = nft_contract
        .call("nft_tokens_for_owner")
        .args_json(json!({
            "account_id": owner.id()
        }))
        .transact()
        .await?
        .json()?;
    assert_eq!(tokens.len(), 1);
    println!("      Passed ✅ test_enum_nft_tokens_for_owner");
    Ok(())
}
