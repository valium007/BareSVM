use crate::{dbg, hv::vcpu, structs::guest_regs, vmexit::ExitType};
use raw_cpuid::cpuid;

pub fn handle(vcpu_ctx: &mut vcpu, regs: &mut guest_regs) -> ExitType {

    let mut cpuid = cpuid!(regs.rax, regs.rcx);
    const CPUID_FN0000_0001_ECX_HYPERVISOR_PRESENT: u32 = 1 << 31;
    cpuid.ecx |= CPUID_FN0000_0001_ECX_HYPERVISOR_PRESENT;

    regs.rax = cpuid.eax as u64;
    regs.rbx = cpuid.ebx as u64;
    regs.rcx = cpuid.ecx as u64;
    regs.rdx = cpuid.edx as u64;

    ExitType::IncrementRIP
}
