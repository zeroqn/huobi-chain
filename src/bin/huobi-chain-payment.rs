use derive_more::{Display, From};

use protocol::traits::{SDKFactory, Service, ServiceMapping, ServiceSDK};
use protocol::{ProtocolError, ProtocolErrorKind, ProtocolResult};

use admission_control::{AdmissionControlService, ADMISSION_CONTROL_SERVICE_NAME};
use asset::{AssetService, ASSET_SERVICE_NAME};
use authorization::{AuthorizationService, AUTHORIZATION_SERVICE_NAME};
use governance::{GovernanceService, GOVERNANCE_SERVICE_NAME};
use kyc::{KycService, KYC_SERVICE_NAME};
use metadata::{MetadataService, METADATA_SERVICE_NAME};
use multi_signature::{MultiSignatureService, MULTI_SIG_SERVICE_NAME};
use riscv::{RiscvService, RISCV_SERVICE_NAME};
use timestamp::{TimestampService, TIMESTAMP_SERVICE_NAME};
use transfer_quota::{TransferQuotaService, TRANSFER_QUOTA_SERVICE_NAME};

type AuthorizationType<SDK> = AuthorizationService<
    AdmissionControlService<
        AssetServiceType<SDK>,
        GovernanceService<AssetServiceType<SDK>, MetadataService<SDK>, SDK>,
        SDK,
    >,
    SDK,
>;

type AdmissionControlType<SDK> = AdmissionControlService<
    AssetServiceType<SDK>,
    GovernanceService<AssetServiceType<SDK>, MetadataService<SDK>, SDK>,
    SDK,
>;

type AssetServiceType<SDK> = AssetService<SDK, TransferQuotaServiceType<SDK>>;

type TransferQuotaServiceType<SDK> =
    TransferQuotaService<SDK, KycService<SDK>, TimestampService<SDK>>;

type GovernanceServiceType<SDK> =
    GovernanceService<AssetServiceType<SDK>, MetadataService<SDK>, SDK>;
type RiscvServiceType<SDK> =
    RiscvService<AssetServiceType<SDK>, GovernanceServiceType<SDK>, KycService<SDK>, SDK>;

struct DefaultServiceMapping;

impl ServiceMapping for DefaultServiceMapping {
    fn get_service<SDK: 'static + ServiceSDK, Factory: SDKFactory<SDK>>(
        &self,
        name: &str,
        factory: &Factory,
    ) -> ProtocolResult<Box<dyn Service>> {
        let service = match name {
            AUTHORIZATION_SERVICE_NAME => {
                Box::new(Self::new_authorization(factory)?) as Box<dyn Service>
            }
            GOVERNANCE_SERVICE_NAME => Box::new(Self::new_governance(factory)?) as Box<dyn Service>,
            ADMISSION_CONTROL_SERVICE_NAME => {
                Box::new(Self::new_admission_ctrl(factory)?) as Box<dyn Service>
            }
            ASSET_SERVICE_NAME => Box::new(Self::new_asset(factory)?) as Box<dyn Service>,
            METADATA_SERVICE_NAME => Box::new(Self::new_metadata(factory)?) as Box<dyn Service>,
            KYC_SERVICE_NAME => Box::new(Self::new_kyc(factory)?) as Box<dyn Service>,
            MULTI_SIG_SERVICE_NAME => Box::new(Self::new_multi_sig(factory)?) as Box<dyn Service>,
            TIMESTAMP_SERVICE_NAME => Box::new(Self::new_timestamp(factory)?) as Box<dyn Service>,
            TRANSFER_QUOTA_SERVICE_NAME => {
                Box::new(Self::new_transfer_quota(factory)?) as Box<dyn Service>
            }
            RISCV_SERVICE_NAME => Box::new(Self::new_riscv(factory)?) as Box<dyn Service>,
            _ => panic!("not found service"),
        };

        Ok(service)
    }

    fn list_service_name(&self) -> Vec<String> {
        vec![
            AUTHORIZATION_SERVICE_NAME.to_owned(),
            ASSET_SERVICE_NAME.to_owned(),
            METADATA_SERVICE_NAME.to_owned(),
            KYC_SERVICE_NAME.to_owned(),
            MULTI_SIG_SERVICE_NAME.to_owned(),
            GOVERNANCE_SERVICE_NAME.to_owned(),
            ADMISSION_CONTROL_SERVICE_NAME.to_owned(),
            TIMESTAMP_SERVICE_NAME.to_owned(),
            TRANSFER_QUOTA_SERVICE_NAME.to_owned(),
            RISCV_SERVICE_NAME.to_owned(),
        ]
    }
}

