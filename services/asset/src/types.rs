#![allow(clippy::mutable_key_type)]
use std::{
    cmp::Ordering,
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

use bytes::Bytes;
use serde::{Deserialize, Serialize};

use muta_codec_derive::RlpFixedCodec;
use protocol::fixed_codec::{FixedCodec, FixedCodecError};
use protocol::types::{Address, Hash, Hex};
use protocol::ProtocolResult;

use crate::ServiceError;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct IssuerWithBalance {
    pub addr:    Address,
    pub balance: u64,
}

impl IssuerWithBalance {
    pub fn new(addr: Address, balance: u64) -> Self {
        IssuerWithBalance { addr, balance }
    }

    pub fn verify(&self) -> Result<(), ServiceError> {
        if self.balance == 0 {
            return Err(ServiceError::MeaningLessValue(
                "issuer's balance".to_string(),
            ));
        }

        if self.addr == Address::default() {
            Err(ServiceError::MeaningLessValue("issuer's addr".to_string()))
        } else {
            Ok(())
        }
    }
}

/// Payload
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct InitGenesisPayload {
    pub id:         Hash,
    pub name:       String,
    pub symbol:     String,
    pub supply:     u64,
    pub precision:  u64,
    pub init_mints: Vec<IssuerWithBalance>,
    pub admin:      Address,
    pub relayable:  bool,
}

impl InitGenesisPayload {
    pub fn verify(&self) -> Result<(), ServiceError> {
        verify_asset_name(&self.name)?;
        verify_asset_symbol(&self.symbol)?;

        if self.id == Hash::default() {
            return Err(ServiceError::MeaningLessValue("asset_id".to_string()));
        }

        if self.admin == Address::default() {
            return Err(ServiceError::MeaningLessValue("admin".to_string()));
        }

        if self.supply == 0 {
            return Err(ServiceError::MeaningLessValue("supply".to_string()));
        }

        let mint_balance = verify_issuers(&self.init_mints)?;
        if mint_balance != self.supply {
            return Err(ServiceError::MintNotEqualSupply);
        }

        Ok(())
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CreateAssetPayload {
    pub name:       String,
    pub symbol:     String,
    pub admin:      Address,
    pub supply:     u64,
    pub init_mints: Vec<IssuerWithBalance>,
    pub precision:  u64,
    pub relayable:  bool,
}

impl CreateAssetPayload {
    pub fn verify(&self) -> Result<(), ServiceError> {
        verify_asset_name(&self.name)?;
        verify_asset_symbol(&self.symbol)?;

        if self.supply == 0 {
            return Err(ServiceError::MeaningLessValue("supply".to_string()));
        }

        let mint_balance = verify_issuers(&self.init_mints)?;
        if mint_balance != self.supply {
            return Err(ServiceError::MintNotEqualSupply);
        }

        Ok(())
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct GetAssetPayload {
    pub id: Hash,
}

impl GetAssetPayload {
    pub fn verify(&self) -> Result<(), ServiceError> {
        if self.id == Hash::default() {
            return Err(ServiceError::MeaningLessValue("asset_id".to_string()));
        }
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TransferPayload {
    pub asset_id: Hash,
    pub to:       Address,
    pub value:    u64,
    pub memo:     String,
}

impl TransferPayload {
    pub fn verify(&self) -> Result<(), ServiceError> {
        verify_memo(&self.memo)?;

        if self.asset_id == Hash::default() {
            return Err(ServiceError::MeaningLessValue("asset_id".to_string()));
        }

        if self.to == Address::default() {
            return Err(ServiceError::MeaningLessValue("to".to_string()));
        }

        if self.value == 0 {
            return Err(ServiceError::MeaningLessValue("value".to_string()));
        }

        Ok(())
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TransferEvent {
    pub asset_id: Hash,
    pub from:     Address,
    pub to:       Address,
    pub value:    u64,
    pub memo:     String,
}

pub type ApprovePayload = TransferPayload;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ApproveEvent {
    pub asset_id: Hash,
    pub grantor:  Address,
    pub grantee:  Address,
    pub value:    u64,
    pub memo:     String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TransferFromPayload {
    pub asset_id:  Hash,
    pub sender:    Address,
    pub recipient: Address,
    pub value:     u64,
    pub memo:      String,
}

impl TransferFromPayload {
    pub fn verify(&self) -> Result<(), ServiceError> {
        verify_memo(&self.memo)?;

        if self.asset_id == Hash::default() {
            return Err(ServiceError::MeaningLessValue("asset_id".to_string()));
        }

        if self.sender == Address::default() {
            return Err(ServiceError::MeaningLessValue("sender".to_string()));
        }

        if self.recipient == Address::default() {
            return Err(ServiceError::MeaningLessValue("recipient".to_string()));
        }

        if self.value == 0 {
            return Err(ServiceError::MeaningLessValue("value".to_string()));
        }

        Ok(())
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct HookTransferFromPayload {
    pub sender:    Address,
    pub recipient: Address,
    pub value:     u64,
    pub memo:      String,
}

impl HookTransferFromPayload {
    pub fn verify(&self) -> Result<(), ServiceError> {
        verify_memo(&self.memo)?;

        if self.sender == Address::default() {
            return Err(ServiceError::MeaningLessValue("sender".to_string()));
        }

        if self.recipient == Address::default() {
            return Err(ServiceError::MeaningLessValue("recipient".to_string()));
        }

        if self.value == 0 {
            return Err(ServiceError::MeaningLessValue("value".to_string()));
        }

        Ok(())
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TransferFromEvent {
    pub asset_id:  Hash,
    pub caller:    Address,
    pub sender:    Address,
    pub recipient: Address,
    pub value:     u64,
    pub memo:      String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct GetBalancePayload {
    pub asset_id: Hash,
    pub user:     Address,
}

impl GetBalancePayload {
    pub fn verify(&self) -> Result<(), ServiceError> {
        if self.asset_id == Hash::default() {
            return Err(ServiceError::MeaningLessValue("asset_id".to_string()));
        }

        if self.user == Address::default() {
            return Err(ServiceError::MeaningLessValue("user".to_string()));
        }

        Ok(())
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct GetBalanceResponse {
    pub asset_id: Hash,
    pub user:     Address,
    pub balance:  u64,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct GetAllowancePayload {
    pub asset_id: Hash,
    pub grantor:  Address,
    pub grantee:  Address,
}

impl GetAllowancePayload {
    pub fn verify(&self) -> Result<(), ServiceError> {
        if self.asset_id == Hash::default() {
            return Err(ServiceError::MeaningLessValue("asset_id".to_string()));
        }

        if self.grantor == Address::default() {
            return Err(ServiceError::MeaningLessValue("grantor".to_string()));
        }

        if self.grantee == Address::default() {
            return Err(ServiceError::MeaningLessValue("grantee".to_string()));
        }

        Ok(())
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct GetAllowanceResponse {
    pub asset_id: Hash,
    pub grantor:  Address,
    pub grantee:  Address,
    pub value:    u64,
}

#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, PartialEq, Default)]
pub struct Asset {
    pub id:        Hash,
    pub name:      String,
    pub symbol:    String,
    pub admin:     Address,
    pub supply:    u64,
    pub precision: u64,
    pub relayable: bool,
}

pub struct AssetBalance {
    pub value:     u64,
    pub allowance: BTreeMap<Address, u64>,
}

impl AssetBalance {
    pub fn new(supply: u64) -> Self {
        AssetBalance {
            value:     supply,
            allowance: BTreeMap::new(),
        }
    }

    pub fn checked_add(&mut self, amount: u64) -> Result<(), ServiceError> {
        let (checked_value, overflow) = self.value.overflowing_add(amount);
        if overflow {
            return Err(ServiceError::BalanceOverflow);
        }

        self.value = checked_value;
        Ok(())
    }

    pub fn checked_sub(&mut self, amount: u64) -> Result<(), ServiceError> {
        let (checked_value, overflow) = self.value.overflowing_sub(amount);
        if overflow {
            return Err(ServiceError::BalanceOverflow);
        }

        self.value = checked_value;
        Ok(())
    }

    pub fn allowance(&self, spender: &Address) -> u64 {
        *self.allowance.get(spender).unwrap_or_else(|| &0)
    }

    pub fn update_allowance(&mut self, spender: Address, value: u64) {
        self.allowance
            .entry(spender)
            .and_modify(|b| *b = value)
            .or_insert(value);
    }
}

impl Deref for AssetBalance {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl DerefMut for AssetBalance {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl PartialOrd<u64> for AssetBalance {
    fn partial_cmp(&self, other: &u64) -> Option<Ordering> {
        Some(self.value.cmp(other))
    }
}

impl PartialEq<u64> for AssetBalance {
    fn eq(&self, other: &u64) -> bool {
        self.value == *other
    }
}

#[derive(RlpFixedCodec)]
struct AllowanceCodec {
    pub addr:  Address,
    pub total: u64,
}

impl rlp::Decodable for AssetBalance {
    fn decode(rlp: &rlp::Rlp) -> Result<Self, rlp::DecoderError> {
        let value = rlp.at(0)?.as_val()?;
        let codec_list: Vec<AllowanceCodec> = rlp::decode_list(rlp.at(1)?.as_raw());
        let mut allowance = BTreeMap::new();
        for v in codec_list {
            allowance.insert(v.addr, v.total);
        }

        Ok(AssetBalance { value, allowance })
    }
}

impl rlp::Encodable for AssetBalance {
    fn rlp_append(&self, s: &mut rlp::RlpStream) {
        s.begin_list(2);
        s.append(&self.value);

        let mut codec_list = Vec::with_capacity(self.allowance.len());

        for (address, allowance) in self.allowance.iter() {
            let fixed_codec = AllowanceCodec {
                addr:  address.clone(),
                total: *allowance,
            };

            codec_list.push(fixed_codec);
        }

        s.append_list(&codec_list);
    }
}

impl FixedCodec for AssetBalance {
    fn encode_fixed(&self) -> ProtocolResult<Bytes> {
        Ok(Bytes::from(rlp::encode(self)))
    }

    fn decode_fixed(bytes: Bytes) -> ProtocolResult<Self> {
        Ok(rlp::decode(bytes.as_ref()).map_err(FixedCodecError::from)?)
    }
}

impl Default for AssetBalance {
    fn default() -> Self {
        AssetBalance {
            value:     0,
            allowance: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintAssetPayload {
    pub asset_id: Hash,
    pub to:       Address,
    pub amount:   u64,
    pub proof:    Hex,
    pub memo:     String,
}

impl MintAssetPayload {
    pub fn verify(&self) -> Result<(), ServiceError> {
        verify_memo(&self.memo)?;

        if self.asset_id == Hash::default() {
            return Err(ServiceError::MeaningLessValue("asset_id".to_string()));
        }

        if self.to == Address::default() {
            return Err(ServiceError::MeaningLessValue("to".to_string()));
        }

        if self.amount == 0 {
            return Err(ServiceError::MeaningLessValue("amount".to_string()));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurnAssetPayload {
    pub asset_id: Hash,
    pub amount:   u64,
    pub proof:    Hex,
    pub memo:     String,
}

impl BurnAssetPayload {
    pub fn verify(&self) -> Result<(), ServiceError> {
        verify_memo(&self.memo)?;

        if self.asset_id == Hash::default() {
            return Err(ServiceError::MeaningLessValue("asset_id".to_string()));
        }

        if self.amount == 0 {
            return Err(ServiceError::MeaningLessValue("amount".to_string()));
        }

        Ok(())
    }
}

pub type RelayAssetPayload = BurnAssetPayload;

#[derive(Debug, Serialize, Deserialize)]
pub struct MintAssetEvent {
    pub asset_id: Hash,
    pub to:       Address,
    pub amount:   u64,
    pub proof:    Hex,
    pub memo:     String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BurnAssetEvent {
    pub asset_id: Hash,
    pub from:     Address,
    pub amount:   u64,
    pub proof:    Hex,
    pub memo:     String,
}
pub type RelayAssetEvent = BurnAssetEvent;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangeAdminPayload {
    pub asset_id:  Hash,
    pub new_admin: Address,
}

impl ChangeAdminPayload {
    pub fn verify(&self) -> Result<(), ServiceError> {
        if self.asset_id == Hash::default() {
            return Err(ServiceError::MeaningLessValue("asset_id".to_string()));
        }

        if self.new_admin == Address::default() {
            return Err(ServiceError::MeaningLessValue("new_admin".to_string()));
        }

        Ok(())
    }
}

fn verify_asset_name(name: &str) -> Result<(), ServiceError> {
    let length = name.chars().count();

    if length > 40 || length == 0 {
        return Err(ServiceError::Format);
    }

    for (index, char) in name.chars().enumerate() {
        if !(char.is_ascii_alphanumeric() || char == '_' || char == ' ') {
            return Err(ServiceError::Format);
        }

        if index == 0 && (char == '_' || char == ' ' || char.is_ascii_digit()) {
            return Err(ServiceError::Format);
        }

        if index == length - 1 && (char == '_' || char == ' ') {
            return Err(ServiceError::Format);
        }
    }

    Ok(())
}

fn verify_asset_symbol(symbol: &str) -> Result<(), ServiceError> {
    let length = symbol.chars().count();

    if length > 10 || length == 0 {
        return Err(ServiceError::Format);
    }

    for (index, char) in symbol.chars().enumerate() {
        if !(char.is_ascii_alphanumeric()) {
            return Err(ServiceError::Format);
        }

        if index == 0 && !char.is_ascii_uppercase() {
            return Err(ServiceError::Format);
        }
    }

    Ok(())
}

fn verify_memo(memo: &str) -> Result<(), ServiceError> {
    let length = memo.chars().count();

    if length > 256 {
        return Err(ServiceError::TooLongMemo);
    }

    Ok(())
}

fn verify_issuers(issuers: &[IssuerWithBalance]) -> Result<u64, ServiceError> {
    let mut accu_mint_balance = 0u64;
    for issuer in issuers {
        issuer.verify()?;
        let (checked_value, overflow) = accu_mint_balance.overflowing_add(issuer.balance);
        if overflow {
            return Err(ServiceError::BalanceOverflow);
        }
        accu_mint_balance = checked_value;
    }

    Ok(accu_mint_balance)
}
