#[cfg(test)]
mod tests;
pub mod types;

use bytes::Bytes;
use derive_more::Display;
use serde::Serialize;

use binding_macro::{cycles, genesis /* , read */, service, write};
use chrono::{Datelike, TimeZone, Utc};

use protocol::traits::{ExecutorParams, ServiceResponse, ServiceSDK, StoreMap};
use protocol::types::{Address, Hash, ServiceContext};

use kyc::{EvalUserTagExpression, KycInterface};
use timestamp::TimestampInterface;

use crate::types::{
    AssetAddress, AssetConfig, ChangeAssetConfigPayload, ChangeRecordEvent, ChangeRecordPayload,
    CreateAssetConfigPayload, Genesis, GetAssetConfigPayload, GetRecordPayload,
    QuotaTransferPayload, QuotaType, Record, TransferQuotaInfo,
};

static TRANSFER_QUOTA_TOKEN: Bytes = Bytes::from_static(b"asset_service");
const ACCOUNT_RECORD_KEY: &str = "account_info_key";
const ASSET_CONFIG_KEY: &str = "asset_config_key";
const INFO_KEY: &str = "info";
pub const TRANSFER_QUOTA_SERVICE_NAME: &str = "transfer_quota";

macro_rules! impl_transfer_quota {
    ($self: expr, $method: ident, $ctx: expr) => {{
        let res = $self.$method($ctx.clone());
        if res.is_error() {
            Err(ServiceResponse::from_error(res.code, res.error_message))
        } else {
            Ok(res.succeed_data)
        }
    }};
    ($self: expr, $method: ident, $ctx: expr, $payload: expr) => {{
        let res = $self.$method($ctx.clone(), $payload);
        if res.is_error() {
            Err(ServiceResponse::from_error(res.code, res.error_message))
        } else {
            Ok(res.succeed_data)
        }
    }};
}

pub trait TransferQuotaInterface {
    fn quota_transfer_(
        &mut self,
        ctx: ServiceContext,
        payload: QuotaTransferPayload,
    ) -> Result<(), ServiceResponse<()>>;

    fn create_asset_config_(
        &mut self,
        ctx: ServiceContext,
        payload: CreateAssetConfigPayload,
    ) -> Result<(), ServiceResponse<()>>;
}

pub struct TransferQuotaService<SDK, KYC, TS> {
    sdk:                   SDK,
    pub kyc_service:       KYC,
    pub timestamp_service: TS,
    // store the account quota info
    // asset_id + address -> records
    account_record:        Box<dyn StoreMap<AssetAddress, Record>>,
    // store the quota config to each asset
    // asset_id -> config
    asset_config:          Box<dyn StoreMap<Hash, AssetConfig>>,
}

impl<SDK: ServiceSDK, KYC: KycInterface, TS: TimestampInterface> TransferQuotaInterface
    for TransferQuotaService<SDK, KYC, TS>
{
    fn quota_transfer_(
        &mut self,
        ctx: ServiceContext,
        payload: QuotaTransferPayload,
    ) -> Result<(), ServiceResponse<()>> {
        impl_transfer_quota!(self, quota_transfer, ctx, payload)
    }

    fn create_asset_config_(
        &mut self,
        ctx: ServiceContext,
        payload: CreateAssetConfigPayload,
    ) -> Result<(), ServiceResponse<()>> {
        impl_transfer_quota!(self, create_asset_config, ctx, payload)
    }
}

