use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::Arc;

use cita_trie::MemoryDB;
use core_storage::{adapter::memory::MemoryAdapter, ImplStorage};
use framework::binding::sdk::{DefaultChainQuerier, DefaultServiceSDK};
use framework::binding::state::{GeneralServiceState, MPTTrie};
use protocol::types::{Address, Bytes, Hash, Hex, ServiceContext, ServiceContextParams};

use crate::types::{
    ApprovePayload, BurnAssetEvent, BurnAssetPayload, ChangeAdminPayload, CreateAssetPayload,
    GetAllowancePayload, GetAssetPayload, GetBalancePayload, HookTransferFromPayload,
    InitGenesisPayload, IssuerWithBalance, MintAssetEvent, MintAssetPayload, RelayAssetEvent,
    RelayAssetPayload, TransferFromPayload, TransferPayload,
};
use crate::{AssetService, ServiceError};
use kyc::KycService;
use timestamp::TimestampService;
use transfer_quota::TransferQuotaService;

macro_rules! service_call {
    ($service:expr, $method:ident, $ctx:expr, $payload:expr) => {{
        let resp = $service.$method($ctx, $payload);
        if resp.is_error() {
            println!("{}", resp.error_message);
        }
        assert!(!resp.is_error());

        resp.succeed_data
    }};
}

macro_rules! create_asset {
    ($service:expr, $ctx:expr, $supply:expr, $precision:expr) => {{
        service_call!($service, create_asset, $ctx, CreateAssetPayload {
            name:       "meow".to_owned(),
            symbol:     "MIMI".to_owned(),
            admin:      ADMIN_ACCOUNT.clone(),
            supply:     $supply,
            init_mints: vec![IssuerWithBalance {
                addr:    ADMIN_ACCOUNT.clone(),
                balance: $supply,
            }],
            precision:  $precision,
            relayable:  true,
        })
    }};
}

type TestSDK = DefaultServiceSDK<
    GeneralServiceState<MemoryDB>,
    DefaultChainQuerier<ImplStorage<MemoryAdapter>>,
>;

lazy_static::lazy_static! {
    pub static ref ADMIN_ACCOUNT: Address = Address::from_hex("0x0000000000000000000000000000000000000001").unwrap();
    pub static ref USER_1: Address = Address::from_hex("0x0000000000000000000000000000000000000002").unwrap();
    pub static ref ASSET_ID: Hash = Hash::digest(Bytes::from_static(b"test_asset"));
    pub static ref CYCLE_LIMIT: u64 = 1024 * 1024 * 1024;
}

#[test]
fn test_create_asset() {
    let precision = 2;
    let supply = 1024 * 1024;
    let caller = ADMIN_ACCOUNT.clone();

    let mut service = TestService::new();
    let ctx = mock_context(caller.clone());

    // test create_asset
    let asset = create_asset!(service, ctx.clone(), supply, precision);
    let asset_got = service_call!(service, get_asset, ctx.clone(), GetAssetPayload {
        id: asset.id.clone(),
    });
    assert_eq!(asset_got, asset);

    let resp = service_call!(service, get_balance, ctx, GetBalancePayload {
        asset_id: asset.id.clone(),
        user:     caller,
    });
    assert_eq!(resp.balance, supply);
    assert_eq!(resp.asset_id, asset.id);
}

#[test]
fn test_transfer() {
    let mut service = TestService::new();
    let caller = ADMIN_ACCOUNT.clone();
    let ctx = mock_context(caller.clone());

    // mint 10_000 tokens to ADMIN
    let asset = create_asset!(service, ctx.clone(), 10000, 10);

    let recipient = USER_1.clone();

    service_call!(service, transfer, ctx.clone(), TransferPayload {
        asset_id: asset.id.clone(),
        to:       recipient.clone(),
        value:    1024,
        memo:     "test".to_owned(),
    });

    let caller_balance = service_call!(service, get_balance, ctx, GetBalancePayload {
        asset_id: asset.id.clone(),
        user:     caller,
    });
    assert_eq!(caller_balance.balance, asset.supply - 1024);

    let ctx = mock_context(recipient.clone());
    let recipient_balance = service_call!(service, get_balance, ctx, GetBalancePayload {
        asset_id: asset.id,
        user:     recipient,
    });
    assert_eq!(recipient_balance.balance, 1024);
}

