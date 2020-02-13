//! Provedis a debug function, let the contract print information to standard
//! output.
use ckb_vm::instructions::Register;

use crate::vm::syscall::common::get_str;
use crate::vm::syscall::convention::SYSCODE_ASSERT;

pub struct SyscallAssert {
    prefix: &'static str,
}

impl SyscallAssert {
    pub fn new(prefix: &'static str) -> Self {
        Self { prefix }
    }
}

impl<Mac: ckb_vm::SupportMachine> ckb_vm::Syscalls<Mac> for SyscallAssert {
    fn initialize(&mut self, _machine: &mut Mac) -> Result<(), ckb_vm::Error> {
        Ok(())
    }

    fn ecall(&mut self, machine: &mut Mac) -> Result<bool, ckb_vm::Error> {
        let code = &machine.registers()[ckb_vm::registers::A7];
        if code.to_u64() != SYSCODE_ASSERT {
            return Ok(false);
        }

        let assertion = machine.registers()[ckb_vm::registers::A0].to_u64();
        if assertion == 0 {
            let msg_ptr = machine.registers()[ckb_vm::registers::A1].to_u64();
            if msg_ptr != 0 {
                let msg = get_str(machine, msg_ptr)?;
                log::debug!("{} [{}]", self.prefix, msg);
            }

            Err(ckb_vm::Error::Unexpected)
        } else {
            Ok(true)
        }
    }
}