#[service]
impl<SDK: ServiceSDK, KYC: KycInterface, TS: TimestampInterface>
    TransferQuotaService<SDK, KYC, TS>
{
    pub fn new(mut sdk: SDK, kyc_service: KYC, timestamp_service: TS) -> Self {
        let account_record: Box<dyn StoreMap<AssetAddress, Record>> =
            sdk.alloc_or_recover_map(ACCOUNT_RECORD_KEY);
        let asset_config: Box<dyn StoreMap<Hash, AssetConfig>> =
            sdk.alloc_or_recover_map(ASSET_CONFIG_KEY);

        Self {
            sdk,
            kyc_service,
            timestamp_service,
            account_record,
            asset_config,
        }
    }

    #[genesis]
    pub fn init_genesis(&mut self, payload: Genesis) {
        for asset_config in payload.config {
            self.asset_config
                .insert(asset_config.asset_id, asset_config.asset_config)
        }

        self.sdk.set_value(INFO_KEY.to_owned(), TransferQuotaInfo {
            admin: payload.admin,
        });
    }

    // only accept Asset Service
    // no one would create asset config manually
    pub fn create_asset_config(
        &mut self,
        ctx: ServiceContext,
        payload: CreateAssetConfigPayload,
    ) -> ServiceResponse<()> {
        match ctx.get_extra() {
            Some(extra) if extra == TRANSFER_QUOTA_TOKEN => (),
            Some(_) => return ServiceError::NotAuthorized.into(),
            None => return ServiceError::NotAuthorized.into(),
        }

        if self.asset_config.get(&payload.asset_id).is_some() {
            return ServiceError::AssetConfigExist.into();
        }

        self.asset_config.insert(payload.asset_id, AssetConfig {
            admin:              payload.admin,
            activated:          false,
            single_bill_quota:  vec![],
            daily_quota_rule:   vec![],
            monthly_quota_rule: vec![],
            yearly_quota_rule:  vec![],
        });
        ServiceResponse::from_succeed(())
    }

    #[cycles(2_000)]
    #[write]
    pub fn change_asset_config(
        &mut self,
        ctx: ServiceContext,
        payload: ChangeAssetConfigPayload,
    ) -> ServiceResponse<()> {
        if !self.is_asset_admin(ctx.get_caller(), payload.asset_id.clone()) {
            return ServiceError::NotAuthorized.into();
        }

        self.asset_config
            .insert(payload.asset_id, payload.asset_config);
        ServiceResponse::from_succeed(())
    }

    #[cycles(2_000)]
    #[read]
    pub fn get_asset_config(
        &self,
        ctx: ServiceContext,
        payload: GetAssetConfigPayload,
    ) -> ServiceResponse<AssetConfig> {
        let config = self.asset_config.get(&payload.asset_id);
        if let Some(asset_config) = config {
            ServiceResponse::from_succeed(asset_config)
        } else {
            ServiceError::AssetNotFound(payload.asset_id).into()
        }
    }

    #[cycles(2_000)]
    #[write]
    pub fn change_record(
        &mut self,
        ctx: ServiceContext,
        payload: ChangeRecordPayload,
    ) -> ServiceResponse<()> {
        if !self.is_asset_admin(ctx.get_caller(), payload.asset_id.clone()) {
            return ServiceError::NotAuthorized.into();
        }

        self.account_record.insert(
            AssetAddress {
                asset_id: payload.asset_id.clone(),
                address:  payload.address.clone(),
            },
            payload.record.clone(),
        );
        Self::emit_event(&ctx, "ChangeRecord".to_owned(), ChangeRecordEvent {
            asset_id: payload.asset_id,
            address:  payload.address,
            record:   payload.record,
        });

        ServiceResponse::from_succeed(())
    }

    #[cycles(2_000)]
    #[read]
    pub fn get_record(
        &self,
        ctx: ServiceContext,
        payload: GetRecordPayload,
    ) -> ServiceResponse<Record> {
        let record = self
            .account_record
            .get(&AssetAddress {
                asset_id: payload.asset_id.clone(),
                address:  payload.address,
            })
            .unwrap_or_else(Record::default);

        ServiceResponse::from_succeed(record)
    }

    #[cycles(2_000)]
    #[write]
    pub fn quota_transfer(
        &mut self,
        ctx: ServiceContext,
        payload: QuotaTransferPayload,
    ) -> ServiceResponse<()> {
        let config = self.asset_config.get(&payload.asset_id);
        if config.is_none() {
            return ServiceError::AssetNotFound(payload.asset_id).into();
        }

        let config = config.unwrap();

        if !config.activated {
            return ServiceResponse::from_succeed(());
        }

        let mut record = self
            .account_record
            .get(&AssetAddress {
                asset_id: payload.asset_id.clone(),
                address:  payload.address.clone(),
            })
            .unwrap_or_else(Record::default);
        let now = self.timestamp_service.now_(&ctx);

        if now.is_err() {
            return now.unwrap_err();
        }

        let now = now.unwrap();

        if let Err(e) = self.check_quota(
            ctx.clone(),
            payload.address.clone(),
            payload.amount,
            &mut record,
            config.clone(),
            QuotaType::SingleBill,
            now,
        ) {
            return e.into();
        }

        if let Err(e) = self.check_quota(
            ctx.clone(),
            payload.address.clone(),
            payload.amount,
            &mut record,
            config.clone(),
            QuotaType::Daily,
            now,
        ) {
            return e.into();
        }

        if let Err(e) = self.check_quota(
            ctx.clone(),
            payload.address.clone(),
            payload.amount,
            &mut record,
            config.clone(),
            QuotaType::Monthly,
            now,
        ) {
            return e.into();
        }

        if let Err(e) = self.check_quota(
            ctx,
            payload.address.clone(),
            payload.amount,
            &mut record,
            config,
            QuotaType::Yearly,
            now,
        ) {
            return e.into();
        }

        // update last_op_timestamp
        record.last_op_time = now;
        self.account_record.insert(
            AssetAddress {
                asset_id: payload.asset_id,
                address:  payload.address,
            },
            record,
        );

        ServiceResponse::from_succeed(())
    }

    // returns if check is ok
    // and if check is ok, modify given record
    // returns result when the rules hit,
    // i.e. kyc_expr == true, and quota is ok, true
    // i.e. kyc_expr not hit any, false
    #[allow(clippy::too_many_arguments)]
    pub fn check_quota(
        &self,
        ctx: ServiceContext,
        address: Address,
        amount: u64,
        record: &mut Record,
        config: AssetConfig,
        quota_type: QuotaType,
        now: u64,
    ) -> Result<(), ServiceError> {
        let rules = match quota_type {
            QuotaType::SingleBill => config.single_bill_quota,

            QuotaType::Yearly => config.yearly_quota_rule,

            QuotaType::Monthly => config.monthly_quota_rule,

            QuotaType::Daily => config.daily_quota_rule,
        };

        // test rules one by one in the forward direction
        for rule in rules {
            let kyc_res = self
                .kyc_service
                .eval_user_tag_expression_(&ctx, EvalUserTagExpression {
                    user:       address.clone(),
                    expression: rule.kyc_expr.clone(),
                });
            if kyc_res.is_err() {
                continue;
            }
            let kyc_res = kyc_res.unwrap();

            // kyc expr hit
            if kyc_res {
                let last_time = Utc.timestamp_millis(record.last_op_time as i64);
                // fix
                let now = Utc.timestamp_millis(now as i64);

                let used = match quota_type {
                    QuotaType::SingleBill => 0,
                    QuotaType::Yearly => {
                        if last_time.year() == now.year() {
                            record.yearly_used_amount
                        } else {
                            0
                        }
                    }
                    QuotaType::Monthly => {
                        if last_time.year() == now.year() && last_time.month() == now.month() {
                            record.monthly_used_amount
                        } else {
                            0
                        }
                    }
                    QuotaType::Daily => {
                        if last_time.year() == now.year()
                            && last_time.month() == now.month()
                            && last_time.day() == now.day()
                        {
                            record.daily_used_amount
                        } else {
                            0
                        }
                    }
                };

                let (added_amount, overflow) = used.overflowing_add(amount);
                if overflow {
                    return Err(ServiceError::QuotaCalcOverflow(quota_type));
                }
                if added_amount > rule.quota {
                    return Err(ServiceError::QuotaExceed(
                        quota_type,
                        added_amount,
                        amount,
                        rule.quota,
                    ));
                }

                match quota_type {
                    QuotaType::SingleBill => (),
                    QuotaType::Yearly => {
                        record.yearly_used_amount = added_amount;
                    }
                    QuotaType::Monthly => {
                        record.monthly_used_amount = added_amount;
                    }
                    QuotaType::Daily => {
                        record.daily_used_amount = added_amount;
                    }
                }
                return Ok(());
            }
        }
        // no rules hit
        Err(ServiceError::QuotaNoRuleHit(quota_type))
    }

    pub fn is_asset_admin(&self, address: Address, asset_id: Hash) -> bool {
        let config = self.asset_config.get(&asset_id);
        if let Some(asset_config) = config {
            if asset_config.admin == address {
                return true;
            }
        }
        false
    }

    #[cfg(test)]
    pub fn set_record(&mut self, asset_id: Hash, address: Address, record: Record) {
        self.account_record
            .insert(AssetAddress { asset_id, address }, record)
    }

    fn emit_event<T: Serialize>(
        ctx: &ServiceContext,
        name: String,
        event: T,
    ) -> ServiceResponse<()> {
        match serde_json::to_string(&event) {
            Err(err) => ServiceError::JsonParse(err).into(),
            Ok(json) => {
                ctx.emit_event(TRANSFER_QUOTA_SERVICE_NAME.to_owned(), name, json);
                ServiceResponse::from_succeed(())
            }
        }
    }
}

