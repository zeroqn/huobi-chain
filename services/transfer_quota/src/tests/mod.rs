use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;

use cita_trie::MemoryDB;
use core_storage::adapter::memory::MemoryAdapter;
use core_storage::ImplStorage;
use framework::binding::sdk::{DefaultChainQuerier, DefaultServiceSDK};
use framework::binding::state::{GeneralServiceState, MPTTrie};
use protocol::traits::ServiceResponse;
use protocol::types::{Address, Bytes, Hash, ServiceContext, ServiceContextParams};

use kyc::{
    types::{
        FixedTagList, Genesis as KYC_Genesis, GetUserTags, OrgName, TagName, TagString,
        UpdateUserTags,
    },
    KycService,
};
use timestamp::{FeedTimePayload, Genesis as TIME_STAMP_Genesis, TimestampService};

use crate::types::{
    AssetConfig, ChangeAssetConfigPayload, Genesis, GenesisAssetConfig, QuotaTransferPayload,
    QuotaType, Record, Rule,
};
use crate::{ServiceError, TransferQuotaService};
use std::ops::{Deref, DerefMut};

lazy_static::lazy_static! {
    pub static ref ADMIN_ACCOUNT: Address = Address::from_hex("0x0000000000000000000000000000000000000001").unwrap();
    pub static ref PASSENGER_ACCOUNT: Address = Address::from_hex("0x0000000000000000000000000000000000000002").unwrap();
    pub static ref USER_1 : Address = Address::from_hex("0x0000000000000000000000000000000000000003").unwrap();
    // 2020/9/7 1:41:13
    pub static ref TIME_ORIGINAL : u64 = 1599414073000;
    // 2020/9/7 1:45:13
    pub static ref TIME_SAME_DAY : u64 = 1599414313000;
    // 2020/9/8 1:45:13
    pub static ref TIME_SAME_MONTH : u64 = 1599500713000;
    // 2020/10/8 1:45:13
    pub static ref TIME_SAME_YEAR : u64 = 1602092713000;
    // 2021/10/8 1:45:13
    pub static ref TIME_DIFF_YEAR : u64 = 1633628713000;

    pub static ref ASSET_ID : Hash = Hash::digest(Bytes::from_static(b"test_asset"));

    pub static ref ERROR_EXCEED_SINGLE_BILL : u64 =  ServiceError::QuotaExceed(QuotaType::SingleBill, 0, 0, 0).code();

    pub static ref ERROR_EXCEED_DAILY : u64 =  ServiceError::QuotaExceed(QuotaType::Daily, 0, 0, 0).code();

    pub static ref ERROR_EXCEED_MONTHLY : u64 =  ServiceError::QuotaExceed(QuotaType::Monthly, 0, 0, 0).code();

    pub static ref ERROR_EXCEED_YEARLY : u64 =  ServiceError::QuotaExceed(QuotaType::Yearly, 0, 0, 0).code();

}

const L1: &str = "L1";
const L2: &str = "L2";
const L3: &str = "L3";
const ANY_TAG: &str = "ANY_TAG";

type TestSDKType = DefaultServiceSDK<
    GeneralServiceState<MemoryDB>,
    DefaultChainQuerier<ImplStorage<MemoryAdapter>>,
>;

type KycServiceType = KycService<TestSDKType>;
type TimestampServiceType = TimestampService<TestSDKType>;

type TransferQuotaServiceType =
    TransferQuotaService<TestSDKType, KycServiceType, TimestampServiceType>;

macro_rules! quota_transfer {
    ($transfer_quota_service: expr, $amount: expr) => {{
        $transfer_quota_service.quota_transfer(context_admin(), QuotaTransferPayload {
            asset_id: (*ASSET_ID).clone(),
            address:  (*USER_1).clone(),
            amount:   $amount,
        })
    }};
}

// | table | single_bill| daily | monthly   | yearly    |
// |  ---- | ---------- | ------| -------   | -------   |
// | kyc L1 | 1_500     | 2_000 | 60_000    | 720_000   |
// | kyc L2 | 3_000     | 5_000 | 150_000   |1_800_000  |
// | kyc L3 | 50_000    |  MAX  |      MAX   |    MAX   |
// | kyc !NULL| 500     | 1_000 | 30_000    | 300_000   |
// | kyc NULL|  100     | 500   | 10_000    | 15_000    |

