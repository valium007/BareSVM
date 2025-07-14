use crate::hv::vcpu;
use crate::{structs::*, utils::*, vmcb::*, vmmcall};
use core::{arch::asm, ptr::addr_of};
use wdk::*;
use x86::msr::*;

const VMMCALL_UNLOAD: u64 = 0x10;
const VMMCALL_MAGIC: u64 = 1;

pub fn vmmcall_handler(vcpu_ctx: &mut vcpu, guest_regs: &mut guest_regs) {
    println!("in vmmcall handler");
    println!("vmmcall called with rcx: {}", guest_regs.rcx);

    match guest_regs.rcx {
        VMMCALL_MAGIC => {
            guest_regs.rax = 0x1337;
        }
        VMMCALL_UNLOAD => {
            vcpu_ctx.unload = true;
        }
        _ => {
            println!("invalid vmmcall_code");
            dbg_break();
        }
    }
}