#[derive(Debug, Display)]
pub enum ServiceError {
    #[display(fmt = "Parsing payload to json failed {:?}", _0)]
    JsonParse(serde_json::Error),
    #[display(
        fmt = "{} Quota exceed, {} used, {} asking, quota is {}",
        _0,
        _1,
        _2,
        _3
    )]
    QuotaExceed(QuotaType, u64, u64, u64),
    #[display(fmt = "{} Quota calculation overflow", _0)]
    QuotaCalcOverflow(QuotaType),
    #[display(fmt = "{} Quota no rule hit", _0)]
    QuotaNoRuleHit(QuotaType),

    #[display(fmt = "Asset {:?} not found", _0)]
    AssetNotFound(Hash),

    #[display(fmt = "Caller is not authorized")]
    NotAuthorized,

    #[display(fmt = "Asset config already exists")]
    AssetConfigExist,
}

impl ServiceError {
    fn code(&self) -> u64 {
        match self {
            ServiceError::JsonParse(_) => 101,
            ServiceError::QuotaExceed(_, _, _, _) => 102,
            ServiceError::QuotaCalcOverflow(_) => 103,
            ServiceError::QuotaNoRuleHit(_) => 104,
            ServiceError::AssetNotFound(_) => 105,
            ServiceError::NotAuthorized => 106,
            ServiceError::AssetConfigExist => 107,
        }
    }
}

impl<T: Default> From<ServiceError> for ServiceResponse<T> {
    fn from(err: ServiceError) -> ServiceResponse<T> {
        ServiceResponse::from_error(err.code(), err.to_string())
    }
}
