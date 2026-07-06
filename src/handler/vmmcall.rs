use crate::{hv::vcpu, structs::*, utils::*, vmcb::*, vmexit::ExitType};
use core::{arch::asm, ptr::addr_of};
use wdk::*;

const VMMCALL_UNLOAD: u64 = 0x10;
const VMMCALL_MAGIC: u64 = 1;

pub fn handle(vcpu_ctx: &mut vcpu, regs: &mut guest_regs) -> ExitType {
    match regs.rcx {
        VMMCALL_MAGIC => {
            regs.rax = 0x1337;
            ExitType::IncrementRIP
        }
        VMMCALL_UNLOAD => ExitType::Devirtualize,
        _ => ExitType::Unknown,
    }
}
