//! `GET /build` — Metis-only raw swap instructions with **0 Jupiter fees**.
//!
//! Unlike `/order`, `/build` returns the per-instruction breakdown plus an ALT
//! map and a fresh blockhash; the caller is expected to assemble a
//! `VersionedTransaction` (see [`assemble_transaction`]).
//!
//! Transactions produced from `/build` cannot be submitted via `/execute` —
//! land them via your own RPC / Jito stack or via `/tx/v1/submit`.

use crate::serde_helpers::{
    field_as_string,
    instruction::{InstructionInternal, PubkeyInternal},
    option_field_as_string,
};
use crate::v2::common::{PlatformFeeV2, RoutePlanStepV2, SwapModeV2};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    address_lookup_table::AddressLookupTableAccount,
    hash::Hash,
    instruction::Instruction,
    message::{v0::Message as MessageV0, VersionedMessage},
    pubkey::Pubkey,
    transaction::VersionedTransaction,
};
use std::collections::HashMap;
use std::str::FromStr;

/// Latency / quality knob for `/build`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BuildMode {
    /// Trades quote optimality for reduced latency. Critical for sniping.
    Fast,
    /// Default routing optimisation.
    Default,
}

/// `slippageBps` accepts either a numeric value (0..=10000) or the literal
/// string `"rtse"` (runtime slippage estimation).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlippageBpsV2 {
    Bps(u16),
    Rtse,
}

impl Serialize for SlippageBpsV2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            SlippageBpsV2::Bps(b) => serializer.serialize_u16(*b),
            SlippageBpsV2::Rtse => serializer.serialize_str("rtse"),
        }
    }
}

impl<'de> Deserialize<'de> for SlippageBpsV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Number(u16),
            Text(String),
        }
        match Repr::deserialize(deserializer)? {
            Repr::Number(n) => Ok(SlippageBpsV2::Bps(n)),
            Repr::Text(s) if s == "rtse" => Ok(SlippageBpsV2::Rtse),
            Repr::Text(s) => Err(serde::de::Error::custom(format!(
                "expected number or \"rtse\", got {s:?}"
            ))),
        }
    }
}

/// `computeUnitPricePercentile` accepts either a named percentile or a raw
/// number (0..=10000).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(untagged)]
pub enum ComputeUnitPricePercentile {
    Number(u32),
    Named(ComputeUnitPricePercentileNamed),
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ComputeUnitPricePercentileNamed {
    /// 25th percentile.
    Medium,
    /// 50th percentile.
    High,
    /// 75th percentile.
    VeryHigh,
}

/// Query parameters for `GET /build`.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BuildRequest {
    #[serde(with = "field_as_string")]
    pub input_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub output_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub amount: u64,
    /// Required for `/build`.
    #[serde(with = "field_as_string")]
    pub taker: Pubkey,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<BuildMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swap_mode: Option<SwapModeV2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slippage_bps: Option<SlippageBpsV2>,

    /// Positive include-list. Mutually exclusive with `exclude_dexes`.
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "string_csv::serialize"
    )]
    pub dexes: Option<Vec<String>>,
    /// Mutually exclusive with `dexes`.
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "string_csv::serialize"
    )]
    pub exclude_dexes: Option<Vec<String>>,

    /// Integrator fee in bps, 0..=10000. Requires `fee_account`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_fee_bps: Option<u16>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "option_field_as_string"
    )]
    pub fee_account: Option<Pubkey>,

    /// 1..=64, default 64.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_accounts: Option<u8>,

    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "option_field_as_string"
    )]
    pub payer: Option<Pubkey>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrap_and_unwrap_sol: Option<bool>,

    /// Mutually exclusive with `native_destination_account`.
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "option_field_as_string"
    )]
    pub destination_token_account: Option<Pubkey>,
    /// Mutually exclusive with `destination_token_account`.
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "option_field_as_string"
    )]
    pub native_destination_account: Option<Pubkey>,

    /// 1..=300, default 150.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blockhash_slots_to_expiry: Option<u16>,

    /// Jupiter Beam tip in lamports (only relevant when submitting via
    /// `/tx/v1/submit`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tip_amount: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub compute_unit_price_percentile: Option<ComputeUnitPricePercentile>,
}

impl BuildRequest {
    /// Constructor with the three required fields. All other knobs are set to
    /// `None`.
    pub fn new(input_mint: Pubkey, output_mint: Pubkey, amount: u64, taker: Pubkey) -> Self {
        Self {
            input_mint,
            output_mint,
            amount,
            taker,
            mode: None,
            swap_mode: None,
            slippage_bps: None,
            dexes: None,
            exclude_dexes: None,
            platform_fee_bps: None,
            fee_account: None,
            max_accounts: None,
            payer: None,
            wrap_and_unwrap_sol: None,
            destination_token_account: None,
            native_destination_account: None,
            blockhash_slots_to_expiry: None,
            tip_amount: None,
            compute_unit_price_percentile: None,
        }
    }
}