#[test]
fn test_single_bill() {
    let mut services = TransferQuotaServiceCollection::default();

    services.set_user_1_tags(L1);

    let res = quota_transfer!(services, 1000);
    assert_eq!(res.code, 0);

    services.set_user_1_tags(L1);

    let res = quota_transfer!(services, 2000);
    assert_eq!(res.code, *ERROR_EXCEED_SINGLE_BILL);
}

#[test]
fn test_daily() {
    let mut services = TransferQuotaServiceCollection::default();

    services.set_user_1_tags(L1);

    let res = quota_transfer!(services, 1000);
    assert_eq!(res.code, 0);
    let res = quota_transfer!(services, 1000);
    assert_eq!(res.code, 0);

    let res = quota_transfer!(services, 1);
    assert_eq!(res.code, *ERROR_EXCEED_DAILY);

    // go to same day
    services.set_timestamp(&TIME_SAME_DAY);

    let res = quota_transfer!(services, 1);
    assert_eq!(res.code, *ERROR_EXCEED_DAILY);

    //  go to another day
    services.set_timestamp(&TIME_SAME_MONTH);

    let res = quota_transfer!(services, 500);
    assert_eq!(res.code, 0);
}

#[test]
fn test_monthly() {
    let mut services = TransferQuotaServiceCollection::default();

    services.set_user_1_tags(L1);

    services.set_record((*ASSET_ID).clone(), (*USER_1).clone(), Record {
        last_op_time:        *TIME_ORIGINAL,
        daily_used_amount:   0,
        monthly_used_amount: 59_000,
        yearly_used_amount:  0,
    });

    let res = quota_transfer!(services, 1000);
    assert_eq!(res.code, 0);

    let res = quota_transfer!(services, 1);
    assert_eq!(res.code, *ERROR_EXCEED_MONTHLY);

    // go to same day
    services.set_timestamp(&TIME_SAME_DAY);

    let res = quota_transfer!(services, 1);
    assert_eq!(res.code, *ERROR_EXCEED_MONTHLY);

    // go to same month
    services.set_timestamp(&TIME_SAME_MONTH);

    let res = quota_transfer!(services, 1);
    assert_eq!(res.code, *ERROR_EXCEED_MONTHLY);

    //  go to another day of month
    services.set_timestamp(&TIME_SAME_YEAR);

    let res = quota_transfer!(services, 500);
    assert_eq!(res.code, 0);
}

#[test]
fn test_yearly() {
    let mut services = TransferQuotaServiceCollection::default();

    services.set_user_1_tags(L1);

    services.set_record((*ASSET_ID).clone(), (*USER_1).clone(), Record {
        last_op_time:        *TIME_ORIGINAL,
        daily_used_amount:   0,
        monthly_used_amount: 0,
        yearly_used_amount:  719_000,
    });

    let res = quota_transfer!(services, 1000);
    assert_eq!(res.code, 0);

    let res = quota_transfer!(services, 1);
    assert_eq!(res.code, *ERROR_EXCEED_YEARLY);

    // go to same day
    services.set_timestamp(&TIME_SAME_DAY);

    let res = quota_transfer!(services, 1);
    assert_eq!(res.code, *ERROR_EXCEED_YEARLY);

    // go to same month
    services.set_timestamp(&TIME_SAME_MONTH);

    let res = quota_transfer!(services, 1);
    assert_eq!(res.code, *ERROR_EXCEED_YEARLY);

    // go to same year
    services.set_timestamp(&TIME_SAME_YEAR);

    let res = quota_transfer!(services, 1);
    assert_eq!(res.code, *ERROR_EXCEED_YEARLY);

    //  go to another day of year
    services.set_timestamp(&TIME_DIFF_YEAR);

    let res = quota_transfer!(services, 500);
    assert_eq!(res.code, 0);
}

