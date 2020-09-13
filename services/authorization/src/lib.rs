use admission_control::AdmissionControlInterface;
use binding_macro::{cycles, service};
use protocol::traits::{ExecutorParams, ServiceResponse, ServiceSDK};
use protocol::types::{ServiceContext, SignedTransaction};
use serde::Deserialize;

use multi_signature::MultiSignatureService;

pub const AUTHORIZATION_SERVICE_NAME: &str = "authorization";

#[derive(Deserialize)]
pub struct PtrSignedTransaction {
    ptr: usize,
}

pub struct AuthorizationService<AC, SDK> {
    _sdk:              SDK,
    pub multi_sig:     MultiSignatureService<SDK>,
    admission_control: AC,
}

#[service]
impl<AC, SDK> AuthorizationService<AC, SDK>
where
    AC: AdmissionControlInterface,
    SDK: ServiceSDK,
{
    pub fn new(_sdk: SDK, multi_sig: MultiSignatureService<SDK>, admission_control: AC) -> Self {
        Self {
            _sdk,
            multi_sig,
            admission_control,
        }
    }

    #[cycles(21_000)]
    #[read]
    fn check_authorization_by_ptr(
        &self,
        ctx: ServiceContext,
        payload: PtrSignedTransaction,
    ) -> ServiceResponse<()> {
        let stx: SignedTransaction = {
            let boxed = unsafe { Box::from_raw(payload.ptr as *mut SignedTransaction) };
            *boxed
        };

        self.check_authorization(ctx, stx)
    }

    #[cycles(21_000)]
    #[read]
    fn check_authorization(
        &self,
        ctx: ServiceContext,
        payload: SignedTransaction,
    ) -> ServiceResponse<()> {
        let resp = self
            .multi_sig
            .verify_signature(ctx.clone(), payload.clone());
        if resp.is_error() {
            return ServiceResponse::<()>::from_error(
                102,
                format!(
                    "verify transaction signature error {:?}",
                    resp.error_message
                ),
            );
        }

        if let Err(reason) = self.admission_control.is_allowed(&ctx, payload) {
            return ServiceResponse::<()>::from_error(102, reason);
        }

        ServiceResponse::from_succeed(())
    }
}