impl DefaultServiceMapping {
    fn new_transfer_quota<SDK: 'static + ServiceSDK, Factory: SDKFactory<SDK>>(
        factory: &Factory,
    ) -> ProtocolResult<TransferQuotaService<SDK, KycService<SDK>, TimestampService<SDK>>> {
        let kyc = Self::new_kyc(factory)?;
        let timestamp = Self::new_timestamp(factory)?;

        Ok(TransferQuotaService::new(
            factory.get_sdk(TRANSFER_QUOTA_SERVICE_NAME)?,
            kyc,
            timestamp,
        ))
    }

    fn new_timestamp<SDK: 'static + ServiceSDK, Factory: SDKFactory<SDK>>(
        factory: &Factory,
    ) -> ProtocolResult<TimestampService<SDK>> {
        Ok(TimestampService::new(
            factory.get_sdk(TIMESTAMP_SERVICE_NAME)?,
        ))
    }

    fn new_asset<SDK: 'static + ServiceSDK, Factory: SDKFactory<SDK>>(
        factory: &Factory,
    ) -> ProtocolResult<AssetServiceType<SDK>> {
        let transfer_quota = Self::new_transfer_quota(factory)?;
        Ok(AssetService::new(
            factory.get_sdk(ASSET_SERVICE_NAME)?,
            Some(transfer_quota),
        ))
    }

    fn new_metadata<SDK: 'static + ServiceSDK, Factory: SDKFactory<SDK>>(
        factory: &Factory,
    ) -> ProtocolResult<MetadataService<SDK>> {
        Ok(MetadataService::new(
            factory.get_sdk(METADATA_SERVICE_NAME)?,
        ))
    }

    fn new_multi_sig<SDK: 'static + ServiceSDK, Factory: SDKFactory<SDK>>(
        factory: &Factory,
    ) -> ProtocolResult<MultiSignatureService<SDK>> {
        Ok(MultiSignatureService::new(
            factory.get_sdk(MULTI_SIG_SERVICE_NAME)?,
        ))
    }

    fn new_kyc<SDK: 'static + ServiceSDK, Factory: SDKFactory<SDK>>(
        factory: &Factory,
    ) -> ProtocolResult<KycService<SDK>> {
        Ok(KycService::new(factory.get_sdk(KYC_SERVICE_NAME)?))
    }

    fn new_governance<SDK: 'static + ServiceSDK, Factory: SDKFactory<SDK>>(
        factory: &Factory,
    ) -> ProtocolResult<GovernanceServiceType<SDK>> {
        let asset = Self::new_asset(factory)?;
        let metadata = Self::new_metadata(factory)?;
        Ok(GovernanceService::new(
            factory.get_sdk(GOVERNANCE_SERVICE_NAME)?,
            asset,
            metadata,
        ))
    }

    fn new_admission_ctrl<SDK: 'static + ServiceSDK, Factory: SDKFactory<SDK>>(
        factory: &Factory,
    ) -> ProtocolResult<AdmissionControlType<SDK>> {
        let asset = Self::new_asset(factory)?;
        let governance = Self::new_governance(factory)?;
        Ok(AdmissionControlService::new(
            factory.get_sdk(ADMISSION_CONTROL_SERVICE_NAME)?,
            asset,
            governance,
        ))
    }

    fn new_authorization<SDK: 'static + ServiceSDK, Factory: SDKFactory<SDK>>(
        factory: &Factory,
    ) -> ProtocolResult<AuthorizationType<SDK>> {
        let multi_sig = Self::new_multi_sig(factory)?;
        let admission_control = Self::new_admission_ctrl(factory)?;
        Ok(AuthorizationService::new(
            factory.get_sdk(AUTHORIZATION_SERVICE_NAME)?,
            multi_sig,
            admission_control,
        ))
    }

    fn new_riscv<SDK: 'static + ServiceSDK, Factory: SDKFactory<SDK>>(
        factory: &Factory,
    ) -> ProtocolResult<RiscvServiceType<SDK>> {
        let asset = Self::new_asset(factory)?;
        let governance = Self::new_governance(factory)?;
        let kyc = Self::new_kyc(factory)?;

        Ok(RiscvService::init(
            factory.get_sdk("riscv")?,
            asset,
            governance,
            kyc,
        ))
    }
}

fn main() {
    muta::run(
        DefaultServiceMapping {},
        "Huobi-chain",
        "v0.5.0-rc.2",
        "Muta Dev <muta@nervos.org>",
        None,
    )
}

#[derive(Debug, Display, From)]
pub enum MappingError {
    #[display(fmt = "service {:?} was not found", service)]
    NotFoundService { service: String },
}
impl std::error::Error for MappingError {}

impl From<MappingError> for ProtocolError {
    fn from(err: MappingError) -> ProtocolError {
        ProtocolError::new(ProtocolErrorKind::Service, Box::new(err))
    }
}
