use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use cita_trie::MemoryDB;
use core_storage::adapter::memory::MemoryAdapter;
use core_storage::ImplStorage;
use framework::binding::sdk::{DefaultChainQuerier, DefaultServiceSDK};
use framework::binding::state::{GeneralServiceState, MPTTrie};
use protocol::traits::ExecutorParams;
use protocol::types::{Address, MerkleRoot, ServiceContext, ServiceContextParams};

use crate::*;

lazy_static::lazy_static! {
    pub static ref ADMIN_ACCOUNT: Address = Address::from_hex("0x0000000000000000000000000000000000000001").unwrap();
    pub static ref PASSENGER_ACCOUNT: Address = Address::from_hex("0x0000000000000000000000000000000000000002").unwrap();

    pub static ref TIME_1 : u64 = 1599334348000;
    pub static ref TIME_2 : u64 = 1599337001000;
    pub static ref TIME_3 : u64 = 1599338988000;
    pub static ref TIME_4 : u64 = 1599338758000;

}

type TestSDK = DefaultServiceSDK<
    GeneralServiceState<MemoryDB>,
    DefaultChainQuerier<ImplStorage<MemoryAdapter>>,
>;

#[test]
fn test_now() {
    let service = service();
    let now = service.now(context_admin()).succeed_data;

    assert_eq!(*TIME_1, now)
}

#[test]
fn test_now_in_auto_mode() {
    // the service now is in oracle mode
    let mut service = service();
    let now = service.now(context_admin()).succeed_data;
    assert_eq!(*TIME_1, now);

    // oracle now should reject set_timestamp
    service.set_timestamp_hook(&executor_params(*TIME_2));

    let now = service.now(context_admin()).succeed_data;
    assert_eq!(*TIME_1, now);

    service.set_oracle(context_admin(), SetOraclePayload { oracle: false });

    let res = service.feed_time(context_admin(), FeedTimePayload { timestamp: *TIME_3 });
    assert_eq!(res.code, ServiceError::NotOracleMode.code());

    let res = service.feed_time(context_passenger(), FeedTimePayload { timestamp: *TIME_3 });
    assert_eq!(res.code, ServiceError::NotAuthorized.code())
}

#[test]
fn test_now_in_oracle_mode() {
    let mut service = service();

    let now = service.now(context((*ADMIN_ACCOUNT).clone())).succeed_data;
    assert_eq!(*TIME_1, now);

    let res = service.feed_time(context_admin(), FeedTimePayload { timestamp: *TIME_2 });
    assert_eq!(res.code, 0);

    let now = service.now(context((*ADMIN_ACCOUNT).clone())).succeed_data;
    assert_eq!(*TIME_2, now);

    let res = service.feed_time(context_passenger(), FeedTimePayload { timestamp: *TIME_3 });
    assert_eq!(res.code, ServiceError::NotAuthorized.code());

    let now = service.now(context((*ADMIN_ACCOUNT).clone())).succeed_data;
    assert_eq!(*TIME_2, now);

    // oracle now should reject set_timestamp
    service.set_timestamp_hook(&executor_params(*TIME_4));

    let now = service.now(context((*ADMIN_ACCOUNT).clone())).succeed_data;
    assert_eq!(*TIME_2, now);
}

#[test]
fn test_reject_stale_time() {
    let mut service = service();
    service.set_oracle(context_admin(), SetOraclePayload { oracle: false });

    service.set_timestamp_hook(&executor_params(*TIME_3));

    let now = service.now(context((*ADMIN_ACCOUNT).clone())).succeed_data;
    assert_eq!(*TIME_3, now);

    service.set_timestamp_hook(&executor_params(*TIME_2));

    let now = service.now(context((*ADMIN_ACCOUNT).clone())).succeed_data;
    assert_eq!(*TIME_3, now);
}

#[test]
fn test_set_get_admin() {
    let mut service = service();

    let addr = service.get_admin(context_admin());
    assert_eq!(addr.code, 0);
    assert_eq!(addr.succeed_data, (*ADMIN_ACCOUNT).clone());

    let res = service.set_admin(context_passenger(), SetAdminPayload {
        admin: (*PASSENGER_ACCOUNT).clone(),
    });
    assert_eq!(res.code, ServiceError::NotAuthorized.code());

    let res = service.set_admin(context_admin(), SetAdminPayload {
        admin: (*PASSENGER_ACCOUNT).clone(),
    });
    assert_eq!(res.code, 0);

    let addr = service.get_admin(context_admin());
    assert_eq!(addr.code, 0);
    assert_eq!(addr.succeed_data, (*PASSENGER_ACCOUNT).clone());
}

#[test]
fn test_set_get_oracle() {
    let mut service = service();

    let addr = service.get_info(context_admin());
    assert_eq!(addr.code, 0);
    assert_eq!(addr.succeed_data.oracle, true);

    let res = service.set_oracle(context_passenger(), SetOraclePayload { oracle: false });
    assert_eq!(res.code, ServiceError::NotAuthorized.code());

    let res = service.set_oracle(context_admin(), SetOraclePayload { oracle: false });
    assert_eq!(res.code, 0);

    let addr = service.get_info(context_admin());
    assert_eq!(addr.code, 0);
    assert_eq!(addr.succeed_data.oracle, false);
}

fn service() -> TimestampService<TestSDK> {
    let storage = ImplStorage::new(Arc::new(MemoryAdapter::new()));
    let chain_db = DefaultChainQuerier::new(Arc::new(storage));

    let trie = MPTTrie::new(Arc::new(MemoryDB::new(false)));
    let state = GeneralServiceState::new(trie);

    let sdk = DefaultServiceSDK::new(Rc::new(RefCell::new(state)), Rc::new(chain_db));

    let mut service = TimestampService::<TestSDK>::new(sdk);
    service.init_genesis(Genesis {
        // 2020/9/6 3:32:28
        start_time: 1599334348000,
        oracle:     true,
        admin:      (*ADMIN_ACCOUNT).clone(),
    });
    service
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

fn executor_params(timestamp: u64) -> ExecutorParams {
    ExecutorParams {
        state_root: MerkleRoot::default(),
        height: 1,
        timestamp,
        cycles_limit: std::u64::MAX,
        proposer: (*ADMIN_ACCOUNT).clone(),
    }
}

fn context_admin() -> ServiceContext {
    context((*ADMIN_ACCOUNT).clone())
}

fn context_passenger() -> ServiceContext {
    context((*PASSENGER_ACCOUNT).clone())
}