/// Raw response of `GET /build`. Use [`BuildInstructions::from`] to convert
/// into typed Solana SDK structures.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BuildResponse {
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
    pub route_plan: Vec<RoutePlanStepV2>,

    #[serde(default)]
    pub platform_fee: Option<PlatformFeeV2>,

    #[serde(default)]
    pub compute_budget_instructions: Vec<InstructionInternal>,
    #[serde(default)]
    pub setup_instructions: Vec<InstructionInternal>,
    pub swap_instruction: InstructionInternal,
    #[serde(default)]
    pub cleanup_instruction: Option<InstructionInternal>,
    #[serde(default)]
    pub other_instructions: Vec<InstructionInternal>,
    #[serde(default)]
    pub tip_instruction: Option<InstructionInternal>,

    /// Raw Jupiter shape: `{ "<ALT pubkey>": ["<addr>", ...] }`. Keys are
    /// strings in JSON; converted to typed `Pubkey` in [`BuildInstructions`].
    #[serde(default)]
    pub addresses_by_lookup_table_address: HashMap<String, Vec<PubkeyInternal>>,

    pub blockhash_with_metadata: BlockhashWithMetadata,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BlockhashWithMetadata {
    /// Jupiter emits the blockhash as a 32-byte JSON array, **not** base58.
    pub blockhash: [u8; 32],
    /// `/build` returns this as a JSON number (unlike `/order` which returns a
    /// string).
    pub last_valid_block_height: u64,
}

/// Typed view over a `/build` response, ready to be passed into
/// [`assemble_transaction`].
#[derive(Debug, Clone)]
pub struct BuildInstructions {
    pub compute_budget: Vec<Instruction>,
    pub setup: Vec<Instruction>,
    pub swap: Instruction,
    pub cleanup: Option<Instruction>,
    pub other: Vec<Instruction>,
    pub tip: Option<Instruction>,
    pub address_lookup_tables: HashMap<Pubkey, Vec<Pubkey>>,
    pub blockhash: Hash,
    pub last_valid_block_height: u64,
}

impl TryFrom<BuildResponse> for BuildInstructions {
    type Error = anyhow::Error;

    fn try_from(value: BuildResponse) -> Result<Self> {
        let address_lookup_tables = value
            .addresses_by_lookup_table_address
            .into_iter()
            .map(|(k, v)| {
                let alt = Pubkey::from_str(&k)
                    .map_err(|e| anyhow!("invalid ALT pubkey {k:?}: {e:?}"))?;
                let addrs = v.into_iter().map(|p| p.0).collect::<Vec<_>>();
                Ok::<_, anyhow::Error>((alt, addrs))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        Ok(Self {
            compute_budget: value
                .compute_budget_instructions
                .into_iter()
                .map(Into::into)
                .collect(),
            setup: value
                .setup_instructions
                .into_iter()
                .map(Into::into)
                .collect(),
            swap: value.swap_instruction.into(),
            cleanup: value.cleanup_instruction.map(Into::into),
            other: value
                .other_instructions
                .into_iter()
                .map(Into::into)
                .collect(),
            tip: value.tip_instruction.map(Into::into),
            address_lookup_tables,
            blockhash: Hash::new_from_array(value.blockhash_with_metadata.blockhash),
            last_valid_block_height: value.blockhash_with_metadata.last_valid_block_height,
        })
    }
}

/// Assemble a v0 [`VersionedTransaction`] (with empty placeholder signatures)
/// from `/build` instructions.
///
/// Instruction order: `compute_budget → setup → swap → cleanup? → other → tip?`.
/// Caller is responsible for signing the resulting transaction.
pub fn assemble_transaction(
    instructions: &BuildInstructions,
    payer: &Pubkey,
) -> Result<VersionedTransaction> {
    let mut all_ixs: Vec<Instruction> = Vec::with_capacity(
        instructions.compute_budget.len()
            + instructions.setup.len()
            + 1
            + instructions.cleanup.is_some() as usize
            + instructions.other.len()
            + instructions.tip.is_some() as usize,
    );
    all_ixs.extend(instructions.compute_budget.iter().cloned());
    all_ixs.extend(instructions.setup.iter().cloned());
    all_ixs.push(instructions.swap.clone());
    if let Some(cleanup) = &instructions.cleanup {
        all_ixs.push(cleanup.clone());
    }
    all_ixs.extend(instructions.other.iter().cloned());
    if let Some(tip) = &instructions.tip {
        all_ixs.push(tip.clone());
    }

    let alts: Vec<AddressLookupTableAccount> = instructions
        .address_lookup_tables
        .iter()
        .map(|(key, addresses)| AddressLookupTableAccount {
            key: *key,
            addresses: addresses.clone(),
        })
        .collect();

    let message = MessageV0::try_compile(payer, &all_ixs, &alts, instructions.blockhash)
        .map_err(|e| anyhow!("MessageV0::try_compile failed: {e:?}"))?;

    Ok(VersionedTransaction {
        signatures: vec![Default::default(); message.header.num_required_signatures as usize],
        message: VersionedMessage::V0(message),
    })
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