#[test]
fn test_kyc_change() {
    let mut services = TransferQuotaServiceCollection::default();

    services.set_user_1_tags(L1);

    services.set_record((*ASSET_ID).clone(), (*USER_1).clone(), Record {
        last_op_time:        *TIME_ORIGINAL,
        daily_used_amount:   2000,
        monthly_used_amount: 0,
        yearly_used_amount:  0,
    });

    let res = quota_transfer!(services, 1);
    assert_eq!(res.code, *ERROR_EXCEED_DAILY);

    services.set_user_1_tags(L2);

    let res = quota_transfer!(services, 1);
    assert_eq!(res.code, 0);

    //======

    services.set_user_1_tags(L2);

    services.set_record((*ASSET_ID).clone(), (*USER_1).clone(), Record {
        last_op_time:        *TIME_ORIGINAL,
        daily_used_amount:   5000,
        monthly_used_amount: 0,
        yearly_used_amount:  0,
    });

    let res = quota_transfer!(services, 1);
    assert_eq!(res.code, *ERROR_EXCEED_DAILY);

    services.set_user_1_tags(L3);

    let res = quota_transfer!(services, 1);
    assert_eq!(res.code, 0);

    //======

    services.set_user_1_tags(ANY_TAG);

    services.set_record((*ASSET_ID).clone(), (*USER_1).clone(), Record {
        last_op_time:        *TIME_ORIGINAL,
        daily_used_amount:   1000,
        monthly_used_amount: 0,
        yearly_used_amount:  0,
    });

    let res = quota_transfer!(services, 1);
    assert_eq!(res.code, *ERROR_EXCEED_DAILY);

    services.set_user_1_tags(L1);

    let res = quota_transfer!(services, 1);
    assert_eq!(res.code, 0);

    //======

    services.clear_user_1_tags();

    let res = services.get_user_tags(context_admin(), GetUserTags {
        org_name: OrgName::from_str("Huobi").unwrap(),
        user:     USER_1.clone(),
    });
    assert!(res.succeed_data.is_empty());

    services.set_record((*ASSET_ID).clone(), (*USER_1).clone(), Record {
        last_op_time:        *TIME_ORIGINAL,
        daily_used_amount:   400,
        monthly_used_amount: 0,
        yearly_used_amount:  0,
    });

    let res = quota_transfer!(services, 100);
    assert_eq!(res.code, 0);

    let res = quota_transfer!(services, 1);
    assert_eq!(res.code, *ERROR_EXCEED_SINGLE_BILL);
}

#[test]
fn test_asset_config_activation() {
    let mut services = TransferQuotaServiceCollection::default();

    services.set_user_1_tags(L1);

    // change to inactivated
    services.change_asset_config(context_admin(), ChangeAssetConfigPayload {
        asset_id:     ASSET_ID.clone(),
        asset_config: AssetConfig {
            admin:              ADMIN_ACCOUNT.clone(),
            activated:          false,
            single_bill_quota:  vec![
                Rule {
                    kyc_expr: "!Huobi.level@`NULL`".to_string(),
                    // quota while kyc_expr returns true
                    quota:    1,
                },
                Rule {
                    kyc_expr: "Huobi.level@`NULL`".to_string(),
                    // quota while kyc_expr returns true
                    quota:    1,
                },
            ],
            daily_quota_rule:   vec![
                Rule {
                    kyc_expr: "!Huobi.level@`NULL`".to_string(),
                    // quota while kyc_expr returns true
                    quota:    1,
                },
                Rule {
                    kyc_expr: "Huobi.level@`NULL`".to_string(),
                    // quota while kyc_expr returns true
                    quota:    1,
                },
            ],
            monthly_quota_rule: vec![
                Rule {
                    kyc_expr: "!Huobi.level@`NULL`".to_string(),
                    // quota while kyc_expr returns true
                    quota:    1,
                },
                Rule {
                    kyc_expr: "Huobi.level@`NULL`".to_string(),
                    // quota while kyc_expr returns true
                    quota:    1,
                },
            ],
            yearly_quota_rule:  vec![
                Rule {
                    kyc_expr: "!Huobi.level@`NULL`".to_string(),
                    // quota while kyc_expr returns true
                    quota:    1,
                },
                Rule {
                    kyc_expr: "Huobi.level@`NULL`".to_string(),
                    // quota while kyc_expr returns true
                    quota:    1,
                },
            ],
        },
    });

    // this exceed all quota of L1
    let res = quota_transfer!(services, 100000000);
    assert_eq!(res.code, 0);
}

struct TransferQuotaServiceCollection(pub TransferQuotaServiceType);

