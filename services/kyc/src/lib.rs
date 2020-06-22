mod error;
mod expression;
mod types;
use error::ServiceError;

use expression::traits::ExpressionDataFeed;
use types::{
    ChangeOrgAdmin, ChangeOrgApproved, EvalUserTagExpression, Event, GetUserTags, KycOrgInfo,
    NoneEmptyVec, OrgName, RegisterNewOrg, TagName, TagString, UpdateOrgSupportTags,
    UpdateUserTags, Validate,
};

use binding_macro::{cycles, genesis, read, service, write};
use derive_more::Constructor;
use muta_codec_derive::RlpFixedCodec;
use protocol::{
    fixed_codec::{FixedCodec, FixedCodecError},
    traits::{ExecutorParams, ServiceResponse, ServiceSDK, StoreMap},
    types::{Address, ServiceContext},
    Bytes, ProtocolResult,
};
use serde::Serialize;

use std::collections::HashMap;

const KYC_SERVICE_ADMIN_KEY: &str = "kyc_service_admin";

macro_rules! require_service_admin {
    ($service:expr, $ctx:expr) => {{
        let admin = $service
            .sdk
            .get_value(&KYC_SERVICE_ADMIN_KEY.to_owned())
            .expect("admin not found");

        if $ctx.get_caller() != admin {
            return ServiceError::NonAuthorized.into();
        }
    }};
}

macro_rules! require_org_exists {
    ($service:expr, $org_name:expr) => {
        if !$service.orgs.contains(&$org_name) {
            return ServiceError::OrgNotFound($org_name).into();
        }
    };
}

#[macro_export]
macro_rules! sub_cycles {
    ($ctx:expr, $cycles:expr) => {
        if !$ctx.sub_cycles($cycles) {
            return ServiceError::OutOfCycles.into();
        }
    };
}

#[derive(Debug, Clone, PartialEq, Eq, RlpFixedCodec, Constructor)]
struct UserTagNamesKey {
    org_name: OrgName,
    user:     Address,
}

#[derive(Debug, Clone, PartialEq, Eq, RlpFixedCodec, Constructor)]
struct UserTagsKey {
    org_name: OrgName,
    user:     Address,
    tag_name: TagName,
}

// NOTE: update_user_tags will not remove old tags. Must check user_tag_names
// before access user_tags.
pub struct KycService<SDK> {
    sdk:            SDK,
    orgs:           Box<dyn StoreMap<OrgName, KycOrgInfo>>,
    orgs_approved:  Box<dyn StoreMap<OrgName, bool>>,
    user_tag_names: Box<dyn StoreMap<UserTagNamesKey, NoneEmptyVec<TagName>>>,
    user_tags:      Box<dyn StoreMap<UserTagsKey, NoneEmptyVec<TagString>>>,
}

#[service]
impl<SDK: ServiceSDK> KycService<SDK> {
    pub fn new(mut sdk: SDK) -> Self {
        let orgs = sdk.alloc_or_recover_map("kyc_orgs");
        let orgs_approved = sdk.alloc_or_recover_map("kyc_orgs_approved");
        let user_tag_names = sdk.alloc_or_recover_map("kyc_user");
        let user_tags = sdk.alloc_or_recover_map("kyc_user_tags");

        Self {
            sdk,
            orgs,
            orgs_approved,
            user_tag_names,
            user_tags,
        }
    }

    // Note: Use Option to provide default value require by ServiceResponse
    #[cycles(21_000)]
    #[read]
    fn get_org_info(
        &self,
        ctx: ServiceContext,
        org_name: OrgName,
    ) -> ServiceResponse<Option<KycOrgInfo>> {
        require_org_exists!(self, org_name);

        // Impossible, already ensure org exists
        let mut org = self.orgs.get(&org_name).unwrap();
        org.approved = self.orgs_approved.get(&org_name).unwrap_or_else(|| false);

        ServiceResponse::from_succeed(Some(org))
    }

