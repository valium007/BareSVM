use core::ffi::c_void;
use x86::{cpuid::CpuId, msr::rdmsr};
use wdk::*;
use wdk_sys::ntddk::*;
use wdk_sys::{PAGE_SIZE,POOL_FLAG_NON_PAGED};
use core::arch::asm;
use core::sync::atomic::{AtomicU64, Ordering};


pub fn is_svm_supported() -> bool {
    // Check `CPUID Fn8000_0001_ECX[SVM] == 0`
    //
    let Some(result) = CpuId::new().get_extended_processor_and_feature_identifiers() else { return false };
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
        println!(
            "the user must change a platform firmware setting to enable SVM"
        );
    } else {
        println!(
            "SVMLock may be unlockable; consult platform firmware or TPM to obtain the key."
        );
    }

    false
}

pub fn allocate(size: usize) -> *mut c_void {
    if(size <= PAGE_SIZE as usize){
        println!("alloc called with size <= PAGE_SIZE");
    }
    let addr = unsafe { ExAllocatePool2(POOL_FLAG_NON_PAGED,size as u64,0x64657246) }; // 0x64657246 = derF
    return addr;
}

pub fn deallocate(p: *mut c_void) {
    unsafe { ExFreePool(p) };
}


pub fn pa(va: *const core::ffi::c_void) -> u64 {
        #[allow(clippy::cast_sign_loss)]
        unsafe {
            MmGetPhysicalAddress(va.cast_mut()).QuadPart as u64
        }
}

pub fn readcr0() -> u64 {
    let ret: usize;
    unsafe { asm!("mov %cr0, {0}", out(reg) ret, options(att_syntax)) }
    ret as u64
}

pub fn readcr4() -> u64 {
    let ret: usize;
    unsafe { asm!("mov %cr4, {0}", out(reg) ret, options(att_syntax)) };
    ret as u64
}

use wdk_sys::{
    ntddk::{
        KeGetCurrentIrql, KeGetProcessorNumberFromIndex, KeQueryActiveProcessorCountEx,
        KeRevertToUserGroupAffinityThread, KeSetSystemGroupAffinityThread, MmGetPhysicalAddress,
    },
    ALL_PROCESSOR_GROUPS, APC_LEVEL, GROUP_AFFINITY, NT_SUCCESS, PAGED_CODE, PROCESSOR_NUMBER,
};

pub fn run_on_all_cpus(callback: fn(u32)) {
    fn processor_count() -> u32 {
        unsafe { KeQueryActiveProcessorCountEx(u16::try_from(ALL_PROCESSOR_GROUPS).unwrap()) }
    }

    PAGED_CODE!();

    for index in 0..processor_count() {
        let mut processor_number = PROCESSOR_NUMBER::default();
        let status = unsafe { KeGetProcessorNumberFromIndex(index, &mut processor_number) };
        assert!(NT_SUCCESS(status));

        let mut old_affinity = GROUP_AFFINITY::default();
        let mut affinity = GROUP_AFFINITY {
            Group: processor_number.Group,
            Mask: 1 << processor_number.Number,
            Reserved: [0, 0, 0],
        };
        unsafe { KeSetSystemGroupAffinityThread(&mut affinity, &mut old_affinity) };

        callback(index);

        unsafe { KeRevertToUserGroupAffinityThread(&mut old_affinity) };
    }
}