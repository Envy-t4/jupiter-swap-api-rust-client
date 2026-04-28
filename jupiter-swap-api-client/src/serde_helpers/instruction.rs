//! Internal serde shapes for deserializing Solana `Instruction`s as returned
//! by Jupiter `/swap-instructions` (v1) and `/build` (v2).
//!
//! Both endpoints emit the same per-instruction shape:
//! ```json
//! { "programId": "<pubkey>", "accounts": [ { pubkey, isSigner, isWritable } ], "data": "<base64>" }
//! ```

use crate::serde_helpers::field_as_string;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InstructionInternal {
    #[serde(with = "field_as_string")]
    pub program_id: Pubkey,
    pub accounts: Vec<AccountMetaInternal>,
    #[serde(with = "base64_deserialize")]
    pub data: Vec<u8>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccountMetaInternal {
    #[serde(with = "field_as_string")]
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PubkeyInternal(#[serde(with = "field_as_string")] pub Pubkey);

impl From<AccountMetaInternal> for AccountMeta {
    fn from(value: AccountMetaInternal) -> Self {
        AccountMeta {
            pubkey: value.pubkey,
            is_signer: value.is_signer,
            is_writable: value.is_writable,
        }
    }
}

impl From<InstructionInternal> for Instruction {
    fn from(value: InstructionInternal) -> Self {
        Instruction {
            program_id: value.program_id,
            accounts: value.accounts.into_iter().map(Into::into).collect(),
            data: value.data,
        }
    }
}

pub mod base64_deserialize {
    use serde::{de, Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        base64::decode(s).map_err(|e| de::Error::custom(format!("base64 decoding error: {:?}", e)))
    }
}
