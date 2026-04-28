use jupiter_swap_api_client::v2::common::{OrderMode, Router};
use jupiter_swap_api_client::v2::order::OrderResponse;

const NO_TAKER: &str = include_str!("fixtures/v2/order_no_taker.json");
const WITH_TAKER: &str = include_str!("fixtures/v2/order_with_taker.json");
const RFQ: &str = include_str!("fixtures/v2/order_rfq.json");

#[test]
fn parses_order_without_taker() {
    let r: OrderResponse = serde_json::from_str(NO_TAKER).expect("deserialize");
    assert_eq!(r.mode, OrderMode::Manual);
    assert_eq!(r.router, Router::Iris);
    assert!(r.transaction.is_none());
    assert!(r.last_valid_block_height.is_none());
    assert_eq!(r.fee_bps, 2);
    assert!(r.fee_mint.is_some());
    assert!(!r.gasless);
    assert!(!r.route_plan.is_empty());
}

#[test]
fn parses_order_with_taker() {
    let r: OrderResponse = serde_json::from_str(WITH_TAKER).expect("deserialize");
    assert_eq!(r.mode, OrderMode::Manual);
    assert_eq!(r.router, Router::Iris);
    assert!(r.transaction.is_some());
    assert_eq!(r.last_valid_block_height, Some(394348955));
    assert_eq!(r.signature_fee_lamports, 5000);
    assert_eq!(r.prioritization_fee_lamports, 512331);
    assert!(r.signature_fee_payer.is_some());

    // Ensures the base64 + bincode round-trip works on a real Jupiter tx.
    let tx = r.decoded_transaction().expect("decode tx");
    assert!(!tx.message.static_account_keys().is_empty());
}

#[test]
fn parses_order_rfq() {
    let r: OrderResponse = serde_json::from_str(RFQ).expect("deserialize");
    assert_eq!(r.mode, OrderMode::Ultra);
    assert_eq!(r.router, Router::Jupiterz);
    assert!(r.guaranteed_price);
    assert_eq!(r.fee_bps, 10);
    assert!(r.quote_id.is_some());
    assert!(r.maker.is_some());
    assert!(r.expire_at.is_none());
    // `feeMint` is absent on RFQ responses.
    assert!(r.fee_mint.is_none());
    let pf = r.platform_fee.expect("rfq has platformFee");
    assert_eq!(pf.fee_bps, 10);
    assert_eq!(pf.amount, Some(1_339_050_587));
    // `feeMint` inside platform_fee is also absent.
    assert!(pf.fee_mint.is_none());
}
