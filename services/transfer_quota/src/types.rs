use bytes::Bytes;
use derive_more::Display;
use serde::{Deserialize, Serialize};

use muta_codec_derive::RlpFixedCodec;
use protocol::fixed_codec::{FixedCodec, FixedCodecError};
use protocol::types::{Address, Hash};
use protocol::ProtocolResult;

#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct TransferQuotaInfo {
    pub admin: Address,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct Genesis {
    pub config: Vec<GenesisAssetConfig>,
    pub admin:  Address,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct GenesisAssetConfig {
    pub asset_id:     Hash,
    pub asset_config: AssetConfig,
}

#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct AssetAddress {
    pub asset_id: Hash,
    pub address:  Address,
}

#[display(
    fmt = "Record(last_op_time : {}, daily_used_amount : {}, monthly_used_amount : {}, yearly_used_amount : {})",
    last_op_time,
    daily_used_amount,
    monthly_used_amount,
    yearly_used_amount
)]
#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, Default, PartialEq, Eq, Display)]
pub struct Record {
    pub last_op_time:        u64,
    pub daily_used_amount:   u64,
    pub monthly_used_amount: u64,
    pub yearly_used_amount:  u64,
}

#[display(
    fmt = "AssetConfig(single_bill_quota : {:?}, daily_quota_rule : {:?}, monthly_quota_rule : {:?}, yearly_quota_rule : {:?})",
    single_bill_quota,
    daily_quota_rule,
    monthly_quota_rule,
    yearly_quota_rule
)]
#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, Default, PartialEq, Eq, Display)]
pub struct AssetConfig {
    pub admin:              Address,
    pub activated:          bool,
    pub single_bill_quota:  Vec<Rule>,
    pub daily_quota_rule:   Vec<Rule>,
    pub monthly_quota_rule: Vec<Rule>,
    pub yearly_quota_rule:  Vec<Rule>,
}

#[display(fmt = "Rule(kyc_expr{} -> quota{})", kyc_expr, quota)]
#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, Default, PartialEq, Eq, Display)]
pub struct Rule {
    pub kyc_expr: String,
    // quota while kyc_expr returns true
    pub quota:    u64,
}

#[derive(Debug, Display, PartialOrd, PartialEq, Copy, Clone)]
pub enum QuotaType {
    #[display(fmt = "SingleBill")]
    SingleBill,
    #[display(fmt = "Daily")]
    Daily,
    #[display(fmt = "Monthly")]
    Monthly,
    #[display(fmt = "Yearly")]
    Yearly,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateAssetConfigPayload {
    pub asset_id: Hash,
    pub admin:    Address,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QuotaTransferPayload {
    pub asset_id: Hash,
    pub address:  Address,
    pub amount:   u64,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChangeAssetConfigPayload {
    pub asset_id:     Hash,
    pub asset_config: AssetConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GetAssetConfigPayload {
    pub asset_id: Hash,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChangeRecordPayload {
    pub asset_id: Hash,
    pub address:  Address,
    pub record:   Record,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChangeRecordEvent {
    pub asset_id: Hash,
    pub address:  Address,
    pub record:   Record,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GetRecordPayload {
    pub asset_id: Hash,
    pub address:  Address,
}
