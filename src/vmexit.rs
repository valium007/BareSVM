use crate::hv::*;
use crate::vmcb::*;
use crate::structs::*;
use crate::vmmcall::*;
use core::arch::asm;
use core::ptr::NonNull;
use wdk::*;


#[unsafe(no_mangle)]
unsafe extern "win64" fn vmexit_handler(
    mut vcpu: NonNull<vcpu>,
    mut guest_regs: NonNull<guest_regs>,
) -> u8 {

    let vcpu_ctx = unsafe { vcpu.as_mut() };
    let guest_regs = unsafe { guest_regs.as_mut() };

    unsafe { asm!("vmload rax", in("rax") vcpu_ctx.host_stack_layout.host_vmcb_pa) };

    guest_regs.rax = vcpu_ctx.guest_vmcb.state_save_area.rax;

    vcpu_ctx.host_stack_layout.trap_frame.rsp = vcpu_ctx.guest_vmcb.state_save_area.rsp;
    vcpu_ctx.host_stack_layout.trap_frame.rip = vcpu_ctx.guest_vmcb.control_area.n_rip;

    match vcpu_ctx.guest_vmcb.control_area.exit_code {
        VMEXIT_VMRUN => vmrun_handler(vcpu_ctx),
        VMEXIT_VMMCALL => {
            vmmcall_handler(vcpu_ctx,guest_regs);
        }
        _ => {
            println!("if this prints, its over");
            dbg_break();
        }
    }

    if vcpu_ctx.unload {
        return devirtualize_cpu(vcpu_ctx, guest_regs);
    }
    // reflect changed regs to guest
    vcpu_ctx.guest_vmcb.state_save_area.rax = guest_regs.rax;
    vcpu_ctx.guest_vmcb.state_save_area.rip = vcpu_ctx.guest_vmcb.control_area.n_rip;

    return 0;
}

fn vmrun_handler(vcpu_ctx: &mut vcpu) {
    vcpu_ctx.guest_vmcb.control_area.event_inj = 2147486477;
}