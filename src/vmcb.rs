use core::mem::offset_of;

use offsetter::*;
use static_assertions::const_assert_eq;

pub const SVM_INTERCEPT_MISC2_VMRUN: u32 = 1 << 0;
pub const SVM_INTERCEPT_MISC2_VMMCALL: u32 = 1 << 1;
pub const SVM_INTERCEPT_MISC1_CPUID: u32 = 1 << 18;
pub const SVM_INTERCEPT_MISC1_NMI: u32 = 1 << 1;
pub const SVM_MSR_VM_HSAVE_PA: u32 = 0xc001_0117;
pub const EFER_SVME: u64 = 1 << 12;
pub const VMEXIT_VMMCALL: u64 = 0x81;
pub const VMEXIT_CPUID: u64 = 0x72;
pub const VMEXIT_VMRUN: u64 = 0x80;
pub const VMEXIT_NMI: u64 = 0x61;


// ── Vmcb ──────────────────────────────────────────────────────────────────
/// The virtual machine control block (VMCB).
/// See: Appendix B Layout of VMCB
    #[repr(C, align(4096))]
    pub struct VmcbRaw {
        pub control_area: ControlArea,
        pub state_save_area: StateSaveArea,
    }


const _: () = assert!(offset_of!(VmcbRaw, control_area) == 0x0);
const _: () = assert!(offset_of!(VmcbRaw, state_save_area) == 0x400);
const _: () = assert!(core::mem::size_of::<VmcbRaw>() == 0x1000);

// ── ControlArea ──────────────────────────────────────────────────────────────
offset_debug!(
    /// The "metadata" area where we can specify what operations to intercept
    /// and can read details of #VMEXIT.
    /// See: Table B-1. VMCB Layout, Control Area
    pub struct ControlArea {
        0x000 pub intercept_cr_read:   u16,
        0x002 pub intercept_cr_write:  u16,
        0x004 pub intercept_dr_read:   u16,
        0x006 pub intercept_dr_write:  u16,
        0x008 pub intercept_exception: u32,
        0x00c pub intercept_misc1:     u32,
        0x010 pub intercept_misc2:     u32,
        0x014 pub intercept_misc3:     u32,
        0x03c pub pause_filter_threshold: u16,
        0x03e pub pause_filter_count:     u16,
        0x040 pub iopm_base_pa:            u64,
        0x048 pub msrpm_base_pa:           u64,
        0x050 pub tsc_offset:              u64,
        0x058 pub guest_asid:              u32,
        0x05c pub tlb_control:             u32,
        0x060 pub vintr:                   u64,
        0x068 pub interrupt_shadow:        u64,
        0x070 pub exit_code:               u64,
        0x078 pub exit_info1:              u64,
        0x080 pub exit_info2:              u64,
        0x088 pub exit_int_info:           u64,
        0x090 pub np_enable:               u64,
        0x098 pub avic_apic_bar:           u64,
        0x0a0 pub guest_pa_pf_ghcb:        u64,
        0x0a8 pub event_inj:               u64,
        0x0b0 pub ncr3:                    u64,
        0x0b8 pub lbr_virtualization_enable: u64,
        0x0c0 pub vmcb_clean:              u32,
        0x0c4 pub _reserved:               u32,
        0x0c8 pub nrip:                    u64,
        0x0d0 pub num_of_bytes_fetched:    u8,
        0x0d1 pub guest_instruction_bytes: [u8; 15],
        0x0e0 pub avic_apic_backing_page_pointer: u64,
        0x0f0 pub avic_logical_table_pointer:     u64,
        0x0f8 pub avic_physical_table_pointer:    u64,
        0x108 pub vmcb_save_state_pointer: u64,
        0x3e0 pub reserved_for_host:       [u8; 0x20],
    }
);
const _: () = assert!(core::mem::size_of::<ControlArea>() == 0x400);

