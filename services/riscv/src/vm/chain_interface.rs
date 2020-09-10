use crate::{common, types::ExecPayload, ServiceError};

use asset::AssetInterface;
use governance::GovernanceInterface;
use kyc::KycInterface;
use protocol::{
    traits::{ServiceResponse, ServiceSDK},
    types::{Address, Hash, ServiceContext},
    Bytes,
};
use serde::Serialize;

use std::{cell::RefCell, rc::Rc};

macro_rules! service_read {
    ($self: expr, $service: ident, $method: ident) => {{
        let resp = $self.$service.borrow().$method(&$self.ctx);
        try_encode_service_response(resp)
    }};

    ($self: expr, $service: ident, $method: ident, $payload: expr) => {{
        let payload = match serde_json::from_str($payload) {
            Ok(data) => data,
            Err(e) => return ServiceError::Serde(e).into(),
        };

        let resp = $self.$service.borrow().$method(&$self.ctx, payload);
        try_encode_service_response(resp)
    }};
}

macro_rules! service_write {
    ($self: expr, $service: ident, $method: ident, $payload: expr) => {{
        let payload = match serde_json::from_str($payload) {
            Ok(data) => data,
            Err(e) => return ServiceError::Serde(e).into(),
        };

        let resp = $self.$service.borrow_mut().$method(&$self.ctx, payload);
        try_encode_service_response(resp)
    }};
}

pub trait ChainInterface {
    fn get_storage(&self, key: &Bytes) -> Bytes;

    // Note: Only Throw ServiceError::WriteInReadonlyContext
    fn set_storage(&mut self, key: Bytes, val: Bytes) -> ServiceResponse<()>;

    fn contract_call(
        &mut self,
        address: Address,
        args: Bytes,
        current_cycle: u64,
    ) -> ServiceResponse<(String, u64)>;

    // Note: We need mut here to update cycles count in ServiceContext
    fn service_read(
        &mut self,
        service: &str,
        method: &str,
        payload: &str,
        current_cycle: u64,
    ) -> ServiceResponse<(String, u64)>;

    fn service_write(
        &mut self,
        service: &str,
        method: &str,
        payload: &str,
        current_cycle: u64,
    ) -> ServiceResponse<(String, u64)>;
}

#[derive(Debug)]
struct CycleContext {
    inner:           ServiceContext,
    all_cycles_used: u64,
}

impl CycleContext {
    pub fn new(ctx: ServiceContext, all_cycles_used: u64) -> Self {
        CycleContext {
            inner: ctx,
            all_cycles_used,
        }
    }
}

pub struct WriteableChain<AS, G, K, SDK> {
    ctx:             ServiceContext,
    payload:         ExecPayload,
    sdk:             Rc<RefCell<SDK>>,
    asset:           Rc<RefCell<AS>>,
    governance:      Rc<RefCell<G>>,
    kyc:             Rc<RefCell<K>>,
    all_cycles_used: u64,
}

