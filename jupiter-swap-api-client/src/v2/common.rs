//! Common v2 types shared between `/order` and `/build`.

use crate::serde_helpers::field_as_string;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// Per-AMM swap leg as reported by v2 `routePlan[*].swapInfo`.
///
/// Unlike v6 `SwapInfo`, v2 omits `feeAmount` and `feeMint` from the route
/// plan. Aggregate fee data lives in the top-level `feeBps` / `platformFee`
/// fields on the response.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SwapInfoV2 {
    #[serde(with = "field_as_string")]
    pub amm_key: Pubkey,
    pub label: String,
    #[serde(with = "field_as_string")]
    pub input_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub output_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub in_amount: u64,
    #[serde(with = "field_as_string")]
    pub out_amount: u64,
}

/// Routing engine that produced the quote.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Router {
    /// Metis on-chain aggregator. Only router that indexes fresh pools.
    Iris,
    /// JupiterZ off-chain RFQ network.
    Jupiterz,
    /// Dflow order-flow auction.
    Dflow,
    /// OKX aggregator.
    Okx,
}

impl Router {
    /// Lower-case identifier as used in the `excludeRouters` query parameter.
    pub fn as_str(&self) -> &'static str {
        match self {
            Router::Iris => "iris",
            Router::Jupiterz => "jupiterz",
            Router::Dflow => "dflow",
            Router::Okx => "okx",
        }
    }
}

/// Order routing mode reported by `/order`.
///
/// `Ultra` is returned only when **no** optional parameter is supplied to
/// `/order` (no `slippageBps`, no `excludeRouters`, no referral, etc.).
/// Any optional parameter forces `Manual` mode and disables gasless flows.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OrderMode {
    Ultra,
    Manual,
}

/// Strategy used by Jupiter when interpreting `priorityFeeLamports` /
/// `jitoTipLamports`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum BroadcastFeeType {
    /// Treat the value as an upper bound; Jupiter may pick something lower.
    MaxCap,
    /// Use exactly the supplied value.
    ExactFee,
}

/// Swap mode supported by v2. Only `ExactIn` is accepted by `/order` and
/// `/build`; the v6 `ExactOut` mode is not available in v2.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SwapModeV2 {
    #[default]
    ExactIn,
}

/// One step of the v2 route plan.
///
/// v2 adds `bps` and `usdValue` next to v6's `percent`, and trims the nested
/// `swapInfo` shape (no per-leg fee fields). See [`SwapInfoV2`].
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RoutePlanStepV2 {
    pub swap_info: SwapInfoV2,
    #[serde(default)]
    pub percent: Option<u8>,
    #[serde(default)]
    pub bps: u16,
    #[serde(default)]
    pub usd_value: f64,
}

/// Integrator fee block populated when `referralAccount` (`/order`) or
/// `platformFeeBps` (`/build`) is set.
///
/// `amount` is only populated for RFQ routes / when an integrator referral is
/// active; `fee_mint` is missing on some RFQ responses. Both stay `Option`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlatformFeeV2 {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::serde_helpers::option_field_as_string"
    )]
    pub amount: Option<u64>,
    pub fee_bps: u16,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::serde_helpers::option_field_as_string"
    )]
    pub fee_mint: Option<Pubkey>,
}
