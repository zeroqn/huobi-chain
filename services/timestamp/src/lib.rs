#[cfg(test)]
mod tests;

use derive_more::{Display, From};
use serde::{Deserialize, Serialize};

use binding_macro::{cycles, genesis /* , read */, hook_before, service, write};
use muta_codec_derive::RlpFixedCodec;
use protocol::fixed_codec::{FixedCodec, FixedCodecError};
use protocol::traits::{ExecutorParams, ServiceResponse, ServiceSDK, StoreUint64};
use protocol::types::{Address, Bytes, ServiceContext};
use protocol::ProtocolResult;

pub const TIMESTAMP_SERVICE_NAME: &str = "timestamp";

const TIMESTAMP_KEY: &str = "timestamp";
const INFO_KEY: &str = "info";

macro_rules! impl_timestamp {
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

macro_rules! require_admin {
    ($service: expr, $ctx:expr) => {
        if !$service.is_admin($ctx) {
            return ServiceError::NotAuthorized.into();
        }
    };
}

macro_rules! get_info {
    ($service:expr) => {{
        let tmp = $service
            .sdk
            .get_value::<_, TimestampInfo>(&INFO_KEY.to_owned());
        if tmp.is_none() {
            return ServiceError::MissingInfo.into();
        }
        tmp.unwrap()
    }};
}

pub trait TimestampInterface {
    fn now_(&self, ctx: &ServiceContext) -> Result<u64, ServiceResponse<()>>;
}

pub struct TimestampService<SDK> {
    sdk:  SDK,
    time: Box<dyn StoreUint64>,
}

impl<SDK: ServiceSDK> TimestampInterface for TimestampService<SDK> {
    fn now_(&self, ctx: &ServiceContext) -> Result<u64, ServiceResponse<()>> {
        impl_timestamp!(self, now, ctx)
    }
}

#[service]
impl<SDK: ServiceSDK> TimestampService<SDK> {
    pub fn new(mut sdk: SDK) -> Self {
        let time: Box<dyn StoreUint64> = sdk.alloc_or_recover_uint64(TIMESTAMP_KEY);
        Self { sdk, time }
    }

    #[genesis]
    pub fn init_genesis(&mut self, payload: Genesis) {
        self.time.set(payload.start_time);
        self.sdk
            .set_value::<_, TimestampInfo>(INFO_KEY.to_owned(), TimestampInfo {
                admin:  payload.admin,
                oracle: payload.oracle,
            });
        log::error!("TimestampService, init_genesis")
    }

    #[cycles(2_000)]
    #[read]
    pub fn now(&self, _ctx: ServiceContext) -> ServiceResponse<u64> {
        ServiceResponse::from_succeed(self.time.get())
    }

    // only work in oracle mode
    #[cycles(2_000)]
    #[write]
    pub fn feed_time(
        &mut self,
        ctx: ServiceContext,
        payload: FeedTimePayload,
    ) -> ServiceResponse<()> {
        require_admin!(self, &ctx);
        let info = self.get_info(ctx);
        if info.is_error() {
            return ServiceResponse::from_error(info.code, info.error_message);
        }

        let info = info.succeed_data;
        if !info.oracle {
            return ServiceResponse::from(ServiceError::NotOracleMode);
        };

        self.set_time(payload.timestamp);
        ServiceResponse::from_succeed(())
    }

    #[cycles(2_000)]
    #[read]
    pub fn get_admin(&self, _ctx: ServiceContext) -> ServiceResponse<Address> {
        if let Some(info) = self.sdk.get_value::<_, TimestampInfo>(&INFO_KEY.to_owned()) {
            ServiceResponse::from_succeed(info.admin)
        } else {
            ServiceError::MissingInfo.into()
        }
    }

    #[cycles(2_000)]
    #[read]
    pub fn get_info(&self, ctx: ServiceContext) -> ServiceResponse<TimestampInfo> {
        if let Some(info) = self.sdk.get_value::<_, TimestampInfo>(&INFO_KEY.to_owned()) {
            ServiceResponse::from_succeed(info)
        } else {
            ServiceError::MissingInfo.into()
        }
    }

    #[cycles(2_000)]
    #[write]
    pub fn set_admin(
        &mut self,
        ctx: ServiceContext,
        payload: SetAdminPayload,
    ) -> ServiceResponse<()> {
        require_admin!(self, &ctx);
        let mut info = get_info!(self);
        info.admin = payload.admin.clone();
        self.sdk.set_value(INFO_KEY.to_owned(), info);

        let event = SetAdminEvent {
            admin: payload.admin,
        };
        Self::emit_event(&ctx, "SetAdmin".to_owned(), event)
    }

    #[cycles(2_000)]
    #[write]
    pub fn set_oracle(
        &mut self,
        ctx: ServiceContext,
        payload: SetOraclePayload,
    ) -> ServiceResponse<()> {
        require_admin!(self, &ctx);
        let mut info = get_info!(self);
        info.oracle = payload.oracle;
        self.sdk.set_value(INFO_KEY.to_owned(), info);

        let event = SetOracleEvent {
            oracle: payload.oracle,
        };
        Self::emit_event(&ctx, "SetOracle".to_owned(), event)
    }

    #[hook_before]
    pub fn set_timestamp_hook(&mut self, params: &ExecutorParams) {
        if let Some(info) = self.sdk.get_value::<_, TimestampInfo>(&INFO_KEY.to_owned()) {
            if !info.oracle {
                self.set_time(params.timestamp)
            }
        } else {
            log::error!("timestamp service, set_timestamp_hook doesn't find TimestampInfo?")
        }
    }

    // called by require_admin!
    fn is_admin(&self, ctx: &ServiceContext) -> bool {
        self.sdk
            .get_value::<_, TimestampInfo>(&INFO_KEY.to_owned())
            .map_or(false, |info| info.admin == ctx.get_caller())
    }

    fn set_time(&mut self, time: u64) {
        if time > self.time.get() {
            self.time.set(time)
        }
    }

    fn emit_event<T: Serialize>(
        ctx: &ServiceContext,
        name: String,
        event: T,
    ) -> ServiceResponse<()> {
        match serde_json::to_string(&event) {
            Err(err) => ServiceError::JsonParse(err).into(),
            Ok(json) => {
                ctx.emit_event(TIMESTAMP_SERVICE_NAME.to_owned(), name, json);
                ServiceResponse::from_succeed(())
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Genesis {
    pub start_time: u64,
    pub oracle:     bool,
    pub admin:      Address,
}

#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, Default)]
pub struct TimestampInfo {
    pub admin:  Address,
    pub oracle: bool,
}

#[derive(RlpFixedCodec, Deserialize, Serialize, Clone, Debug, Default)]
pub struct FeedTimePayload {
    pub timestamp: u64,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct SetAdminPayload {
    pub admin: Address,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct SetAdminEvent {
    pub admin: Address,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct SetOraclePayload {
    pub oracle: bool,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct SetOracleEvent {
    pub oracle: bool,
}

#[derive(Debug, Display, From)]
pub enum ServiceError {
    #[display(fmt = "Not in oracle mode")]
    NotOracleMode,
    #[display(fmt = "Caller is not authorized")]
    NotAuthorized,
    #[display(fmt = "Can not get timestamp info")]
    MissingInfo,
    #[display(fmt = "Parsing payload to json failed {:?}", _0)]
    JsonParse(serde_json::Error),
}

impl ServiceError {
    fn code(&self) -> u64 {
        match self {
            ServiceError::NotOracleMode => 101,
            ServiceError::NotAuthorized => 102,
            ServiceError::MissingInfo => 103,
            ServiceError::JsonParse(_) => 104,
        }
    }
}

impl<T: Default> From<ServiceError> for ServiceResponse<T> {
    fn from(err: ServiceError) -> ServiceResponse<T> {
        ServiceResponse::from_error(err.code(), err.to_string())
    }
}