impl<AS, G, K, SDK> WriteableChain<AS, G, K, SDK>
where
    AS: AssetInterface,
    G: GovernanceInterface,
    K: KycInterface,
    SDK: ServiceSDK + 'static,
{
    pub fn new(
        ctx: ServiceContext,
        payload: ExecPayload,
        sdk: Rc<RefCell<SDK>>,
        asset: Rc<RefCell<AS>>,
        governance: Rc<RefCell<G>>,
        kyc: Rc<RefCell<K>>,
    ) -> Self {
        Self {
            ctx,
            payload,
            asset,
            governance,
            kyc,
            sdk,
            all_cycles_used: 0,
        }
    }

    fn serve<F: FnMut() -> ServiceResponse<String>>(
        cycle_ctx: &mut CycleContext,
        current_cycle: u64,
        mut f: F,
    ) -> ServiceResponse<(String, u64)> {
        let (vm_cycle, overflow) = current_cycle.overflowing_sub(cycle_ctx.all_cycles_used);
        if overflow {
            return ServiceError::OutOfCycles.into();
        } else {
            crate::sub_cycles!(cycle_ctx.inner, vm_cycle);
        }

        let resp = f();
        if resp.is_error() {
            return ServiceResponse::from_error(resp.code, resp.error_message);
        }

        cycle_ctx.all_cycles_used = cycle_ctx.inner.get_cycles_used();
        ServiceResponse::from_succeed((resp.succeed_data, cycle_ctx.all_cycles_used))
    }

    fn contract_key(&self, key: &Bytes) -> Hash {
        common::combine_key(self.payload.address.as_bytes().as_ref(), key)
    }

    fn read_governance(&self, method: &str, _payload: &str) -> ServiceResponse<String> {
        match method {
            "get_info" => service_read!(self, governance, get_info),
            _ => ServiceError::MethodNotFound.into(),
        }
    }

    fn write_governance(&self, method: &str, payload: &str) -> ServiceResponse<String> {
        match method {
            "declare_profit" => service_write!(self, governance, declare_profit, payload),
            _ => ServiceError::MethodNotFound.into(),
        }
    }

    fn read_asset(&self, method: &str, payload: &str) -> ServiceResponse<String> {
        match method {
            "native_asset" => service_read!(self, asset, native_asset),
            "balance" => service_read!(self, asset, balance, payload),
            _ => ServiceError::MethodNotFound.into(),
        }
    }

    fn write_asset(&self, method: &str, payload: &str) -> ServiceResponse<String> {
        match method {
            "transfer_" => service_write!(self, asset, transfer_, payload),
            "transfer_from_" => service_write!(self, asset, transfer_from_, payload),
            "approve_" => service_write!(self, asset, approve_, payload),
            "burn_" => service_write!(self, asset, burn_, payload),
            "relay_" => service_write!(self, asset, relay_, payload),
            _ => ServiceError::MethodNotFound.into(),
        }
    }

    fn read_kyc(&self, method: &str, payload: &str) -> ServiceResponse<String> {
        match method {
            "get_orgs_" => service_read!(self, kyc, get_orgs_),
            "get_org_info_" => service_read!(self, kyc, get_org_info_, payload),
            "get_org_supported_tags_" => service_read!(self, kyc, get_org_supported_tags_, payload),
            "eval_user_tag_expression_" => {
                service_read!(self, kyc, eval_user_tag_expression_, payload)
            }
            _ => ServiceError::MethodNotFound.into(),
        }
    }

    fn write_kyc(&self, method: &str, payload: &str) -> ServiceResponse<String> {
        match method {
            "change_org_approved_" => service_write!(self, kyc, change_org_approved_, payload),
            "register_org_" => service_write!(self, kyc, register_org_, payload),
            "update_supported_tags_" => service_write!(self, kyc, update_supported_tags_, payload),
            "update_user_tags_" => service_write!(self, kyc, update_user_tags_, payload),
            _ => ServiceError::MethodNotFound.into(),
        }
    }
}

impl<AS, G, K, SDK> ChainInterface for WriteableChain<AS, G, K, SDK>
where
    AS: AssetInterface,
    G: GovernanceInterface,
    K: KycInterface,
    SDK: ServiceSDK + 'static,
{
    fn get_storage(&self, key: &Bytes) -> Bytes {
        let contract_key = self.contract_key(key);

        self.sdk
            .borrow()
            .get_value::<Hash, Bytes>(&contract_key)
            .unwrap_or_default()
    }

    fn set_storage(&mut self, key: Bytes, val: Bytes) -> ServiceResponse<()> {
        let contract_key = self.contract_key(&key);
        self.sdk.borrow_mut().set_value(contract_key, val);
        ServiceResponse::from_succeed(())
    }

    fn contract_call(
        &mut self,
        address: Address,
        args: Bytes,
        current_cycle: u64,
    ) -> ServiceResponse<(String, u64)> {
        let json_payload = match ExecPayload::new(address, args).json() {
            Ok(p) => p,
            Err(e) => return e.into(),
        };

        let resp = self.service_write("riscv", "exec", &json_payload, current_cycle);
        decode_json_response(resp)
    }

    fn service_read(
        &mut self,
        service: &str,
        method: &str,
        payload: &str,
        current_cycle: u64,
    ) -> ServiceResponse<(String, u64)> {
        let mut cycle_ctx = CycleContext::new(self.ctx.clone(), self.all_cycles_used);

        let resp = Self::serve(&mut cycle_ctx, current_cycle, || -> _ {
            match service {
                "asset" => self.read_asset(method, payload),
                "governance" => self.read_governance(method, payload),
                "kyc" => self.read_kyc(method, payload),
                _ => ServiceError::ServiceNotFound.into(),
            }
        });

        self.all_cycles_used = cycle_ctx.all_cycles_used;
        resp
    }

    fn service_write(
        &mut self,
        service: &str,
        method: &str,
        payload: &str,
        current_cycle: u64,
    ) -> ServiceResponse<(String, u64)> {
        let mut cycle_ctx = CycleContext::new(self.ctx.clone(), self.all_cycles_used);

        let resp = Self::serve(&mut cycle_ctx, current_cycle, || -> _ {
            match service {
                "asset" => self.write_asset(method, payload),
                "governance" => self.write_governance(method, payload),
                "kyc" => self.write_kyc(method, payload),
                _ => ServiceError::ServiceNotFound.into(),
            }
        });

        self.all_cycles_used = cycle_ctx.all_cycles_used;
        resp
    }
}

