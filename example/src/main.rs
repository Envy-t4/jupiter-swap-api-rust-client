use std::env;

use jupiter_swap_api_client::v2::build::{assemble_transaction, BuildInstructions, BuildMode, BuildRequest};
use jupiter_swap_api_client::v2::common::Router;
use jupiter_swap_api_client::v2::order::OrderRequest;
use jupiter_swap_api_client::{
    quote::QuoteRequest, swap::SwapRequest, transaction_config::TransactionConfig,
    JupiterSwapApiClient,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey, transaction::VersionedTransaction};
use solana_sdk::{pubkey::Pubkey, signature::NullSigner};
use tokio;

const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
const NATIVE_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");

pub const TEST_WALLET: Pubkey = pubkey!("2AQdpHJ2JpcEgPiATUXjQxA8QmafFegfQwSLWSprPicm"); // Coinbase 2 wallet

#[tokio::main]
async fn main() {
    // EXAMPLE_API selects which surface to demo: v1 (default), v2_order, v2_build.
    let example_api = env::var("EXAMPLE_API").unwrap_or_else(|_| "v1".into());

    match example_api.as_str() {
        "v2_order" => run_v2_order().await,
        "v2_build" => run_v2_build().await,
        _ => run_v1().await,
    }
}

async fn run_v1() {
    let api_base_url = env::var("API_BASE_URL").unwrap_or("https://quote-api.jup.ag/v6".into());
    let api_key = env::var("JUPITER_API_KEY").ok();

    println!("Using base url: {}", api_base_url);

    let jupiter_swap_api_client = match api_key {
        Some(key) => JupiterSwapApiClient::new_with_api_key(api_base_url, key),
        None => JupiterSwapApiClient::new(api_base_url),
    };

    let quote_request = QuoteRequest {
        amount: 1_000_000,
        input_mint: USDC_MINT,
        output_mint: NATIVE_MINT,
        slippage_bps: 50,
        ..QuoteRequest::default()
    };

    let quote_response = jupiter_swap_api_client.quote(&quote_request).await.unwrap();
    println!("{quote_response:#?}");

    let swap_response = jupiter_swap_api_client
        .swap(&SwapRequest {
            user_public_key: TEST_WALLET,
            quote_response: quote_response.clone(),
            config: TransactionConfig::default(),
        })
        .await
        .unwrap();

    println!("Raw tx len: {}", swap_response.swap_transaction.len());

    let versioned_transaction: VersionedTransaction =
        bincode::deserialize(&swap_response.swap_transaction).unwrap();

    let null_signer = NullSigner::new(&TEST_WALLET);
    let signed_versioned_transaction =
        VersionedTransaction::try_new(versioned_transaction.message, &[&null_signer]).unwrap();

    let rpc_client = RpcClient::new("https://api.mainnet-beta.solana.com".into());
    let error = rpc_client
        .send_and_confirm_transaction(&signed_versioned_transaction)
        .await
        .unwrap_err();
    println!("{error}");

    let swap_instructions = jupiter_swap_api_client
        .swap_instructions(&SwapRequest {
            user_public_key: TEST_WALLET,
            quote_response,
            config: TransactionConfig::default(),
        })
        .await
        .unwrap();
    println!("swap_instructions: {swap_instructions:?}");
}

async fn run_v2_order() {
    let api_base_url =
        env::var("API_BASE_URL").unwrap_or("https://lite-api.jup.ag/swap/v2".into());
    let client = match env::var("JUPITER_API_KEY").ok() {
        Some(key) => JupiterSwapApiClient::new_with_api_key(api_base_url.clone(), key),
        None => JupiterSwapApiClient::new(api_base_url.clone()),
    };
    println!("Using base url: {}", api_base_url);

    // Force Metis-only routing (excludes JupiterZ/Dflow/OKX) to keep latency
    // predictable on snipe paths. This pushes the request into manual mode.
    let request = OrderRequest {
        input_mint: NATIVE_MINT,
        output_mint: USDC_MINT,
        amount: 100_000_000,
        taker: Some(TEST_WALLET),
        slippage_bps: Some(50),
        exclude_routers: Some(vec![Router::Jupiterz, Router::Dflow, Router::Okx]),
        ..Default::default()
    };

    let response = client.order(&request).await.unwrap();
    println!(
        "router={:?} mode={:?} feeBps={} out={} requestId={}",
        response.router, response.mode, response.fee_bps, response.out_amount, response.request_id
    );

    if response.transaction.is_some() {
        let tx = response.decoded_transaction().unwrap();
        println!("decoded tx, signatures slots: {}", tx.signatures.len());
    }
}

async fn run_v2_build() {
    let api_base_url =
        env::var("API_BASE_URL").unwrap_or("https://lite-api.jup.ag/swap/v2".into());
    let client = match env::var("JUPITER_API_KEY").ok() {
        Some(key) => JupiterSwapApiClient::new_with_api_key(api_base_url.clone(), key),
        None => JupiterSwapApiClient::new(api_base_url.clone()),
    };
    println!("Using base url: {}", api_base_url);

    let mut request = BuildRequest::new(NATIVE_MINT, USDC_MINT, 100_000_000, TEST_WALLET);
    request.mode = Some(BuildMode::Fast);

    let response = client.build(&request).await.unwrap();
    println!(
        "out={} setup_ixs={} cb_ixs={} alt_count={}",
        response.out_amount,
        response.setup_instructions.len(),
        response.compute_budget_instructions.len(),
        response.addresses_by_lookup_table_address.len(),
    );

    let typed: BuildInstructions = response.try_into().unwrap();
    let tx = assemble_transaction(&typed, &TEST_WALLET).unwrap();
    println!(
        "assembled tx: {} accounts, blockhash valid until slot {}",
        tx.message.static_account_keys().len(),
        typed.last_valid_block_height,
    );
}
