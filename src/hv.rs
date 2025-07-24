extern crate alloc;
use crate::segments::*;
use crate::structs::*;
use crate::utils::*;
use crate::vmcb::*;
use alloc::boxed::Box;
use core::arch::{asm, global_asm};
use core::ptr::*;
use core::sync::atomic::{AtomicU64, Ordering};
use static_assertions::*;
use wdk::{dbg_break, println};
use wdk_sys::{
    ALL_PROCESSOR_GROUPS, PAGE_SIZE, CONTEXT, KAFFINITY, NT_SUCCESS, PAGED_CODE,
    POOL_FLAG_NON_PAGED, PROCESSOR_NUMBER, ntddk::*,
};
use x86::msr::{IA32_EFER, IA32_PAT};
use x86_64::instructions::tables::{sgdt, sidt};
use x86_64::registers::control::*;

#[unsafe(no_mangle)]
unsafe extern "win64" {
    unsafe fn launch_vm(guest_vmcb_pa: *mut u64);
}
global_asm!(include_str!("vmlaunch.asm"));

// use this to store which cpu is virtualized
static VIRTUALIZED_BITSET: AtomicU64 = AtomicU64::new(0);

fn is_virtualized(processor: u32) -> bool {
    let bit = 1 << processor;
    VIRTUALIZED_BITSET.load(Ordering::Relaxed) & bit != 0
}

fn set_virtualized(processor: u32) {
    let bit = 1 << processor;
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
    pub host_state_area: [u8; PAGE_SIZE as usize],
    pub prev_vmexit: u64,
    pub unload: bool,
}

impl vcpu {
    pub fn setup_vmcb(&mut self, context: &mut CONTEXT) {
        let gdtr = sgdt();
        let idtr = sidt();

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
            self.guest_vmcb.state_save_area.cr0 = Cr0::read_raw();
            self.guest_vmcb.state_save_area.cr2 = Cr2::read_raw();
            self.guest_vmcb.state_save_area.cr3 = readcr3();
            self.guest_vmcb.state_save_area.cr4 = Cr4::read_raw();
        }

        self.guest_vmcb.state_save_area.rflags = context.EFlags as u64;
        self.guest_vmcb.state_save_area.rsp = context.Rsp;
        self.guest_vmcb.state_save_area.rip = context.Rip;

        unsafe { asm!("vmsave rax", in("rax") self.host_stack_layout.guest_vmcb_pa) };

        let host_state_area_pa = pa(self.host_state_area.as_ptr() as *const _);
        unsafe { wrmsr(SVM_MSR_VM_HSAVE_PA, host_state_area_pa) };

        unsafe { asm!("vmsave rax", in("rax") self.host_stack_layout.host_vmcb_pa) };
    }

    pub fn new(context: &mut CONTEXT) -> Box<Self> {
        let instance = Self {
            host_stack_layout: host_stack_layout {
                stack_contents: [0u8; STACK_CONTENTS_SIZE],
                trap_frame: unsafe { core::mem::zeroed() },
                guest_vmcb_pa: 0,
                host_vmcb_pa: 0,
                self_data: core::ptr::null_mut(),
                shared_data: core::ptr::null_mut(),
                padding_1: u64::MAX,
                reserved_1: u64::MAX,
            },
            guest_vmcb: unsafe { core::mem::zeroed() },
            host_vmcb: unsafe { core::mem::zeroed() },
            host_state_area: [0u8; PAGE_SIZE as usize],
            prev_vmexit: 0,
            unload: false,
        };
        let mut instance = Box::new(instance);
        instance.setup_vmcb(context);
        return instance;
    }
}

fn virtualize_cpu(processor: u32) {
    let mut context = CONTEXT::default();
    unsafe { RtlCaptureContext(&mut context as *mut CONTEXT) };

    if !is_virtualized(processor) {
        set_virtualized(processor);
        enable_svm();
        let mut vcpu = vcpu::new(&mut context);
        let host_rsp = &vcpu.host_stack_layout.guest_vmcb_pa as *const u64 as *mut u64;
        unsafe { launch_vm(host_rsp) };
    }
    println!("virtualized #cpu: {}", processor)
}

pub fn virtualize() {
    for processor in 0..processor_count() {
        let Some(executor) = ProcessorExecutor::switch_to_processor(processor) else {
            return println!("failed to switch to #cpu: {}", processor);
        };

        virtualize_cpu(processor);
        core::mem::drop(executor);
    }
}

pub fn devirtualize_cpu(vcpu_ctx: &mut vcpu, guest_regs: &mut guest_regs) -> u8 {
    guest_regs.rax = vcpu_ctx as *mut _ as u32 as u64; // storing addr of vcpu_ctx
    guest_regs.rdx = vcpu_ctx as *mut _ as u64 >> 32; // in these two registers

    guest_regs.rbx = vcpu_ctx.guest_vmcb.control_area.n_rip;
    guest_regs.rcx = vcpu_ctx.guest_vmcb.state_save_area.rsp;

    let guest_vmcb_pa = pa(addr_of!(vcpu_ctx.guest_vmcb) as _);

    unsafe {
        asm!("vmload rax", in("rax") guest_vmcb_pa);
        asm!("sti");
        asm!("stgi");

        // Disable svm.
        let msr = rdmsr(IA32_EFER) & !EFER_SVME;
        wrmsr(IA32_EFER, msr);

        // Restore guest eflags.
        asm!("push {}; popfq", in(reg) (*vcpu_ctx).guest_vmcb.state_save_area.rflags);
    }
    return 1;
}

pub fn devirtualize() {
    for processor in 0..processor_count() {
        let Some(executor) = ProcessorExecutor::switch_to_processor(processor) else {
            return println!("failed to switch to #cpu: {}", processor);
        };

        unsafe {
            core::arch::asm!("vmmcall", in("rcx") 0x10, options(nostack, nomem));
        }
        println!("devirtualized #cpu: {}", processor);
        core::mem::drop(executor);
    }
}