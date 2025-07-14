use crate::segments::*;
use crate::structs::*;
use crate::utils::*;
use crate::vmcb::*;
use core::arch::asm;
use core::arch::global_asm;
use core::ffi::c_void;
use core::ptr::null;
use core::ptr::null_mut;
use core::ptr::{NonNull, addr_of};
use core::sync::atomic::{AtomicU64, Ordering};
use static_assertions::*;
use wdk::{dbg_break, println};
use wdk_sys::{
    CONTEXT,
    ntddk::{KeQueryActiveProcessorCount, RtlCaptureContext},
};
use x86::bits64::paging::{BASE_PAGE_SIZE, PAddr};
use x86::controlregs::*;
use x86::msr::{IA32_EFER, IA32_PAT, rdmsr, wrmsr};
use x86_64::instructions::tables::{sgdt, sidt};

#[unsafe(no_mangle)]
unsafe extern "win64" {
    unsafe fn launch_vm(guest_vmcb_pa: *mut u64);
}
global_asm!(include_str!("vmlaunch.asm"));

// use this to store which cpu is virtualized
static VIRTUALIZED_BITSET: AtomicU64 = AtomicU64::new(0);
static mut vcpu_pool: *mut vcpu = null_mut();

fn is_virtualized(idx: u32) -> bool {
    let bit = 1 << idx;
    VIRTUALIZED_BITSET.load(Ordering::Relaxed) & bit != 0
}

fn set_virtualized(idx: u32) {
    let bit = 1 << idx;
    VIRTUALIZED_BITSET.fetch_or(bit, Ordering::Relaxed);
}

#[repr(C, align(4096))]
pub struct host_stack_layout {
    pub stack_contents: [u8; STACK_CONTENTS_SIZE],
    pub trap_frame: KTRAP_FRAME,
    pub guest_vmcb_pa: u64,
    pub host_vmcb_pa: u64,
    pub self_data: *mut u64, // self reference that will point to a vcpu struct
    pub shared_data: *mut u64, // shared_data will be used for msr bitmap in future
    pub padding_1: u64,
    pub reserved_1: u64,
}
const_assert_eq!(core::mem::size_of::<host_stack_layout>(), KERNEL_STACK_SIZE);

#[repr(C, align(4096))]
pub struct vcpu {
    pub host_stack_layout: host_stack_layout,
    pub guest_vmcb: vmcb,
    pub host_vmcb: vmcb,
    pub host_state_area: [u8; BASE_PAGE_SIZE],
    pub prev_vmexit: u64,
    pub unload: bool,
}

const_assert_eq!(
    core::mem::size_of::<vcpu>(),
    KERNEL_STACK_SIZE + 4 * BASE_PAGE_SIZE
);