    #[cycles(21_000)]
    #[read]
    fn get_org_supported_tags(
        &self,
        ctx: ServiceContext,
        org_name: OrgName,
    ) -> ServiceResponse<Vec<TagName>> {
        require_org_exists!(self, org_name);

        // Impossible, already ensure org exists
        let org = self.orgs.get(&org_name).unwrap();
        ServiceResponse::from_succeed(org.supported_tags.into())
    }

    #[cycles(21_000)]
    #[read]
    fn get_user_tags(
        &self,
        ctx: ServiceContext,
        payload: GetUserTags,
    ) -> ServiceResponse<HashMap<TagName, NoneEmptyVec<TagString>>> {
        if let Err(e) = payload.validate() {
            return e.into();
        }

        require_org_exists!(self, payload.org_name);

        let tag_names_key = UserTagNamesKey::new(payload.org_name.clone(), payload.user.clone());
        let tag_names: Vec<TagName> = match self.user_tag_names.get(&tag_names_key) {
            Some(names) => names.into(),
            None => return ServiceResponse::from_succeed(HashMap::new()),
        };

        let required_cycles = tag_names.len() * 10_000;
        sub_cycles!(ctx, required_cycles as u64);

        let mut user_tags = HashMap::with_capacity(tag_names.len());
        for tag_name in tag_names.into_iter() {
            let tags_key = UserTagsKey::new(
                payload.org_name.clone(),
                payload.user.clone(),
                tag_name.to_owned(),
            );

            if let Some(tags) = self.user_tags.get(&tags_key) {
                user_tags.insert(tag_name, tags);
            }
        }

        ServiceResponse::from_succeed(user_tags)
    }

    #[cycles(21_000)]
    #[read]
    fn eval_user_tag_expression(
        &self,
        ctx: ServiceContext,
        payload: EvalUserTagExpression,
    ) -> ServiceResponse<bool> {
        if let Err(e) = payload.validate() {
            return e.into();
        }

        let required_cycles = payload.expression.len() * 10_000;
        sub_cycles!(ctx, required_cycles as u64);

        let evaluated = match expression::evaluate(self, payload.user, payload.expression) {
            Ok(r) => r,
            Err(e) => return ServiceError::Expression(e).into(),
        };

        ServiceResponse::from_succeed(evaluated)
    }

    #[cycles(21_000)]
    #[write]
    fn change_org_approved(
        &mut self,
        ctx: ServiceContext,
        payload: ChangeOrgApproved,
    ) -> ServiceResponse<()> {
        require_service_admin!(self, &ctx);
        require_org_exists!(self, payload.org_name);

        self.orgs_approved
            .insert(payload.org_name.clone(), payload.approved.clone());

        Self::emit_event(&ctx, Event {
            topic: "change_org_approved".to_owned(),
            data:  payload,
        })
    }

    #[cycles(21_000)]
    #[write]
    fn change_key_admin(&mut self, ctx: ServiceContext, new_admin: Address) -> ServiceResponse<()> {
        require_service_admin!(self, &ctx);

        self.sdk
            .set_value(KYC_SERVICE_ADMIN_KEY.to_owned(), new_admin);
        ServiceResponse::from_succeed(())
    }

    #[cycles(21_000)]
    #[write]
    fn change_org_admin(
        &mut self,
        ctx: ServiceContext,
        payload: ChangeOrgAdmin,
    ) -> ServiceResponse<()> {
        require_org_exists!(self, payload.name);

        let mut org = self.orgs.get(&payload.name).unwrap();
        if ctx.get_caller() != org.admin {
            return ServiceError::NonAuthorized.into();
        }

        org.admin = payload.new_admin.clone();
        self.orgs.insert(payload.name.clone(), org);

        Self::emit_event(&ctx, Event {
            topic: "change_org_admin".to_owned(),
            data:  payload,
        })
    }

