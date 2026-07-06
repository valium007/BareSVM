use core::arch::asm;
use x86_64::registers::model_specific::Msr;

#[inline]
pub fn vmsave(vmcb_pa: u64) {
    unsafe {
        asm!(
            "mov rax, {}",
            "vmsave rax",
            in(reg) vmcb_pa, options(nostack, preserves_flags),
        )
    };
}

#[inline]
pub fn vmload(vmcb_pa: u64) {
    unsafe {
        asm!(
            "mov rax, {}",
            "vmload rax",
            in(reg) vmcb_pa, options(nostack, preserves_flags),
        )
    };
}

#[inline]
pub fn readcr3() -> u64 {
    let ret: usize;
    unsafe { asm!("mov {0}, cr3", out(reg) ret) }
    ret as u64
}

#[inline]
pub fn rdmsr(msr: u32) -> u64 {
    unsafe {
        let val = Msr::new(msr).read();
        val
    }
}

#[inline]
pub fn wrmsr(msr: u32, value: u64) {
    unsafe {
        Msr::new(msr).write(value);
    }
}

#[inline]
pub fn stgi() {
    unsafe { asm!("stgi") }
}