impl Deref for TransferQuotaServiceCollection {
    type Target = TransferQuotaServiceType;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TransferQuotaServiceCollection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for TransferQuotaServiceCollection {
    fn default() -> Self {
        let storage = ImplStorage::new(Arc::new(MemoryAdapter::new()));
        let chain_db = DefaultChainQuerier::new(Arc::new(storage));
        let chain_db = Rc::new(chain_db);

        let trie = MPTTrie::new(Arc::new(MemoryDB::new(false)));
        let state = GeneralServiceState::new(trie);
        let state = Rc::new(RefCell::new(state));

        let sdk = DefaultServiceSDK::new(Rc::clone(&state), Rc::clone(&chain_db));

        let kyc_service = KycService::new(sdk);

        let sdk_2 = DefaultServiceSDK::new(Rc::clone(&state), Rc::clone(&chain_db));

        let timestamp_service = TimestampService::new(sdk_2);

        let sdk_3 = DefaultServiceSDK::new(Rc::clone(&state), Rc::clone(&chain_db));

        let transfer_quota_service =
            TransferQuotaService::new(sdk_3, kyc_service, timestamp_service);

        let mut ret = TransferQuotaServiceCollection(transfer_quota_service);
        ret.prepare_transfer_quota();
        ret.prepare_kyc();
        ret.prepare_timestamp();
        ret
    }
}

impl TransferQuotaServiceCollection {
    fn prepare_transfer_quota(&mut self) {
        self.init_genesis(Genesis {
            admin:  (*ADMIN_ACCOUNT).clone(),
            config: vec![GenesisAssetConfig {
                asset_id:     (*ASSET_ID).clone(),
                asset_config: AssetConfig {
                    admin:              (*ADMIN_ACCOUNT).clone(),
                    activated:          true,
                    single_bill_quota:  vec![
                        Rule {
                            kyc_expr: "Huobi.level@`L1`".to_string(),
                            // quota while kyc_expr returns true
                            quota:    1_500,
                        },
                        Rule {
                            kyc_expr: "Huobi.level@`L2`".to_string(),
                            // quota while kyc_expr returns true
                            quota:    3_000,
                        },
                        Rule {
                            kyc_expr: "Huobi.level@`L3`".to_string(),
                            // quota while kyc_expr returns true
                            quota:    50_000,
                        },
                        Rule {
                            kyc_expr: "!Huobi.level@`NULL`".to_string(),
                            // quota while kyc_expr returns true
                            quota:    500,
                        },
                        Rule {
                            kyc_expr: "Huobi.level@`NULL`".to_string(),
                            // quota while kyc_expr returns true
                            quota:    100,
                        },
                    ],
                    // the expr will stop while first eval gets true
                    daily_quota_rule:   vec![
                        Rule {
                            kyc_expr: "Huobi.level@`L1`".to_string(),
                            // quota while kyc_expr returns true
                            quota:    2_000,
                        },
                        Rule {
                            kyc_expr: "Huobi.level@`L2`".to_string(),
                            // quota while kyc_expr returns true
                            quota:    5_000,
                        },
                        Rule {
                            kyc_expr: "Huobi.level@`L3`".to_string(),
                            // quota while kyc_expr returns true
                            quota:    u64::max_value(),
                        },
                        Rule {
                            kyc_expr: "!Huobi.level@`NULL`".to_string(),
                            // quota while kyc_expr returns true
                            quota:    1_000,
                        },
                        Rule {
                            kyc_expr: "Huobi.level@`NULL`".to_string(),
                            // quota while kyc_expr returns true
                            quota:    500,
                        },
                    ],
                    monthly_quota_rule: vec![
                        Rule {
                            kyc_expr: "Huobi.level@`L1`".to_string(),
                            // quota while kyc_expr returns true
                            quota:    60_000,
                        },
                        Rule {
                            kyc_expr: "Huobi.level@`L2`".to_string(),
                            // quota while kyc_expr returns true
                            quota:    150_000,
                        },
                        Rule {
                            kyc_expr: "Huobi.level@`L3`".to_string(),
                            // quota while kyc_expr returns true
                            quota:    u64::max_value(),
                        },
                        Rule {
                            kyc_expr: "!Huobi.level@`NULL`".to_string(),
                            // quota while kyc_expr returns true
                            quota:    30_000,
                        },
                        Rule {
                            kyc_expr: "Huobi.level@`NULL`".to_string(),
                            // quota while kyc_expr returns true
                            quota:    10_000,
                        },
                    ],
                    yearly_quota_rule:  vec![
                        Rule {
                            kyc_expr: "Huobi.level@`L1`".to_string(),
                            // quota while kyc_expr returns true
                            quota:    720_000,
                        },
                        Rule {
                            kyc_expr: "Huobi.level@`L2`".to_string(),
                            // quota while kyc_expr returns true
                            quota:    1_800_000,
                        },
                        Rule {
                            kyc_expr: "Huobi.level@`L3`".to_string(),
                            // quota while kyc_expr returns true
                            quota:    u64::max_value(),
                        },
                        Rule {
                            kyc_expr: "!Huobi.level@`NULL`".to_string(),
                            // quota while kyc_expr returns true
                            quota:    300_000,
                        },
                        Rule {
                            kyc_expr: "Huobi.level@`NULL`".to_string(),
                            // quota while kyc_expr returns true
                            quota:    15_000,
                        },
                    ],
                },
            }],
        });
        // give a TIME_ORIGINAL
        self.set_record((*ASSET_ID).clone(), (*USER_1).clone(), Record {
            last_op_time:        *TIME_ORIGINAL,
            daily_used_amount:   0,
            monthly_used_amount: 0,
            yearly_used_amount:  0,
        });
    }

