use jupiter_swap_api_client::v2::build::{assemble_transaction, BuildInstructions, BuildResponse};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

const FAST: &str = include_str!("fixtures/v2/build_fast.json");

#[test]
fn parses_build_fast() {
    let r: BuildResponse = serde_json::from_str(FAST).expect("deserialize");
    assert!(r.swap_instruction.accounts.len() > 0);
    assert!(!r.compute_budget_instructions.is_empty());
    assert!(!r.setup_instructions.is_empty());
    assert!(r.tip_instruction.is_none());
    assert_eq!(r.blockhash_with_metadata.blockhash.len(), 32);
    assert!(r.blockhash_with_metadata.last_valid_block_height > 0);
    assert!(!r.addresses_by_lookup_table_address.is_empty());
}

#[test]
fn build_response_converts_to_typed_instructions() {
    let r: BuildResponse = serde_json::from_str(FAST).expect("deserialize");
    let typed: BuildInstructions = r.try_into().expect("convert");
    assert_eq!(typed.compute_budget.len(), 1);
    assert!(!typed.setup.is_empty());
    assert!(typed.tip.is_none());
    assert_eq!(typed.address_lookup_tables.len(), 1);
}

#[test]
fn assemble_transaction_produces_signable_tx() {
    let r: BuildResponse = serde_json::from_str(FAST).expect("deserialize");
    let typed: BuildInstructions = r.try_into().expect("convert");
    let payer = Pubkey::from_str("9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM").unwrap();
    let tx = assemble_transaction(&typed, &payer).expect("assemble");
    // The compiled message must list the payer as the first static account.
    assert_eq!(tx.message.static_account_keys()[0], payer);
    // At least one signature slot is reserved.
    assert!(!tx.signatures.is_empty());
}