#[test]
fn test_approve() {
    let mut service = TestService::new();
    let caller = ADMIN_ACCOUNT.clone();
    let ctx = mock_context(caller.clone());
    let asset = create_asset!(service, ctx.clone(), 1000, 10);

    let recipient = USER_1.clone();

    service_call!(service, approve, ctx.clone(), ApprovePayload {
        asset_id: asset.id.clone(),
        to:       recipient.clone(),
        value:    1024,
        memo:     "test".to_owned(),
    });

    let allowance = service_call!(service, get_allowance, ctx, GetAllowancePayload {
        asset_id: asset.id.clone(),
        grantor:  caller,
        grantee:  recipient.clone(),
    });
    assert_eq!(allowance.asset_id, asset.id);
    assert_eq!(allowance.grantee, recipient);
    assert_eq!(allowance.value, 1024);
}

#[test]
fn test_transfer_from() {
    let mut service = TestService::new();
    let caller = ADMIN_ACCOUNT.clone();
    let ctx = mock_context(caller.clone());
    let asset = create_asset!(service, ctx.clone(), 1000, 10);

    let recipient = USER_1.clone();

    service_call!(service, approve, ctx.clone(), ApprovePayload {
        asset_id: asset.id.clone(),
        to:       recipient.clone(),
        value:    1024,
        memo:     "test".to_owned(),
    });

    let recipient_ctx = mock_context(recipient.clone());

    service_call!(
        service,
        transfer_from,
        recipient_ctx.clone(),
        TransferFromPayload {
            asset_id:  asset.id.clone(),
            sender:    caller.clone(),
            recipient: recipient.clone(),
            value:     24,
            memo:      "test".to_owned(),
        }
    );

    // allowance = 1024 -24 due to 'transfer_from' 24
    let allowance = service_call!(service, get_allowance, ctx.clone(), GetAllowancePayload {
        asset_id: asset.id.clone(),
        grantor:  caller.clone(),
        grantee:  recipient.clone(),
    });
    assert_eq!(allowance.asset_id, asset.id);
    assert_eq!(allowance.grantee, recipient);
    assert_eq!(allowance.value, 1000);

    let sender_balance = service_call!(service, get_balance, ctx, GetBalancePayload {
        asset_id: asset.id.clone(),
        user:     caller,
    });
    assert_eq!(sender_balance.balance, asset.supply - 24);

    let recipient_balance = service_call!(service, get_balance, recipient_ctx, GetBalancePayload {
        asset_id: asset.id,
        user:     recipient,
    });
    assert_eq!(recipient_balance.balance, 24);
}

#[test]
fn test_change_admin() {
    let mut service = TestService::new();
    let caller = USER_1.clone();
    let ctx = mock_context(caller.clone());

    let changed = service.change_admin(ctx, ChangeAdminPayload {
        new_admin: caller.clone(),
        asset_id:  ASSET_ID.clone(),
    });
    assert!(changed.is_error());

    let ctx = mock_context(ADMIN_ACCOUNT.clone());

    service_call!(service, change_admin, ctx, ChangeAdminPayload {
        new_admin: caller.clone(),
        asset_id:  ASSET_ID.clone(),
    });

    let admin = service.admin(&ASSET_ID.clone());
    assert_eq!(admin, caller);
}

#[test]
fn test_mint() {
    let mut service = TestService::new();
    let caller = ADMIN_ACCOUNT.clone();
    let ctx = mock_context(caller);
    let asset = create_asset!(service, ctx.clone(), 10000, 10);

    let recipient = USER_1.clone();

    let payload = MintAssetPayload {
        asset_id: asset.id.clone(),
        to:       recipient.clone(),
        amount:   100,
        proof:    Hex::from_string("0x1122".to_owned()).unwrap(),
        memo:     "".to_owned(),
    };

    let ctx_user_1 = mock_context(USER_1.clone());
    let minted = service.mint(ctx_user_1, payload.clone());
    assert_eq!(minted.code, ServiceError::Unauthorized.code());

    service_call!(service, mint, ctx.clone(), payload);
    // create asset event takes 1 slot
    assert_eq!(ctx.get_events().len(), 2);

    let event: MintAssetEvent = serde_json::from_str(&ctx.get_events()[1].data).expect("event");
    assert_eq!(event.asset_id, asset.id);
    assert_eq!(event.to, recipient);
    assert_eq!(event.amount, 100);

    let recipient_balance = service_call!(service, get_balance, ctx.clone(), GetBalancePayload {
        asset_id: asset.id.clone(),
        user:     recipient,
    });
    assert_eq!(recipient_balance.balance, 100);

    let asset_ret = service_call!(service, get_asset, ctx, GetAssetPayload { id: asset.id });

    assert_eq!(asset_ret.supply, 10100)
}