    fn prepare_kyc(&mut self) {
        self.kyc_service.init_genesis(KYC_Genesis {
            org_name:        OrgName::from_str("Huobi").unwrap(),
            org_description: "description".to_owned(),
            org_admin:       (*ADMIN_ACCOUNT).clone(),
            supported_tags:  vec![TagName::from_str("level").unwrap()],
            service_admin:   (*ADMIN_ACCOUNT).clone(),
        });

        // set_user_1_tags(kyc, "L1");
    }

    fn prepare_timestamp(&mut self) {
        self.timestamp_service.init_genesis(TIME_STAMP_Genesis {
            start_time: *TIME_ORIGINAL,
            oracle:     true,
            admin:      ADMIN_ACCOUNT.clone(),
        })
    }

    fn set_user_1_tags(&mut self, level: &str) {
        let mut tags: HashMap<TagName, FixedTagList> = HashMap::new();
        tags.insert(
            TagName::from_str("level").unwrap(),
            FixedTagList::from_vec(vec![TagString::from_str(level).unwrap()]).unwrap(),
        );
        let use_tags = UpdateUserTags {
            org_name: OrgName::from_str("Huobi").unwrap(),
            user: (*USER_1).clone(),
            tags,
        };

        let res = self.kyc_service.update_user_tags(context_admin(), use_tags);
        assert_eq!(res.code, 0)
    }

    fn clear_user_1_tags(&mut self) {
        let tags: HashMap<TagName, FixedTagList> = HashMap::new();

        let use_tags = UpdateUserTags {
            org_name: OrgName::from_str("Huobi").unwrap(),
            user: (*USER_1).clone(),
            tags,
        };

        let res = self.kyc_service.update_user_tags(context_admin(), use_tags);
        assert_eq!(res.code, 0)
    }

    fn set_timestamp(&mut self, time: &u64) {
        let res = self
            .timestamp_service
            .feed_time(context_admin(), FeedTimePayload { timestamp: *time });
        assert_eq!(res.code, 0)
    }

    fn get_user_tags(
        &self,
        ctx: ServiceContext,
        payload: GetUserTags,
    ) -> ServiceResponse<HashMap<TagName, FixedTagList>> {
        self.kyc_service.get_user_tags(ctx, payload)
    }
}

fn context(caller: Address) -> ServiceContext {
    let params = ServiceContextParams {
        tx_hash: None,
        nonce: None,
        cycles_limit: 1024 * 1024 * 1024,
        cycles_price: 1,
        cycles_used: Rc::new(RefCell::new(0)),
        caller,
        height: 1,
        timestamp: 0,
        service_name: "service_name".to_owned(),
        service_method: "service_method".to_owned(),
        service_payload: "service_payload".to_owned(),
        extra: None,
        events: Rc::new(RefCell::new(vec![])),
    };

    ServiceContext::new(params)
}

fn context_admin() -> ServiceContext {
    context((*ADMIN_ACCOUNT).clone())
}
