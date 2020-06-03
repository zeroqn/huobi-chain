#[macro_use]
pub mod macros;

pub mod duktape;

use std::cell::RefCell;
use std::io::Read;
use std::rc::Rc;
use std::sync::Arc;

use async_trait::async_trait;
use cita_trie::MemoryDB;

use framework::binding::sdk::{DefalutServiceSDK, DefaultChainQuerier};
use framework::binding::state::{GeneralServiceState, MPTTrie};
use protocol::traits::{Context, Dispatcher, ServiceResponse, Storage};
use protocol::types::{
    Address, Block, Hash, Proof, Receipt, ServiceContext, ServiceContextParams, SignedTransaction,
};
use protocol::{Bytes, ProtocolResult};

use crate::types::{DeployPayload, ExecPayload, GetContractPayload, InterpreterType};
use crate::RiscvService;

type TestRiscvService = RiscvService<
    DefalutServiceSDK<
        GeneralServiceState<MemoryDB>,
        DefaultChainQuerier<MockStorage>,
        MockDispatcher,
    >,
>;

thread_local! {
    static RISCV_SERVICE: RefCell<TestRiscvService> = RefCell::new(new_riscv_service());
}

fn with_dispatcher_service<R: for<'a> serde::Deserialize<'a> + Default>(
    f: impl FnOnce(&mut TestRiscvService) -> ServiceResponse<R>,
) -> R {
    RISCV_SERVICE.with(|cell| {
        let mut service = cell.borrow_mut();

        let resp = f(&mut service);
        assert!(!resp.is_error());

        resp.succeed_data
    })
}

#[test]
fn test_deploy_and_run() {
    let cycles_limit = 0x99_9999; // 1024 * 1024 * 1024; // 1073741824
    let caller = Address::from_hex("0x755cdba6ae4f479f7164792b318b2a06c759833b").unwrap();
    let tx_hash =
        Hash::from_hex("0x412a6c54cf3d3dbb16b49c34e6cd93d08a245298032eb975ee51105b4c296828")
            .unwrap();
    let nonce =
        Hash::from_hex("0x0000000000000000000000000000000000000000000000000000000000000000")
            .unwrap();
    let context = mock_context(cycles_limit, caller, tx_hash, nonce);

    let mut service = new_riscv_service();

    let code = {
        let mut file = std::fs::File::open("src/tests/simple_storage").unwrap();
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();
        let buffer = Bytes::from(buffer);
        hex::encode(buffer.as_ref())
    };

    let deploy_result = service!(service, deploy, context.clone(), DeployPayload {
        code:      code.clone(),
        intp_type: InterpreterType::Binary,
        init_args: "set k init".into(),
    });
    assert_eq!(&deploy_result.init_ret, "");

    // test get_contract
    let address = deploy_result.address;
    let get_contract_resp = service!(service, get_contract, context.clone(), GetContractPayload {
        address:      address.clone(),
        get_code:     true,
        storage_keys: vec![hex::encode("k"), "".to_owned(), "3a".to_owned()],
    });
    assert_eq!(&get_contract_resp.code, &code);
    assert_eq!(&get_contract_resp.storage_values, &vec![
        hex::encode("init"),
        "".to_owned(),
        "".to_owned()
    ]);

    let exec_result = service!(service, call, context.clone(), ExecPayload {
        address: address.clone(),
        args:    "get k".into(),
    });
    assert_eq!(&exec_result, "init");

    let exec_result = service!(service, exec, context.clone(), ExecPayload {
        address: address.clone(),
        args:    "set k v".into(),
    });
    assert_eq!(&exec_result, "");

    let exec_result = service!(service, exec, context.clone(), ExecPayload {
        address: address.clone(),
        args:    "get k".into(),
    });
    assert_eq!(&exec_result, "v");

    // wrong command
    let exec_result = service.exec(context.clone(), ExecPayload {
        address: address.clone(),
        args:    "clear k v".into(),
    });
    assert!(exec_result.is_error());

    // wrong command 2
    let exec_result = service.exec(context, ExecPayload {
        address,
        args: "set k".into(),
    });
    assert!(exec_result.is_error());
}

struct MockDispatcher;