// ── StateSaveArea ─────────────────────────────────────────────────────────────
offset_debug!(
    /// The area to specify and read guest register values.
    /// See: Table B-2. VMCB Layout, State Save Area
    pub struct StateSaveArea {
        0x000 pub es_selector:  u16,
        0x002 pub es_attrib:    u16,
        0x004 pub es_limit:     u32,
        0x008 pub es_base:      u64,
        0x010 pub cs_selector:  u16,
        0x012 pub cs_attrib:    u16,
        0x014 pub cs_limit:     u32,
        0x018 pub cs_base:      u64,
        0x020 pub ss_selector:  u16,
        0x022 pub ss_attrib:    u16,
        0x024 pub ss_limit:     u32,
        0x028 pub ss_base:      u64,
        0x030 pub ds_selector:  u16,
        0x032 pub ds_attrib:    u16,
        0x034 pub ds_limit:     u32,
        0x038 pub ds_base:      u64,
        0x040 pub fs_selector:  u16,
        0x042 pub fs_attrib:    u16,
        0x044 pub fs_limit:     u32,
        0x048 pub fs_base:      u64,
        0x050 pub gs_selector:  u16,
        0x052 pub gs_attrib:    u16,
        0x054 pub gs_limit:     u32,
        0x058 pub gs_base:      u64,
        0x060 pub gdtr_selector: u16,
        0x062 pub gdtr_attrib:   u16,
        0x064 pub gdtr_limit:    u32,
        0x068 pub gdtr_base:     u64,
        0x070 pub ldtr_selector: u16,
        0x072 pub ldtr_attrib:   u16,
        0x074 pub ldtr_limit:    u32,
        0x078 pub ldtr_base:     u64,
        0x080 pub idtr_selector: u16,
        0x082 pub idtr_attrib:   u16,
        0x084 pub idtr_limit:    u32,
        0x088 pub idtr_base:     u64,
        0x090 pub tr_selector:   u16,
        0x092 pub tr_attrib:     u16,
        0x094 pub tr_limit:      u32,
        0x098 pub tr_base:       u64,
        0x0cb pub cpl:           u8,
        0x0d0 pub efer:          u64,
        0x148 pub cr4:           u64,
        0x150 pub cr3:           u64,
        0x158 pub cr0:           u64,
        0x160 pub dr7:           u64,
        0x168 pub dr6:           u64,
        0x170 pub rflags:        u64,
        0x178 pub rip:           u64,
        0x1d8 pub rsp:           u64,
        0x1e0 pub s_cet:         u64,
        0x1e8 pub ssp:           u64,
        0x1f0 pub isst_addr:     u64,
        0x1f8 pub rax:           u64,
        0x200 pub star:          u64,
        0x208 pub lstar:         u64,
        0x210 pub cstar:         u64,
        0x218 pub sf_mask:       u64,
        0x220 pub kernel_gs_base: u64,
        0x228 pub sysenter_cs:   u64,
        0x230 pub sysenter_esp:  u64,
        0x238 pub sysenter_eip:  u64,
        0x240 pub cr2:           u64,
        0x268 pub gpat:          u64,
        0x270 pub dbg_ctl:       u64,
        0x278 pub br_from:       u64,
        0x280 pub br_to:         u64,
        0x288 pub last_excep_from: u64,
        0x290 pub last_excep_to:   u64,
        0x2e0 pub spec_ctl:      u64,

        0x770 pub lbr_select: u64,
        0x778 pub ibs_fetch_ctl: u64,
        0x780 pub ibs_fetch_linaddr: u64,
        0x788 pub ibs_op_ctl: u64,
        0x790 pub ibs_op_rip: u64,
        0x798 pub ibs_op_data: u64,
        0x7a0 pub ibs_op_data2: u64,
        0x7a8 pub ibs_op_data3: u64,
        0x7b0 pub ibs_dc_linaddr: u64,
        0x7b8 pub bp_ibstgt_rip: u64,
        0x7c0 pub ic_ibs_extd_ctl: u64,
    }
);
const _: () = assert!(core::mem::size_of::<StateSaveArea>() == 0x7c8);
