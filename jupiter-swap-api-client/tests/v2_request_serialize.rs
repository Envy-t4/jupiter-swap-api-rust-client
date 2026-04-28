use jupiter_swap_api_client::v2::build::{
    BuildMode, BuildRequest, ComputeUnitPricePercentile, ComputeUnitPricePercentileNamed,
    SlippageBpsV2,
};
use jupiter_swap_api_client::v2::common::Router;
use jupiter_swap_api_client::v2::order::OrderRequest;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

fn sol() -> Pubkey {
    Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap()
}
fn usdc() -> Pubkey {
    Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap()
}
fn taker() -> Pubkey {
    Pubkey::from_str("9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM").unwrap()
}

#[test]
fn order_minimal_query_omits_optional_fields() {
    let req = OrderRequest {
        input_mint: sol(),
        output_mint: usdc(),
        amount: 100_000_000,
        ..Default::default()
    };
    let q = serde_qs::to_string(&req).unwrap();
    assert!(q.contains("inputMint=So11111111111111111111111111111111111111112"));
    assert!(q.contains("amount=100000000"));
    // No optional knobs leak into the query string.
    assert!(!q.contains("taker"));
    assert!(!q.contains("slippageBps"));
    assert!(!q.contains("excludeRouters"));
}

#[test]
fn order_serialises_exclude_routers_as_csv() {
    let req = OrderRequest {
        input_mint: sol(),
        output_mint: usdc(),
        amount: 1,
        exclude_routers: Some(vec![Router::Jupiterz, Router::Dflow, Router::Okx]),
        ..Default::default()
    };
    let q = serde_qs::to_string(&req).unwrap();
    assert!(q.contains("excludeRouters=jupiterz%2Cdflow%2Cokx"));
}

#[test]
fn order_serialises_exclude_dexes_as_csv() {
    let req = OrderRequest {
        input_mint: sol(),
        output_mint: usdc(),
        amount: 1,
        exclude_dexes: Some(vec!["Raydium".into(), "Meteora DLMM".into()]),
        ..Default::default()
    };
    let q = serde_qs::to_string(&req).unwrap();
    // serde_qs uses form-urlencoded space (`+`); the comma becomes `%2C`.
    assert!(
        q.contains("excludeDexes=Raydium%2CMeteora+DLMM"),
        "unexpected query: {q}"
    );
}

#[test]
fn build_serialises_slippage_rtse() {
    let req = BuildRequest {
        slippage_bps: Some(SlippageBpsV2::Rtse),
        mode: Some(BuildMode::Fast),
        ..BuildRequest::new(sol(), usdc(), 100_000_000, taker())
    };
    let q = serde_qs::to_string(&req).unwrap();
    assert!(q.contains("slippageBps=rtse"));
    assert!(q.contains("mode=fast"));
    assert!(q.contains("taker="));
}

#[test]
fn build_serialises_compute_unit_percentile_named_and_number() {
    let mut req = BuildRequest::new(sol(), usdc(), 1, taker());
    req.compute_unit_price_percentile =
        Some(ComputeUnitPricePercentile::Named(ComputeUnitPricePercentileNamed::VeryHigh));
    let q = serde_qs::to_string(&req).unwrap();
    assert!(q.contains("computeUnitPricePercentile=veryHigh"));

    req.compute_unit_price_percentile = Some(ComputeUnitPricePercentile::Number(7500));
    let q = serde_qs::to_string(&req).unwrap();
    assert!(q.contains("computeUnitPricePercentile=7500"));
}
