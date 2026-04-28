//! `GET /order` — managed quote and (optionally) ready-to-sign transaction.
//!
//! See `dev.jup.ag/api-reference/swap/order`. The response carries a base64
//! `VersionedTransaction` whenever `taker` is supplied; otherwise the
//! `transaction` field is `null` and the call behaves as a quote-only request.

use crate::serde_helpers::{field_as_string, option_field_as_string};
use crate::v2::common::{
    BroadcastFeeType, OrderMode, PlatformFeeV2, RoutePlanStepV2, Router, SwapModeV2,
};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

/// Query parameters for `GET /order`.
///
/// Any optional parameter being set forces `mode=manual` server-side, which
/// disables gasless flows. To stay in `mode=ultra` send only `input_mint`,
/// `output_mint` and `amount`.
#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct OrderRequest {
    #[serde(with = "field_as_string")]
    pub input_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub output_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub amount: u64,

    /// Pubkey of the wallet that will sign the transaction. Required for
    /// `/execute`. Without it the response `transaction` is `null`.
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "option_field_as_string"
    )]
    pub taker: Option<Pubkey>,

    /// Receiver of the output token. Must differ from `taker`. Disables
    /// JupiterZ when set.
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "option_field_as_string"
    )]
    pub receiver: Option<Pubkey>,

    /// v2 only supports `ExactIn`; left optional so it can be omitted to keep
    /// `mode=ultra`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swap_mode: Option<SwapModeV2>,

    /// 0..=10000. Setting any value forces `mode=manual`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slippage_bps: Option<u16>,

    /// Referral PDA. Requires `referral_fee` to be set. Disables JupiterZ.
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "option_field_as_string"
    )]
    pub referral_account: Option<Pubkey>,

    /// Integrator fee in bps, 50..=255. Requires `referral_account`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referral_fee: Option<u8>,

    /// Integrator gas payer (forces routing → iris only, requires two
    /// signatures on the resulting transaction).
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "option_field_as_string"
    )]
    pub payer: Option<Pubkey>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_fee_lamports: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub jito_tip_lamports: Option<u64>,

    /// Ignored if neither `priority_fee_lamports` nor `jito_tip_lamports` is
    /// set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub broadcast_fee_type: Option<BroadcastFeeType>,

    /// Routers to exclude. Pass `[Jupiterz, Dflow, Okx]` to force Metis-only.
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "routers_csv::serialize"
    )]
    pub exclude_routers: Option<Vec<Router>>,

    /// DEX labels to exclude. Only applied by Metis (`iris`); no-op for other
    /// routers. CSV in the wire format.
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "string_csv::serialize"
    )]
    pub exclude_dexes: Option<Vec<String>>,
}

/// Response body for `GET /order`.
///
/// All fields that may legitimately be missing in particular routing modes
/// (RFQ-only fields, error fields, etc.) are wrapped in `Option`.
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderResponse {
    pub mode: OrderMode,
    pub router: Router,
    /// DEPRECATED, use `router`. Kept for forward compatibility while Jupiter
    /// keeps emitting it.
    #[serde(default)]
    pub swap_type: Option<String>,

    #[serde(with = "field_as_string")]
    pub input_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub output_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub in_amount: u64,
    #[serde(with = "field_as_string")]
    pub out_amount: u64,
    #[serde(with = "field_as_string")]
    pub other_amount_threshold: u64,
    pub swap_mode: SwapModeV2,
    pub slippage_bps: u16,

    #[serde(default)]
    pub in_usd_value: Option<f64>,
    #[serde(default)]
    pub out_usd_value: Option<f64>,
    #[serde(default)]
    pub swap_usd_value: Option<f64>,

    #[serde(default)]
    pub price_impact: f64,
    /// DEPRECATED but still emitted.
    #[serde(default)]
    pub price_impact_pct: Option<String>,

    pub route_plan: Vec<RoutePlanStepV2>,

    /// Missing on RFQ responses.
    #[serde(
        default,
        deserialize_with = "option_field_as_string::deserialize"
    )]
    pub fee_mint: Option<Pubkey>,
    pub fee_bps: u16,
    #[serde(default)]
    pub platform_fee: Option<PlatformFeeV2>,

    #[serde(default)]
    pub signature_fee_lamports: u64,
    #[serde(
        default,
        deserialize_with = "option_field_as_string::deserialize"
    )]
    pub signature_fee_payer: Option<Pubkey>,

    #[serde(default)]
    pub prioritization_fee_lamports: u64,
    #[serde(
        default,
        deserialize_with = "option_field_as_string::deserialize"
    )]
    pub prioritization_fee_payer: Option<Pubkey>,

    #[serde(default)]
    pub rent_fee_lamports: u64,
    #[serde(
        default,
        deserialize_with = "option_field_as_string::deserialize"
    )]
    pub rent_fee_payer: Option<Pubkey>,

    /// Base64-encoded `VersionedTransaction`. `None` when `taker` was not
    /// supplied or when transaction assembly failed (see `error_*` fields).
    #[serde(default)]
    pub transaction: Option<String>,

    /// Only present on responses where `transaction` is populated.
    #[serde(
        default,
        deserialize_with = "option_field_as_string::deserialize"
    )]
    pub last_valid_block_height: Option<u64>,

    #[serde(default)]
    pub gasless: bool,
    /// JIT-style routing optimisation flag emitted by Jupiter.
    #[serde(default)]
    pub jit_optimized: bool,
    /// Only set for RFQ routes (mode=ultra via JupiterZ).
    #[serde(default)]
    pub guaranteed_price: bool,

    pub request_id: String,
    #[serde(default)]
    pub total_time: Option<u64>,

    /// RFQ-only. Present when `router == Jupiterz`.
    #[serde(default)]
    pub quote_id: Option<String>,
    #[serde(
        default,
        deserialize_with = "option_field_as_string::deserialize"
    )]
    pub maker: Option<Pubkey>,
    #[serde(default)]
    pub expire_at: Option<i64>,

    /// Set when transaction assembly failed server-side (taker supplied but
    /// `transaction` is null).
    #[serde(default)]
    pub error_code: Option<i32>,
    #[serde(default)]
    pub error_message: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

impl OrderResponse {
    /// Decode the base64 `transaction` field into a [`VersionedTransaction`].
    ///
    /// Returns an error if `transaction` is `None`, base64 decoding fails, or
    /// bincode deserialization fails.
    pub fn decoded_transaction(&self) -> Result<VersionedTransaction> {
        let raw = self
            .transaction
            .as_ref()
            .ok_or_else(|| anyhow!("OrderResponse has no transaction (taker not supplied or assembly failed)"))?;
        let bytes = base64::decode(raw)
            .map_err(|e| anyhow!("base64 decode error: {e:?}"))?;
        bincode::deserialize::<VersionedTransaction>(&bytes)
            .map_err(|e| anyhow!("bincode deserialize error: {e:?}"))
    }
}

mod routers_csv {
    use crate::v2::common::Router;
    use serde::Serializer;

    pub fn serialize<S>(value: &Option<Vec<Router>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(v) if !v.is_empty() => {
                let joined = v
                    .iter()
                    .map(Router::as_str)
                    .collect::<Vec<_>>()
                    .join(",");
                serializer.serialize_str(&joined)
            }
            _ => serializer.serialize_none(),
        }
    }
}

mod string_csv {
    use serde::Serializer;

    pub fn serialize<S>(value: &Option<Vec<String>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(v) if !v.is_empty() => serializer.serialize_str(&v.join(",")),
            _ => serializer.serialize_none(),
        }
    }
}