#[test]
fn test_burn() {
    let mut service = TestService::new();
    let caller = ADMIN_ACCOUNT.clone();
    let ctx = mock_context(ADMIN_ACCOUNT.clone());
    let asset = create_asset!(service, ctx.clone(), 10000, 10);

    let payload = BurnAssetPayload {
        asset_id: asset.id.clone(),
        amount:   100,
        proof:    Hex::from_string("0xaaBB".to_owned()).unwrap(),
        memo:     "".to_owned(),
    };

    service_call!(service, burn, ctx.clone(), payload);

    // contains a create asset event
    assert_eq!(ctx.get_events().len(), 2);
    let event: BurnAssetEvent = serde_json::from_str(&ctx.get_events()[1].data).expect("event");
    assert_eq!(event.asset_id, asset.id);
    assert_eq!(event.from, caller);
    assert_eq!(event.amount, 100);

    let caller_balance = service_call!(service, get_balance, ctx.clone(), GetBalancePayload {
        asset_id: asset.id.clone(),
        user:     caller,
    });
    assert_eq!(caller_balance.balance, asset.supply - 100);

    let asset_ret = service_call!(service, get_asset, ctx, GetAssetPayload { id: asset.id });

    assert_eq!(asset_ret.supply, 9900)
}

#[test]
fn test_relayable() {
    let mut service = TestService::new();
    let caller = ADMIN_ACCOUNT.clone();
    let ctx = mock_context(caller.clone());
    let asset = create_asset!(service, ctx.clone(), 10000, 10);

    let payload = RelayAssetPayload {
        asset_id: asset.id.clone(),
        amount:   100,
        proof:    Hex::from_string("0xaaBB".to_owned()).unwrap(),
        memo:     "".to_owned(),
    };
    service_call!(service, relay, ctx.clone(), payload);

    assert_eq!(ctx.get_events().len(), 3);
    let event: RelayAssetEvent = serde_json::from_str(&ctx.get_events()[2].data).expect("event");
    assert_eq!(event.asset_id, asset.id);
    assert_eq!(event.from, caller);
    assert_eq!(event.amount, 100);

    let caller_balance = service_call!(service, get_balance, ctx.clone(), GetBalancePayload {
        asset_id: asset.id.clone(),
        user:     caller,
    });
    assert_eq!(caller_balance.balance, asset.supply - 100);

    let asset_ret = service_call!(service, get_asset, ctx, GetAssetPayload { id: asset.id });

    assert_eq!(asset_ret.supply, 9900)
}

#[test]
fn test_unrelayable() {
    let caller = ADMIN_ACCOUNT.clone();

    let mut service = TestService::new();
    let ctx = mock_context(caller);

    // test create_asset
    let asset = service
        .create_asset(ctx.clone(), CreateAssetPayload {
            name:       "Cat9".to_owned(),
            symbol:     "MIMI".to_owned(),
            admin:      ADMIN_ACCOUNT.clone(),
            supply:     10000,
            init_mints: vec![IssuerWithBalance {
                addr:    ADMIN_ACCOUNT.clone(),
                balance: 10000,
            }],
            precision:  100,
            relayable:  false,
        })
        .succeed_data;

    let resp = service.relay(ctx, RelayAssetPayload {
        asset_id: asset.id,
        amount:   100,
        proof:    Hex::from_string("0xaaBB".to_owned()).unwrap(),
        memo:     "".to_owned(),
    });

    assert_eq!(resp.code, ServiceError::NotRelayable.code())
}

