use crate::{
    dbg, handler::{cpuid, vmmcall}, hv::*, inline::stgi, log::*, print, structs::*, utils::pa, vmcb::*, widestr
};
use core::{
    arch::{asm, x86_64::__cpuid},
    ptr::NonNull,
};
use raw_cpuid::cpuid;
use wdk::*;
use x86_64::registers::{
    model_specific,
    model_specific::{LStar, Msr},
};

pub enum VmExit {

}

pub enum ExitType {
    IncrementRIP,
    Devirtualize,
    Unknown
}

#[unsafe(no_mangle)]
extern "win64" fn vmexit_handler(
    mut vcpu: NonNull<vcpu>,
    mut guest_regs: NonNull<guest_regs>,
) -> u8 {
    let vcpu_ctx = unsafe { vcpu.as_mut() };
    let guest_regs = unsafe { guest_regs.as_mut() };

    guest_regs.rax = vcpu_ctx.guest_vmcb.state_save_area.rax;

    let exit = match vcpu_ctx.guest_vmcb.control_area.exit_code {

        VMEXIT_CPUID => cpuid::handle(vcpu_ctx, guest_regs),
        VMEXIT_VMRUN => vmrun_handler(vcpu_ctx),
        VMEXIT_VMMCALL => vmmcall::handle(vcpu_ctx, guest_regs),
        VMEXIT_NMI => {
            let rip = vcpu_ctx.guest_vmcb.state_save_area.rip;
            let rsp = vcpu_ctx.guest_vmcb.state_save_area.rsp;
            let nrip =vcpu_ctx.guest_vmcb.control_area.nrip;
            dbg!("in NMI handler rip={:0X} nrip={:0X} rsp={:0X}",rip,nrip,rsp);
            dbg_break();
            ExitType::IncrementRIP
        },
        _ => {
            ExitType::Unknown
        }
    };

    match exit {
        ExitType::Devirtualize =>  { return devirtualize_cpu(vcpu_ctx, guest_regs) },
        ExitType::IncrementRIP => {
            vcpu_ctx.guest_vmcb.state_save_area.rax = guest_regs.rax;
            vcpu_ctx.guest_vmcb.state_save_area.rip = vcpu_ctx.guest_vmcb.control_area.nrip;
        }
        ExitType::Unknown => {}
    }
    return 1;
}

fn vmrun_handler(vcpu_ctx: &mut vcpu) -> ExitType {
    vcpu_ctx.guest_vmcb.control_area.event_inj = 2147486477;
    ExitType::IncrementRIP
}
