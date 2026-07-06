use crate::{dbg, inline::*, msr::IA32_EFER, vmcb::EFER_SVME};
use core::{arch::asm, ffi::c_void};
use raw_cpuid::CpuId;
use wdk::*;
use wdk_sys::{ntddk::*, *};
use x86_64::registers::{control::Efer, model_specific::Msr};

unsafe extern "win64" {
    pub fn ZwYieldExecution() -> NTSTATUS;
}

// credits to https://github.com/not-matthias/amd_hypervisor
pub fn is_svm_supported() -> bool {
    // Check `CPUID Fn8000_0001_ECX[SVM] == 0`
    //
    let Some(result) = CpuId::new().get_extended_processor_and_feature_identifiers() else {
        return false;
    };
    if !result.has_svm() {
        println!("Processor does not support SVM");
        return false;
    }
    // Check `VM_CR.SVMDIS == 0`
    //
    // See in the AMD Manual '15.30.1  VM_CR MSR (C001_0114h)'
    //
    const SVM_MSR_VM_CR: u32 = 0xC001_0114;
    const SVM_VM_CR_SVMDIS: u64 = 1 << 4;

    let vm_cr = unsafe { rdmsr(SVM_MSR_VM_CR) };
    if (vm_cr & SVM_VM_CR_SVMDIS) == 0 {
        return true;
    }

    // Check `CPUID Fn8000_000A_EDX[SVML]==0`
    //
    if CpuId::new()
        .get_svm_info()
        .map(|svm_info| svm_info.has_svm_lock())
        .unwrap_or_default()
    {
        println!("the user must change a platform firmware setting to enable SVM");
    } else {
        println!("SVMLock may be unlockable; consult platform firmware or TPM to obtain the key.");
    }

    false
}

pub fn enable_svm() {
    wrmsr(IA32_EFER, rdmsr(IA32_EFER) | EFER_SVME);
    println!("enabled svm!");
}

pub fn pa(va: *const core::ffi::c_void) -> u64 {
    #[allow(clippy::cast_sign_loss)]
    unsafe {
        MmGetPhysicalAddress(va.cast_mut()).QuadPart as u64
    }
}

pub fn processor_count() -> u32 {
    unsafe { KeQueryActiveProcessorCount(core::ptr::null_mut()) }
}

pub struct ProcessorExecutor {
    old_affinity: KAFFINITY,
}

impl ProcessorExecutor {
    pub fn switch_to_processor(i: u32) -> Option<Self> {
        if i > processor_count() {
            dbg!("Invalid processor index: {}", i);
            return None;
        }
        let old_affinity = unsafe { KeSetSystemAffinityThreadEx(1u64 << i) };

        unsafe { ZwYieldExecution() };

        let mut cpu = PROCESSOR_NUMBER::default();
        unsafe { KeGetCurrentProcessorNumberEx(&mut cpu as *mut PROCESSOR_NUMBER) };
        dbg!("running at processor: {}", cpu.Number);

        Some(Self { old_affinity })
    }
}

impl Drop for ProcessorExecutor {
    fn drop(&mut self) {
        dbg!("Switching execution back to previous processor");
        unsafe {
            KeRevertToUserAffinityThreadEx(self.old_affinity);
        }
    }
}