#[test]
fn test_transfer_to_self() {
    let mut service = TestService::new();
    let caller = ADMIN_ACCOUNT.clone();
    let ctx = mock_context(caller.clone());
    let asset = create_asset!(service, ctx.clone(), 10000, 10);

    service_call!(service, transfer, ctx.clone(), TransferPayload {
        asset_id: asset.id.clone(),
        to:       caller.clone(),
        value:    100,
        memo:     "test".to_owned(),
    });

    let caller_balance = service_call!(service, get_balance, ctx, GetBalancePayload {
        asset_id: asset.id,
        user:     caller,
    });
    assert_eq!(caller_balance.balance, asset.supply);
}

#[test]
fn test_check_format() {
    let caller = ADMIN_ACCOUNT.clone();

    let mut service = TestService::new();
    let ctx = mock_context(caller);

    // test create_asset

    let create_asset_resp = service.create_asset(ctx.clone(), CreateAssetPayload {
        name:       "咪咪".to_owned(),
        symbol:     "MIMI".to_owned(),
        admin:      ADMIN_ACCOUNT.clone(),
        supply:     10000,
        init_mints: vec![IssuerWithBalance {
            addr:    ADMIN_ACCOUNT.clone(),
            balance: 10000,
        }],
        precision:  100,
        relayable:  true,
    });

    assert!(create_asset_resp.is_error());

    let create_asset_resp = service.create_asset(ctx.clone(), CreateAssetPayload {
        name:       "we1l".to_owned(),
        symbol:     "😺".to_owned(),
        admin:      ADMIN_ACCOUNT.clone(),
        supply:     10000,
        init_mints: vec![IssuerWithBalance {
            addr:    ADMIN_ACCOUNT.clone(),
            balance: 10000,
        }],
        precision:  100,
        relayable:  true,
    });

    assert!(create_asset_resp.is_error());

    let create_asset_resp = service.create_asset(ctx.clone(), CreateAssetPayload {
        name:       "we1l".to_owned(),
        symbol:     "m".to_owned(),
        admin:      ADMIN_ACCOUNT.clone(),
        supply:     10000,
        init_mints: vec![IssuerWithBalance {
            addr:    ADMIN_ACCOUNT.clone(),
            balance: 10000,
        }],
        precision:  100,
        relayable:  true,
    });

    assert!(create_asset_resp.is_error());

    let create_asset_resp = service.create_asset(ctx.clone(), CreateAssetPayload {
        name:       "_we1l".to_owned(),
        symbol:     "M".to_owned(),
        admin:      ADMIN_ACCOUNT.clone(),
        supply:     10000,
        init_mints: vec![IssuerWithBalance {
            addr:    ADMIN_ACCOUNT.clone(),
            balance: 10000,
        }],
        precision:  100,
        relayable:  true,
    });

    assert!(create_asset_resp.is_error());

    let create_asset_resp = service.create_asset(ctx.clone(), CreateAssetPayload {
        name:       "we1l_".to_owned(),
        symbol:     "M".to_owned(),
        admin:      ADMIN_ACCOUNT.clone(),
        supply:     10000,
        init_mints: vec![IssuerWithBalance {
            addr:    ADMIN_ACCOUNT.clone(),
            balance: 10000,
        }],
        precision:  100,
        relayable:  true,
    });

    assert!(create_asset_resp.is_error());

    let create_asset_resp = service.create_asset(ctx.clone(), CreateAssetPayload {
        name:       " we1l".to_owned(),
        symbol:     "M".to_owned(),
        admin:      ADMIN_ACCOUNT.clone(),
        supply:     10000,
        init_mints: vec![IssuerWithBalance {
            addr:    ADMIN_ACCOUNT.clone(),
            balance: 10000,
        }],
        precision:  100,
        relayable:  true,
    });

    assert!(create_asset_resp.is_error());

    let create_asset_resp = service.create_asset(ctx.clone(), CreateAssetPayload {
        name:       "we1l ".to_owned(),
        symbol:     "M".to_owned(),
        admin:      ADMIN_ACCOUNT.clone(),
        supply:     10000,
        init_mints: vec![IssuerWithBalance {
            addr:    ADMIN_ACCOUNT.clone(),
            balance: 10000,
        }],
        precision:  100,
        relayable:  true,
    });

    assert!(create_asset_resp.is_error());

    let create_asset_resp = service.create_asset(ctx, CreateAssetPayload {
        name:       "1we1l ".to_owned(),
        symbol:     "M".to_owned(),
        admin:      ADMIN_ACCOUNT.clone(),
        supply:     10000,
        init_mints: vec![IssuerWithBalance {
            addr:    ADMIN_ACCOUNT.clone(),
            balance: 10000,
        }],
        precision:  100,
        relayable:  true,
    });

    assert!(create_asset_resp.is_error());
}