impl vcpu {
    pub fn setup_vcpu(&mut self, context: &mut CONTEXT) {
        let gdtr = sgdt();
        let idtr = sidt();

        self.unload = false;
        self.host_stack_layout.guest_vmcb_pa = pa(addr_of!(self.guest_vmcb) as _);
        self.host_stack_layout.host_vmcb_pa = pa(addr_of!(self.host_vmcb) as _);
        self.host_stack_layout.self_data = self as *mut vcpu as *mut u64;

        println!("guest_vmcb_pa: {}", self.host_stack_layout.guest_vmcb_pa);
        println!("host_area_pa: {}", self.host_stack_layout.host_vmcb_pa);

        //self.guest_vmcb.control_area.intercept_misc1 |= SVM_INTERCEPT_MISC1_CPUID;
        self.guest_vmcb.control_area.intercept_misc2 |= SVM_INTERCEPT_MISC2_VMRUN;
        self.guest_vmcb.control_area.intercept_misc2 |= SVM_INTERCEPT_MISC2_VMMCALL; //intercept vmmcall here

        self.guest_vmcb.control_area.guest_asid = 1;

        self.guest_vmcb.state_save_area.gdtr_base = gdtr.base.as_u64();
        self.guest_vmcb.state_save_area.gdtr_limit = gdtr.limit as _;
        self.guest_vmcb.state_save_area.idtr_base = idtr.base.as_u64();
        self.guest_vmcb.state_save_area.idtr_limit = idtr.limit as _;

        self.guest_vmcb.state_save_area.cs_limit = segment_limit(context.SegCs);
        self.guest_vmcb.state_save_area.ds_limit = segment_limit(context.SegDs);
        self.guest_vmcb.state_save_area.es_limit = segment_limit(context.SegEs);
        self.guest_vmcb.state_save_area.ss_limit = segment_limit(context.SegSs);

        self.guest_vmcb.state_save_area.cs_selector = context.SegCs;
        self.guest_vmcb.state_save_area.ds_selector = context.SegDs;
        self.guest_vmcb.state_save_area.es_selector = context.SegEs;
        self.guest_vmcb.state_save_area.ss_selector = context.SegSs;

        self.guest_vmcb.state_save_area.cs_attrib =
            segment_access_right(context.SegCs, gdtr.base.as_u64());
        self.guest_vmcb.state_save_area.ds_attrib =
            segment_access_right(context.SegDs, gdtr.base.as_u64());
        self.guest_vmcb.state_save_area.es_attrib =
            segment_access_right(context.SegEs, gdtr.base.as_u64());
        self.guest_vmcb.state_save_area.ss_attrib =
            segment_access_right(context.SegSs, gdtr.base.as_u64());

        unsafe {
            self.guest_vmcb.state_save_area.efer = rdmsr(IA32_EFER);
            self.guest_vmcb.state_save_area.gpat = rdmsr(IA32_PAT);
            self.guest_vmcb.state_save_area.cr0 = readcr0();
            self.guest_vmcb.state_save_area.cr2 = cr2() as _;
            self.guest_vmcb.state_save_area.cr3 = cr3() as _;
            self.guest_vmcb.state_save_area.cr4 = readcr4();
        }

        self.guest_vmcb.state_save_area.rflags = context.EFlags as u64;
        self.guest_vmcb.state_save_area.rsp = context.Rsp;
        self.guest_vmcb.state_save_area.rip = context.Rip;

        unsafe { asm!("vmsave rax", in("rax") self.host_stack_layout.guest_vmcb_pa) };

        let host_state_area_pa = pa(self.host_state_area.as_ptr() as *const _);
        unsafe { wrmsr(SVM_MSR_VM_HSAVE_PA, host_state_area_pa) };

        unsafe { asm!("vmsave rax", in("rax") self.host_stack_layout.host_vmcb_pa) };
    }
}

pub fn virtualize_cpu(idx: u32) {
    // capture context here, when the guest begins execution the guest_rip
    // will point here and the is_virtualized() will return true.

    let mut context = CONTEXT::default();
    unsafe {
        RtlCaptureContext(&mut context as *mut CONTEXT);
    }

    let mut vcpu_ptr = unsafe { vcpu_pool.add(idx as usize) };

    if vcpu_ptr.is_null() {
        println!("#cpu: {} data not found!", idx);
    }

    if !is_virtualized(idx) {
        unsafe {
            wrmsr(IA32_EFER, rdmsr(IA32_EFER) | EFER_SVME);
        }

        let mut vcpu = unsafe { &mut *vcpu_ptr };
        vcpu.setup_vcpu(&mut context);
        set_virtualized(idx);

        let host_rsp = &vcpu.host_stack_layout.guest_vmcb_pa as *const u64 as *mut u64;

        unsafe {
            launch_vm(host_rsp);
            println!("this should never print!");
            dbg_break();
        }
    }
    println!("virtualized #cpu: {}", idx)
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

pub fn virtualize() {
    if setup_resources() == true {
        run_on_all_cpus(virtualize_cpu)
    } else {
        println!("resources allocation failed!")
    }
}

pub fn devirtualize() {
    run_on_all_cpus(|idx| {
        println!("devirtualizing #cpu {}", idx);
        unsafe {
            asm!("mov rcx, 0x10");
            asm!("vmmcall");
        }
    });
    println!("done!");
    unsafe { deallocate(vcpu_pool as *mut c_void) };
}

fn setup_resources() -> bool {
    let cpu_count = unsafe { KeQueryActiveProcessorCount(null_mut()) };
    println!("#cpus: {}", cpu_count);

    unsafe {
        vcpu_pool = allocate(core::mem::size_of::<vcpu>() * cpu_count as usize) as *mut vcpu;
        if vcpu_pool.is_null() {
            println!("vcpu_pool is empty!");
            return false;
        }
    };
    println!("allocated vcpu_pool");
    true
}