    #[cycles(21_000)]
    #[write]
    fn register(&mut self, ctx: ServiceContext, new_org: RegisterNewOrg) -> ServiceResponse<()> {
        require_service_admin!(self, &ctx);

        if let Err(e) = new_org.validate() {
            return e.into();
        }
        if self.orgs.contains(&new_org.name) {
            return ServiceError::OrgAlreadyExists.into();
        }

        let required_cycles = {
            let string_bytes =
                new_org.name.len() + new_org.description.len() + new_org.admin.as_bytes().len();
            let tags = new_org.supported_tags.len();

            string_bytes * 1000 + tags * 10_000
        };
        sub_cycles!(ctx, required_cycles as u64);

        let org = KycOrgInfo {
            name:           new_org.name.clone(),
            description:    new_org.description,
            admin:          new_org.admin,
            supported_tags: new_org.supported_tags.clone(),
            approved:       false,
        };

        self.orgs.insert(new_org.name.to_owned(), org);

        #[derive(Debug, Serialize)]
        struct NewOrgEvent {
            name:           OrgName,
            supported_tags: Vec<TagString>,
        }

        Self::emit_event(&ctx, Event {
            topic: "register".to_owned(),
            data:  NewOrgEvent {
                name:           new_org.name,
                supported_tags: new_org.supported_tags,
            },
        })
    }

    #[cycles(21_000)]
    #[write]
    fn update_supported_tags(
        &mut self,
        ctx: ServiceContext,
        payload: UpdateOrgSupportTags,
    ) -> ServiceResponse<()> {
        require_service_admin!(self, &ctx);
        require_org_exists!(self, payload.org_name);

        let required_cycles = payload.supported_tags.len() * 10_000;
        sub_cycles!(ctx, required_cycles as u64);

        // Impossible, already checked by require_org_exists!()
        let mut org = self.orgs.get(&payload.org_name).unwrap();
        org.supported_tags = payload.supported_tags.clone();

        Self::emit_event(&ctx, Event {
            topic: "update_supported_tags".to_owned(),
            data:  payload,
        })
    }

    #[cycles(21_000)]
    #[write]
    fn update_user_tags(
        &mut self,
        ctx: ServiceContext,
        payload: UpdateUserTags,
    ) -> ServiceResponse<()> {
        require_org_exists!(self, payload.org_name);

        // Impossible, already checked by require_org_exists!()
        let org = self.orgs.get(&payload.org_name).unwrap();
        if org.admin != ctx.get_caller() {
            return ServiceError::NonAuthorized.into();
        }

        // Update tag_names
        let maybe_tag_names = {
            let tag_names = payload.tags.keys().cloned().collect::<Vec<TagName>>();
            NoneEmptyVec::from_vec(tag_names)
        };
        let tag_names_key = UserTagNamesKey::new(payload.org_name.clone(), payload.user.clone());

        let tag_names = match maybe_tag_names {
            Ok(names) => names,
            Err(e) => {
                self.user_tag_names.remove(&tag_names_key);

                return Self::emit_event(&ctx, Event {
                    topic: "update_user_tags".to_owned(),
                    data:  payload,
                });
            }
        };

        let required_cycles = tag_names.len() * 10_000;
        sub_cycles!(ctx, required_cycles as u64);

        self.user_tag_names.insert(tag_names_key, tag_names);

        // Update tags
        for (tag_name, tags) in payload.tags.iter() {
            let required_cycles = tags.len() * 10_000;
            sub_cycles!(ctx, required_cycles as u64);

            let tags_key = UserTagsKey::new(
                payload.org_name.clone(),
                payload.user.clone(),
                tag_name.to_owned(),
            );
            self.user_tags.insert(tags_key, tags.to_owned());
        }

        Self::emit_event(&ctx, Event {
            topic: "update_user_tags".to_owned(),
            data:  payload,
        })
    }

    fn emit_event<T: Serialize>(ctx: &ServiceContext, event: T) -> ServiceResponse<()> {
        match serde_json::to_string(&event) {
            Err(err) => ServiceError::Serde(err).into(),
            Ok(json) => {
                ctx.emit_event(json);
                ServiceResponse::from_succeed(())
            }
        }
    }
}

impl<SDK: ServiceSDK> ExpressionDataFeed for KycService<SDK> {
    fn get_tags(&self, target_address: Address, kyc: String, tag: String) -> Vec<String> {
        println!("get_tags:{}:{}.{}", target_address.as_hex(), kyc, tag);
        vec!["KYC.TAG".to_string()]
    }
}
