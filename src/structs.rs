use static_assertions::*;
use wdk::*;
use wdk_sys::{ntddk::*, *};

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct guest_regs {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub rbx: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rax: u64,
}
const_assert_eq!(core::mem::size_of::<guest_regs>(), 0x80 /* 16 * 0x8 */);

// credits to https://github.com/not-matthias/amd_hypervisor
#[repr(C)]
pub struct KTRAP_FRAME {
    /*
     * Home address for the parameter registers.
     */
    pub p1_home: u64,
    pub p2_home: u64,
    pub p3_home: u64,
    pub p4_home: u64,
    pub p5: u64,
    /*
     * Previous processor mode (system services only) and previous IRQL
     * (interrupts only).
     */
    pub previous_mode: KPROCESSOR_MODE,
    pub previous_irql: KIRQL,
    /*
     * Page fault load/store indicator.
     */
    pub fault_indicator: u8,
    /*
     * Exception active indicator.
     *
     *    0 - interrupt frame.
     *    1 - exception frame.
     *    2 - service frame.
     */
    pub exception_active: u8,
    /*
     * Floating point state.
     */
    pub mx_csr: u32,
    /*
     *  Volatile registers.
     *
     * N.B. These registers are only saved on exceptions and interrupts. They
     *      are not saved for system calls.
     */
    pub rax: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    /*
     * Gsbase is only used if the previous mode was kernel.
     *
     * GsSwap is only used if the previous mode was user.
     *
     * Note: This was originally an union (GsSwap).
     */
    pub gs_base: u64,
    /*
     * Volatile floating registers.
     *
     * N.B. These registers are only saved on exceptions and interrupts. They
     *      are not saved for system calls.
     */
    pub xmm0: u128,
    pub xmm1: u128,
    pub xmm2: u128,
    pub xmm3: u128,
    pub xmm4: u128,
    pub xmm5: u128,
    /*
     * First parameter, page fault address, context record address if user APC
     * bypass.
     *
     * Note: This was originally an union (ContextRecord).
     */
    pub fault_address: u64,
    /*
     *  Debug registers.
     */
    pub dr0: u64,
    pub dr1: u64,
    pub dr2: u64,
    pub dr3: u64,
    pub dr6: u64,
    pub dr7: u64,
    /*
     * Special debug registers.
     *
     * Note: This was originally in its own structure.
     */
    pub debug_control: u64,
    pub last_branch_to_rip: u64,
    pub last_branch_from_rip: u64,
    pub last_exception_to_rip: u64,
    pub last_exception_from_rip: u64,
    /*
     *  Segment registers
     */
    pub seg_ds: u16,
    pub seg_es: u16,
    pub seg_fs: u16,
    pub seg_gs: u16,
    /*
     * Previous trap frame address.
     */
    pub trap_frame: u64,
    /*
     * Saved nonvolatile registers RBX, RDI and RSI. These registers are only
     * saved in system service trap frames.
     */
    pub rbx: u64,
    pub rdi: u64,
    pub rsi: u64,
    /*
     * Saved nonvolatile register RBP. This register is used as a frame
     * pointer during trap processing and is saved in all trap frames.
     */
    pub rbp: u64,
    /*
     * Information pushed by hardware.
     *
     * N.B. The error code is not always pushed by hardware. For those cases
     *      where it is not pushed by hardware a dummy error code is allocated
     *      on the stack.
     *
     * Note: This was originally an union (ExceptionFrame).
     */
    pub error_code: u64,
    pub rip: u64,
    pub seg_cs: u16,
    pub fill_0: u8,
    pub logging: u8,
    pub fill_1: [u16; 2],
    pub e_flags: u32,
    pub fill_2: u32,
    pub rsp: u64,
    pub seg_ss: u16,
    pub fill_3: u16,
    pub fill_4: u32,
}

pub const KERNEL_STACK_SIZE: usize = 0x6000;
pub const STACK_CONTENTS_SIZE: usize = KERNEL_STACK_SIZE
    - (core::mem::size_of::<*mut u64>() * 6)
    - core::mem::size_of::<KTRAP_FRAME>();
