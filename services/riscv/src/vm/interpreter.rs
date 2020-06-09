use std::cell::RefCell;
use std::rc::Rc;

use ckb_vm::machine::asm::{AsmCoreMachine, AsmMachine};
use ckb_vm::{DefaultMachineBuilder, SupportMachine};

use protocol::{
    types::{Address, ServiceContext},
    Bytes,
};

use crate::types::{InterpreterResult, InterpreterType};
use crate::vm;
use crate::vm::ChainInterface;

// Duktape execution environment
#[cfg(debug_assertions)]
const DUKTAPE_EE: &[u8] = std::include_bytes!("c/duktape_ee.bin");

#[derive(Clone, Debug)]
pub enum MachineType {
    NativeRust,
    Asm,
}

#[derive(Clone, Debug)]
pub struct InterpreterConf {
    pub machine_type: MachineType,
}

impl Default for InterpreterConf {
    fn default() -> Self {
        InterpreterConf {
            machine_type: MachineType::Asm,
        }
    }
}

#[derive(Clone, Debug)]
pub struct InterpreterParams {
    pub address: Address,
    pub code:    Bytes,
    pub args:    Bytes,
    pub is_init: bool,
}

impl InterpreterParams {
    pub fn new(address: Address, code: Bytes, args: Bytes) -> InterpreterParams {
        Self {
            address,
            code,
            args,
            is_init: false,
        }
    }

    pub fn new_for_init(address: Address, code: Bytes, args: Bytes) -> InterpreterParams {
        let mut params = Self::new(address, code, args);
        params.is_init = true;
        params
    }
}

pub struct Interpreter {
    pub context: ServiceContext,
    pub cfg:     InterpreterConf,
    pub r#type:  InterpreterType,
    pub iparams: InterpreterParams,
    pub chain:   Rc<RefCell<dyn ChainInterface>>,
}

impl Interpreter {
    pub fn new(
        context: ServiceContext,
        cfg: InterpreterConf,
        r#type: InterpreterType,
        iparams: InterpreterParams,
        chain: Rc<RefCell<dyn ChainInterface>>,
    ) -> Self {
        Self {
            context,
            cfg,
            r#type,
            iparams,
            chain,
        }
    }

    pub fn run(&self) -> Result<InterpreterResult, ckb_vm::Error> {
        let (code, init_payload) = match self.r#type {
            InterpreterType::Binary => (self.iparams.code.clone(), None),
            #[cfg(debug_assertions)]
            InterpreterType::Duktape => (Bytes::from(DUKTAPE_EE), Some(self.iparams.code.clone())),
        };

        let mut args: Vec<Bytes> = vec!["main".into()];
        if let Some(payload) = init_payload {
            args.push(payload);
        }

        let ret_data = Rc::new(RefCell::new(Vec::new()));
        let cycles_lmit = self.context.get_cycles_limit();
        let (exitcode, cycles) = match self.cfg.machine_type {
            MachineType::NativeRust => {
                let core_machine =
                    ckb_vm::DefaultCoreMachine::<u64, ckb_vm::SparseMemory<u64>>::new_with_max_cycles(
                        cycles_lmit
                    );
                let mut machine = ckb_vm::DefaultMachineBuilder::<
                    ckb_vm::DefaultCoreMachine<u64, ckb_vm::SparseMemory<u64>>,
                >::new(core_machine)
                .instruction_cycle_func(Box::new(vm::cost_model::instruction_cycles))
                .syscall(Box::new(vm::SyscallDebug))
                .syscall(Box::new(vm::SyscallAssert))
                .syscall(Box::new(vm::SyscallEnvironment::new(
                    self.context.clone(),
                    self.iparams.clone(),
                )))
                .syscall(Box::new(vm::SyscallIO::new(
                    self.iparams.args.to_vec(),
                    Rc::<RefCell<_>>::clone(&ret_data),
                )))
                .syscall(Box::new(vm::SyscallChainInterface::new(
                    Rc::<RefCell<_>>::clone(&self.chain),
                )))
                .build();
                machine.load_program(&code, &args[..])?;
                let exitcode = machine.run()?;
                let cycles = machine.cycles();
                (exitcode, cycles)
            }
            MachineType::Asm => {
                let core_machine = AsmCoreMachine::new_with_max_cycles(cycles_lmit);
                let machine = DefaultMachineBuilder::<Box<AsmCoreMachine>>::new(core_machine)
                    .instruction_cycle_func(Box::new(vm::cost_model::instruction_cycles))
                    .syscall(Box::new(vm::SyscallDebug))
                    .syscall(Box::new(vm::SyscallAssert))
                    .syscall(Box::new(vm::SyscallEnvironment::new(
                        self.context.clone(),
                        self.iparams.clone(),
                    )))
                    .syscall(Box::new(vm::SyscallIO::new(
                        self.iparams.args.to_vec(),
                        Rc::<RefCell<_>>::clone(&ret_data),
                    )))
                    .syscall(Box::new(vm::SyscallChainInterface::new(
                        Rc::<RefCell<_>>::clone(&self.chain),
                    )))
                    .build();
                let mut machine = AsmMachine::new(machine, None);
                machine.load_program(&code, &args[..])?;
                let exitcode = machine.run()?;
                let cycles = machine.machine.cycles();
                (exitcode, cycles)
            }
        };
        let ret = ret_data.borrow();
        let result = InterpreterResult {
            ret_code:    exitcode,
            ret:         Bytes::from(ret.to_vec()),
            cycles_used: cycles,
        };
        Ok(result)
    }
}
