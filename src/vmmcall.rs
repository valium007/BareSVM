use crate::hv::vcpu;
use crate::{utils::*,structs::*, vmmcall, vmcb::*};
use wdk::*;
use core::{ptr::addr_of,arch::asm};
use x86::msr::*;

const VMMCALL_UNLOAD: u64 = 0x10;
const VMMCALL_MAGIC: u64 = 1;

pub fn vmmcall_handler(vcpu_ctx: &mut vcpu, guest_regs: &mut guest_regs) {
    
    println!("in vmmcall handler");
    println!("vmmcall called with rcx: {}",guest_regs.rcx);
    
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

pub fn devirtualize_cpu(vcpu_ctx: &mut vcpu, guest_regs: &mut guest_regs) -> u8 {

    guest_regs.rax = vcpu_ctx as *mut _ as u32 as u64;
    guest_regs.rdx = vcpu_ctx as *mut _ as u64 >> 32;

    guest_regs.rbx = vcpu_ctx.guest_vmcb.control_area.n_rip;
    guest_regs.rcx = vcpu_ctx.guest_vmcb.state_save_area.rsp;

    let guest_vmcb_pa = pa(addr_of!(vcpu_ctx.guest_vmcb) as _);

    unsafe {
    asm!("vmload rax", in("rax") guest_vmcb_pa);

    asm!("cli");
    asm!("stgi");

    // Disable svm.
    let msr = rdmsr(IA32_EFER) & !EFER_SVME;
    wrmsr(IA32_EFER, msr);

    // Restore guest eflags.
    asm!("push {}; popfq", in(reg) (*vcpu_ctx).guest_vmcb.state_save_area.rflags);
    }
    return 1;

}