#[test]
fn test_multiple_issuers_genesis() {
    let mut service = TestService::new();
    let caller = ADMIN_ACCOUNT.clone();
    let ctx = mock_context(caller);

    let asset = service_call!(service, create_asset, ctx.clone(), CreateAssetPayload {
        name:       "meow".to_owned(),
        symbol:     "MIMI".to_owned(),
        admin:      ADMIN_ACCOUNT.clone(),
        supply:     10000,
        init_mints: vec![
            IssuerWithBalance {
                addr:    ADMIN_ACCOUNT.clone(),
                balance: 5000,
            },
            IssuerWithBalance {
                addr:    USER_1.clone(),
                balance: 5000,
            }
        ],
        precision:  10,
        relayable:  true,
    });

    for addr in vec![ADMIN_ACCOUNT.clone(), USER_1.clone()] {
        let account = service_call!(service, get_balance, ctx.clone(), GetBalancePayload {
            asset_id: asset.id.clone(),
            user:     addr,
        });
        assert_eq!(account.balance, 5000);
    }
}

#[test]
#[should_panic]
fn test_genesis_issuers_balance_overflow() {
    let storage = ImplStorage::new(Arc::new(MemoryAdapter::new()));
    let chain_db = DefaultChainQuerier::new(Arc::new(storage));
    let chain_db = Rc::new(chain_db);

    let trie = MPTTrie::new(Arc::new(MemoryDB::new(false)));
    let state = GeneralServiceState::new(trie);
    let state = Rc::new(RefCell::new(state));

    let timestamp_service = TimestampService::new(DefaultServiceSDK::new(
        Rc::clone(&state),
        Rc::clone(&chain_db),
    ));

    let kyc_service = KycService::new(DefaultServiceSDK::new(
        Rc::clone(&state),
        Rc::clone(&chain_db),
    ));

    let transfer_quota_service = TransferQuotaService::new(
        DefaultServiceSDK::new(Rc::clone(&state), Rc::clone(&chain_db)),
        kyc_service,
        timestamp_service,
    );

    let mut asset_service = AssetService::new(
        DefaultServiceSDK::new(Rc::clone(&state), Rc::clone(&chain_db)),
        Some(transfer_quota_service),
    );

    asset_service.init_genesis(InitGenesisPayload {
        id:         ASSET_ID.clone(),
        name:       "native_token".to_owned(),
        symbol:     "NT".to_owned(),
        supply:     1000,
        precision:  10,
        init_mints: vec![
            IssuerWithBalance::new(ADMIN_ACCOUNT.clone(), u64::max_value()),
            IssuerWithBalance::new(USER_1.clone(), 500),
        ],
        admin:      ADMIN_ACCOUNT.clone(),
        relayable:  true,
    });
}

#[test]
#[should_panic]
fn test_genesis_issuers_balance_not_equal_to_supply() {
    let storage = ImplStorage::new(Arc::new(MemoryAdapter::new()));
    let chain_db = DefaultChainQuerier::new(Arc::new(storage));
    let chain_db = Rc::new(chain_db);

    let trie = MPTTrie::new(Arc::new(MemoryDB::new(false)));
    let state = GeneralServiceState::new(trie);
    let state = Rc::new(RefCell::new(state));

    let timestamp_service = TimestampService::new(DefaultServiceSDK::new(
        Rc::clone(&state),
        Rc::clone(&chain_db),
    ));

    let kyc_service = KycService::new(DefaultServiceSDK::new(
        Rc::clone(&state),
        Rc::clone(&chain_db),
    ));

    let transfer_quota_service = TransferQuotaService::new(
        DefaultServiceSDK::new(Rc::clone(&state), Rc::clone(&chain_db)),
        kyc_service,
        timestamp_service,
    );

    let mut asset_service = AssetService::new(
        DefaultServiceSDK::new(Rc::clone(&state), Rc::clone(&chain_db)),
        Some(transfer_quota_service),
    );

    asset_service.init_genesis(InitGenesisPayload {
        id:         ASSET_ID.clone(),
        name:       "native_token".to_owned(),
        symbol:     "NT".to_owned(),
        supply:     1000,
        precision:  10,
        init_mints: vec![
            IssuerWithBalance::new(ADMIN_ACCOUNT.clone(), 300),
            IssuerWithBalance::new(USER_1.clone(), 500),
        ],
        admin:      ADMIN_ACCOUNT.clone(),
        relayable:  true,
    });
}

