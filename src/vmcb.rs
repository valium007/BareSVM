use static_assertions::const_assert_eq;

pub const SVM_INTERCEPT_MISC2_VMRUN: u32 = 1 << 0;
pub const SVM_INTERCEPT_MISC2_VMMCALL: u32 = 1 << 1;
pub const SVM_INTERCEPT_MISC1_CPUID: u32 = 1 << 18;
pub const SVM_MSR_VM_HSAVE_PA: u32 = 0xc001_0117;
pub const EFER_SVME: u64 = 1 << 12;
pub const VMEXIT_VMMCALL: u64 = 0x81;
pub const VMEXIT_CPUID: u64 = 0x0072;
pub const VMEXIT_VMRUN: u64 = 0x0080;

#[repr(C)]
pub struct control_area {
    pub intercept_cr_read: u16,              // +0x000
    pub intercept_cr_write: u16,             // +0x002
    pub intercept_dr_read: u16,              // +0x004
    pub intercept_dr_write: u16,             // +0x006
    pub intercept_exception: u32,            // +0x008
    pub intercept_misc1: u32,                // +0x00c
    pub intercept_misc2: u32,                // +0x010
    pub reserved1: [u8; 0x03c - 0x014],      // +0x014
    pub pause_filter_threshold: u16,         // +0x03c
    pub pause_filter_count: u16,             // +0x03e
    pub iopm_base_pa: u64,                   // +0x040
    pub msrpm_base_pa: u64,                  // +0x048
    pub tsc_offset: u64,                     // +0x050
    pub guest_asid: u32,                     // +0x058
    pub tlb_control: u32,                    // +0x05c
    pub vintr: u64,                          // +0x060
    pub interrupt_shadow: u64,               // +0x068
    pub exit_code: u64,                      // +0x070
    pub exit_info1: u64,                     // +0x078
    pub exit_info2: u64,                     // +0x080
    pub exit_int_info: u64,                  // +0x088
    pub np_enable: u64,                      // +0x090
    pub avic_apic_bar: u64,                  // +0x098
    pub guest_pa_of_ghcb: u64,               // +0x0a0
    pub event_inj: u64,                      // +0x0a8
    pub n_cr3: u64,                          // +0x0b0
    pub lbr_virtualization_enable: u64,      // +0x0b8
    pub vmcb_clean: u64,                     // +0x0c0
    pub n_rip: u64,                          // +0x0c8
    pub num_of_bytes_fetched: u8,            // +0x0d0
    pub guest_instruction_bytes: [u8; 15],   // +0x0d1
    pub avic_apic_backing_page_pointer: u64, // +0x0e0
    pub reserved2: u64,                      // +0x0e8
    pub avic_logical_table_pointer: u64,     // +0x0f0
    pub avic_physical_table_pointer: u64,    // +0x0f8
    pub reserved3: u64,                      // +0x100
    pub vmcb_save_state_pointer: u64,        // +0x108
    pub reserved4: [u8; 0x400 - 0x110],      // +0x110
}
const_assert_eq!(core::mem::size_of::<control_area>(), 0x400);

#[repr(C)]
pub struct state_save {
    pub es_selector: u16,
    pub es_attrib: u16,
    pub es_limit: u32,
    pub es_base: u64,
    pub cs_selector: u16,
    pub cs_attrib: u16,
    pub cs_limit: u32,
    pub cs_base: u64,
    pub ss_selector: u16,
    pub ss_attrib: u16,
    pub ss_limit: u32,
    pub ss_base: u64,
    pub ds_selector: u16,
    pub ds_attrib: u16,
    pub ds_limit: u32,
    pub ds_base: u64,
    pub fs_selector: u16,
    pub fs_attrib: u16,
    pub fs_limit: u32,
    pub fs_base: u64,
    pub gs_selector: u16,
    pub gs_attrib: u16,
    pub gs_limit: u32,
    pub gs_base: u64,
    pub gdtr_selector: u16,
    pub gdtr_attrib: u16,
    pub gdtr_limit: u32,
    pub gdtr_base: u64,
    pub ldtr_selector: u16,
    pub ldtr_attrib: u16,
    pub ldtr_limit: u32,
    pub ldtr_base: u64,
    pub idtr_selector: u16,
    pub idtr_attrib: u16,
    pub idtr_limit: u32,
    pub idtr_base: u64,
    pub tr_selector: u16,
    pub tr_attrib: u16,
    pub tr_limit: u32,
    pub tr_base: u64,
    pub reserved1: [u8; 43],
    pub cpl: u8,
    pub reserved2: u32,
    pub efer: u64,
    pub reserved3: [u8; 112],
    pub cr4: u64,
    pub cr3: u64,
    pub cr0: u64,
    pub dr7: u64,
    pub dr6: u64,
    pub rflags: u64,
    pub rip: u64,
    pub reserved4: [u8; 88],
    pub rsp: u64,
    pub reserved5: [u8; 24],
    pub rax: u64,
    pub star: u64,
    pub lstar: u64,
    pub cstar: u64,
    pub sf_mask: u64,
    pub kernel_gs_base: u64,
    pub sysenter_cs: u64,
    pub sysenter_esp: u64,
    pub sysenter_eip: u64,
    pub cr2: u64,
    pub reserved6: [u8; 32usize],
    pub gpat: u64,
    pub dbg_ctl: u64,
    pub br_from: u64,
    pub br_to: u64,
    pub last_excep_from: u64,
    pub last_excep_to: u64,
}
const_assert_eq!(core::mem::size_of::<state_save>(), 0x298);

#[repr(C, align(4096))]
pub struct vmcb {
    pub control_area: control_area,
    pub state_save_area: state_save,
}
const_assert_eq!(core::mem::size_of::<vmcb>(), 0x1000);
