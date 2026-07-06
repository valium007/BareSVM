extern crate alloc;
use crate::{
    dbg, inline::*, msr::*, segments::*, structs::*, utils::*, vmcb::*, vmlaunch::vmlaunch,
};
use alloc::boxed::Box;
use core::{
    arch::{asm, global_asm},
    ptr::*,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};
use static_assertions::*;
use wdk_sys::{
    ALL_PROCESSOR_GROUPS, CONTEXT, KAFFINITY, NT_SUCCESS, PAGE_SIZE, PAGED_CODE,
    POOL_FLAG_NON_PAGED, PROCESSOR_NUMBER, ULONG_PTR, ntddk::*,
};
use x86_64::{
    instructions::{
        interrupts::enable,
        tables::{sgdt, sidt},
    },
    registers::{
        control::*,
        model_specific::{Efer, Msr},
    },
};

pub static mut logger: bool = false;

static VIRTUALIZED_BITSET: AtomicU64 = AtomicU64::new(0);
fn is_virtualized(processor: u32) -> bool {
    let bit = 1 << processor;
    VIRTUALIZED_BITSET.load(Ordering::Relaxed) & bit != 0
}

fn set_virtualized(processor: u32) {
    let bit = 1 << processor;
    VIRTUALIZED_BITSET.fetch_or(bit, Ordering::Relaxed);
}

#[repr(C)]
pub struct iret_frame {
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

#[repr(C, align(4096))]
pub struct host_stack_layout {
    pub stack_contents: [u8; STACK_CONTENTS_SIZE],
    pub guest_vmcb_pa: u64,
    pub self_data: *mut u64, // self reference that will point to a vcpu struct
    pub shared_data: *mut u64, // shared_data will be used for msr bitmap in future
    pub guest_frame: iret_frame,
}

#[repr(C, align(4096))]
pub struct vcpu {
    pub host_stack_layout: host_stack_layout,
    pub host_state_area: [u8; PAGE_SIZE as usize],
    pub guest_vmcb: VmcbRaw,
    pub prev_vmexit: u64,
}

impl vcpu {
    #[inline(never)]
    pub fn setup_vmcb(&mut self, context: &mut CONTEXT) {
        let gdtr = sgdt();
        let idtr = sidt();

        self.host_stack_layout.guest_vmcb_pa = pa(addr_of!(self.guest_vmcb) as _);
        self.host_stack_layout.self_data = self as *mut vcpu as *mut u64;

        dbg!("guest_vmcb_pa: {:x}", self.host_stack_layout.guest_vmcb_pa);

        self.guest_vmcb.control_area.intercept_misc1 |= SVM_INTERCEPT_MISC1_CPUID;
        self.guest_vmcb.control_area.intercept_misc1 |= SVM_INTERCEPT_MISC1_NMI;
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

        vmsave(self.host_stack_layout.guest_vmcb_pa);

        let host_state_area_pa = pa(self.host_state_area.as_ptr() as *const _);
        wrmsr(SVM_MSR_VM_HSAVE_PA, host_state_area_pa)
    }

    #[inline(never)]
    pub fn new(context: &mut CONTEXT) -> Box<Self> {
        let instance = Self {
            host_stack_layout: host_stack_layout {
                stack_contents: [0u8; STACK_CONTENTS_SIZE],
                guest_vmcb_pa: 0,
                self_data: core::ptr::null_mut(),
                shared_data: core::ptr::null_mut(),
                guest_frame: iret_frame {
                    rip: 0,
                    cs: 0,
                    rflags: 0,
                    rsp: 0,
                    ss: 0,
                },
            },
            guest_vmcb: unsafe { core::mem::zeroed() },
            host_state_area: [0u8; PAGE_SIZE as usize],
            prev_vmexit: 0,
        };
        let mut instance = Box::new(instance);
        instance.setup_vmcb(context);
        return instance;
    }
}

#[inline(never)]
fn virtualize_cpu(processor: u32) {
    let mut context = CONTEXT::default();
    unsafe { RtlCaptureContext(&mut context as *mut CONTEXT) };

    if !is_virtualized(processor) {
        set_virtualized(processor);
        enable_svm();
        let mut vcpu = vcpu::new(&mut context);
        let host_rsp = &vcpu.host_stack_layout.guest_vmcb_pa as *const u64 as *mut u64;
        vmlaunch(host_rsp);
    }
    dbg!("virtualized #cpu: {}", processor)
}

#[inline(never)]
pub fn virtualize() {
    for processor in 0..processor_count() {
        let Some(executor) = ProcessorExecutor::switch_to_processor(processor) else {
            return dbg!("failed to switch to #cpu: {}", processor);
        };

        virtualize_cpu(processor);
        core::mem::drop(executor);
    }
}

pub fn devirtualize_cpu(vcpu_ctx: &mut vcpu, guest_regs: &mut guest_regs) -> u8 {
    let guest_vmcb_pa = pa(addr_of!(vcpu_ctx.guest_vmcb) as _);

    vmload(guest_vmcb_pa);
    enable();
    stgi();

    // disable svm
    wrmsr(IA32_EFER, rdmsr(IA32_EFER) & !EFER_SVME);

    guest_regs.rax = vcpu_ctx.guest_vmcb.state_save_area.rax;
    vcpu_ctx.host_stack_layout.guest_frame.rip = vcpu_ctx.guest_vmcb.control_area.nrip;
    vcpu_ctx.host_stack_layout.guest_frame.cs =
        vcpu_ctx.guest_vmcb.state_save_area.cs_selector.into();
    vcpu_ctx.host_stack_layout.guest_frame.rflags = vcpu_ctx.guest_vmcb.state_save_area.rflags;
    vcpu_ctx.host_stack_layout.guest_frame.rsp = vcpu_ctx.guest_vmcb.state_save_area.rsp;
    vcpu_ctx.host_stack_layout.guest_frame.ss =
        vcpu_ctx.guest_vmcb.state_save_area.ss_selector.into();

    return 0;
}

#[inline(never)]
pub fn devirtualize() {
    for processor in 0..processor_count() {
        let Some(executor) = ProcessorExecutor::switch_to_processor(processor) else {
            return dbg!("failed to switch to #cpu: {}", processor);
        };

        unsafe {
            core::arch::asm!("vmmcall", in("rcx") 0x10, options(nostack, nomem));
        }

        core::mem::drop(executor);
    }
}