#[test]
fn test_hook_transfer_from_emit_no_event() {
    let mut service = TestService::new();
    let recipient = USER_1.clone();

    let ctx = {
        let params = ServiceContextParams {
            tx_hash:         None,
            nonce:           None,
            cycles_limit:    *CYCLE_LIMIT,
            cycles_price:    1,
            cycles_used:     Rc::new(RefCell::new(0)),
            caller:          recipient.clone(),
            height:          1,
            timestamp:       0,
            service_name:    "service_name".to_owned(),
            service_method:  "service_method".to_owned(),
            service_payload: "service_payload".to_owned(),
            extra:           Some(Bytes::from_static(b"governance")),
            events:          Rc::new(RefCell::new(vec![])),
        };

        ServiceContext::new(params)
    };

    let admin = ADMIN_ACCOUNT.clone();
    service.hook_transfer_from(ctx.clone(), HookTransferFromPayload {
        sender:    admin.clone(),
        recipient: recipient.clone(),
        value:     24,
        memo:      "test".to_owned(),
    });
    assert_eq!(ctx.get_events().len(), 0);

    let sender_balance = service_call!(service, get_balance, ctx.clone(), GetBalancePayload {
        asset_id: TestService::genesis().id,
        user:     admin,
    });
    assert_eq!(sender_balance.balance, 500 - 24);

    let recipient_balance = service_call!(service, get_balance, ctx, GetBalancePayload {
        asset_id: TestService::genesis().id,
        user:     recipient,
    });
    assert_eq!(recipient_balance.balance, 500 + 24);
}

struct TestService(
    AssetService<
        TestSDK,
        TransferQuotaService<TestSDK, KycService<TestSDK>, TimestampService<TestSDK>>,
    >,
);

impl Deref for TestService {
    type Target = AssetService<
        TestSDK,
        TransferQuotaService<TestSDK, KycService<TestSDK>, TimestampService<TestSDK>>,
    >;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TestService {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TestService {
    // fn new() -> TestService<TestSDK,
    // TransferQuotaService<TestSDK,KycService<TestSDK>,TimestampService<TestSDK> >>
    // {
    fn new() -> Self {
        let storage = ImplStorage::new(Arc::new(MemoryAdapter::new()));
        let chain_db = DefaultChainQuerier::new(Arc::new(storage));
        let chain_db = Rc::new(chain_db);

        let trie = MPTTrie::new(Arc::new(MemoryDB::new(false)));
        let state = GeneralServiceState::new(trie);
        let state = Rc::new(RefCell::new(state));

        let mut asset_service = AssetService::new(
            DefaultServiceSDK::new(Rc::clone(&state), Rc::clone(&chain_db)),
            None,
        );

        asset_service.init_genesis(TestService::genesis());

        TestService(asset_service)
    }

    fn genesis() -> InitGenesisPayload {
        InitGenesisPayload {
            id:         ASSET_ID.clone(),
            name:       "native_token".to_owned(),
            symbol:     "NT".to_owned(),
            supply:     1000,
            precision:  10,
            init_mints: vec![
                IssuerWithBalance::new(ADMIN_ACCOUNT.clone(), 500),
                IssuerWithBalance::new(USER_1.clone(), 500),
            ],
            admin:      ADMIN_ACCOUNT.clone(),
            relayable:  true,
        }
    }
}

fn mock_context(caller: Address) -> ServiceContext {
    let params = ServiceContextParams {
        tx_hash: None,
        nonce: None,
        cycles_limit: *CYCLE_LIMIT,
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