impl Dispatcher for MockDispatcher {
    fn read(&self, _context: ServiceContext) -> ServiceResponse<String> {
        unimplemented!()
    }

    fn write(&self, context: ServiceContext) -> ServiceResponse<String> {
        let payload: ExecPayload =
            serde_json::from_str(context.get_payload()).expect("dispatcher payload");

        RISCV_SERVICE.with(|cell| {
            let mut service = cell.borrow_mut();

            // binding-macro/src/service.rs => fn write_()
            let resp = service.exec(context, payload);
            if resp.is_error() {
                return resp;
            }

            let mut data_json = serde_json::to_string(&resp.succeed_data).expect("json encode");
            if data_json == "null" {
                data_json = "".to_owned();
            }
            ServiceResponse::<String>::from_succeed(data_json)
        })
    }
}

fn new_riscv_service() -> RiscvService<
    DefalutServiceSDK<
        GeneralServiceState<MemoryDB>,
        DefaultChainQuerier<MockStorage>,
        MockDispatcher,
    >,
> {
    let chain_db = DefaultChainQuerier::new(Arc::new(MockStorage {}));
    let trie = MPTTrie::new(Arc::new(MemoryDB::new(false)));
    let state = GeneralServiceState::new(trie);

    let sdk = DefalutServiceSDK::new(
        Rc::new(RefCell::new(state)),
        Rc::new(chain_db),
        MockDispatcher {},
    );

    RiscvService::init(sdk)
}

fn mock_context(cycles_limit: u64, caller: Address, tx_hash: Hash, nonce: Hash) -> ServiceContext {
    let params = ServiceContextParams {
        tx_hash: Some(tx_hash),
        nonce: Some(nonce),
        cycles_limit,
        cycles_price: 1,
        cycles_used: Rc::new(RefCell::new(0)),
        caller,
        height: 1,
        timestamp: 0,
        extra: None,
        service_name: "service_name".to_owned(),
        service_method: "service_method".to_owned(),
        service_payload: "service_payload".to_owned(),
        events: Rc::new(RefCell::new(vec![])),
    };

    ServiceContext::new(params)
}

struct MockStorage;

#[async_trait]
impl Storage for MockStorage {
    async fn insert_transactions(
        &self,
        _: Context,
        _: u64,
        _: Vec<SignedTransaction>,
    ) -> ProtocolResult<()> {
        unimplemented!()
    }

    async fn get_transactions(
        &self,
        _: Context,
        _: u64,
        _: Vec<Hash>,
    ) -> ProtocolResult<Vec<Option<SignedTransaction>>> {
        unimplemented!()
    }

    async fn get_transaction_by_hash(
        &self,
        _: Context,
        _: Hash,
    ) -> ProtocolResult<Option<SignedTransaction>> {
        unimplemented!()
    }

    async fn insert_block(&self, _: Context, _: Block) -> ProtocolResult<()> {
        unimplemented!()
    }

    async fn get_block(&self, _: Context, _: u64) -> ProtocolResult<Option<Block>> {
        unimplemented!()
    }

    async fn insert_receipts(&self, _: Context, _: u64, _: Vec<Receipt>) -> ProtocolResult<()> {
        unimplemented!()
    }

    async fn get_receipt_by_hash(&self, _: Context, _: Hash) -> ProtocolResult<Option<Receipt>> {
        unimplemented!()
    }

    async fn get_receipts(
        &self,
        _: Context,
        _: u64,
        _: Vec<Hash>,
    ) -> ProtocolResult<Vec<Option<Receipt>>> {
        unimplemented!()
    }

    async fn update_latest_proof(&self, _: Context, _: Proof) -> ProtocolResult<()> {
        unimplemented!()
    }

    async fn get_latest_proof(&self, _: Context) -> ProtocolResult<Proof> {
        unimplemented!()
    }

    async fn get_latest_block(&self, _: Context) -> ProtocolResult<Block> {
        unimplemented!()
    }

    async fn update_overlord_wal(&self, _: Context, _: Bytes) -> ProtocolResult<()> {
        unimplemented!()
    }

    async fn load_overlord_wal(&self, _: Context) -> ProtocolResult<Bytes> {
        unimplemented!()
    }
}