pub struct ReadonlyChain<AS, G, K, SDK> {
    inner: WriteableChain<AS, G, K, SDK>,
}

impl<AS, G, K, SDK> ReadonlyChain<AS, G, K, SDK>
where
    AS: AssetInterface,
    G: GovernanceInterface,
    K: KycInterface,
    SDK: ServiceSDK + 'static,
{
    pub fn new(
        ctx: ServiceContext,
        payload: ExecPayload,
        sdk: Rc<RefCell<SDK>>,
        asset: Rc<RefCell<AS>>,
        governance: Rc<RefCell<G>>,
        kyc: Rc<RefCell<K>>,
    ) -> Self {
        Self {
            inner: WriteableChain::new(ctx, payload, sdk, asset, governance, kyc),
        }
    }
}

impl<AS, G, K, SDK> ChainInterface for ReadonlyChain<AS, G, K, SDK>
where
    AS: AssetInterface,
    G: GovernanceInterface,
    K: KycInterface,
    SDK: ServiceSDK + 'static,
{
    fn get_storage(&self, key: &Bytes) -> Bytes {
        self.inner.get_storage(key)
    }

    fn set_storage(&mut self, _key: Bytes, _val: Bytes) -> ServiceResponse<()> {
        ServiceError::WriteInReadonlyContext.into()
    }

    fn contract_call(
        &mut self,
        address: Address,
        args: Bytes,
        current_cycle: u64,
    ) -> ServiceResponse<(String, u64)> {
        let json_payload = match ExecPayload::new(address, args).json() {
            Ok(p) => p,
            Err(e) => return e.into(),
        };

        let resp = self.service_read("riscv", "call", &json_payload, current_cycle);
        decode_json_response(resp)
    }

    fn service_read(
        &mut self,
        service: &str,
        method: &str,
        payload: &str,
        current_cycle: u64,
    ) -> ServiceResponse<(String, u64)> {
        self.inner
            .service_read(service, method, payload, current_cycle)
    }

    fn service_write(
        &mut self,
        _service: &str,
        _method: &str,
        _payload: &str,
        _current_cycle: u64,
    ) -> ServiceResponse<(String, u64)> {
        ServiceError::WriteInReadonlyContext.into()
    }
}

fn decode_json_response(resp: ServiceResponse<(String, u64)>) -> ServiceResponse<(String, u64)> {
    if resp.is_error() {
        return ServiceResponse::from_error(resp.code, resp.error_message);
    }

    let (json_ret, cycle) = resp.succeed_data;
    let raw_ret = match serde_json::from_str(&json_ret) {
        Ok(r) => r,
        Err(err) => return ServiceError::Serde(err).into(),
    };

    ServiceResponse::from_succeed((raw_ret, cycle))
}

fn try_encode_service_response<T: Serialize>(
    resp: Result<T, ServiceResponse<()>>,
) -> ServiceResponse<String> {
    match resp {
        Ok(data) => match serde_json::to_string(&data) {
            Ok(json_string) => ServiceResponse::from_succeed(json_string),
            Err(err) => ServiceError::Serde(err).into(),
        },
        Err(err) => ServiceResponse::from_error(err.code, err.error_message),
    }
}
