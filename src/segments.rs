// credits to https://github.com/not-matthias/amd_hypervisor
use bitfield::bitfield;
use core::arch::asm;

bitfield! {
    pub struct SegmentDescriptor(u64);
    impl Debug;
    pub get_limit_low, set_limit_low: 15, 0;            // [0-15]
    pub get_base_low, set_base_low: 31, 16;             // [16-31]
    pub get_base_middle, set_base_middle: 39, 32;       // [32-39]
    pub get_type, set_type: 43, 40;                     // [40-43]
    pub get_system, set_system: 44, 44;                 // [44]
    pub get_dpl, set_dpl: 46, 45;                       // [45-46]
    pub get_present, set_present: 47, 47;               // [47]
    pub get_limit_high, set_limit_high: 51, 48;         // [48-51]
    pub get_avl, set_avl: 52, 52;                       // [52]
    pub get_long_mode, set_long_mode: 53, 53;           // [53]
    pub get_default_bit, set_default_bit: 54, 54;       // [54]
    pub get_granularity, set_granularity: 55, 55;       // [55]
    pub get_base_high, set_base_high: 63, 56;           // [56-63]
}

bitfield! {
    pub struct SegmentAttribute(u16);
    impl Debug;
    pub get_type, set_type: 3, 0;                       // [0-4]
    pub get_system, set_system: 4, 4;                   // [4]
    pub get_dpl, set_dpl: 6, 5;                         // [5-6]
    pub get_present, set_present: 7, 7;                 // [7]
    pub get_avl, set_avl: 8, 8;                         // [8]
    pub get_long_mode, set_long_mode: 9, 9;             // [9]
    pub get_default_bit, set_default_bit: 10, 10;       // [10]
    pub get_grunularity, set_granularity: 11, 11;       // [11]
    // reserved                                     // [12-15]
}

pub fn segment_access_right(segment_selector: u16, gdt_base: u64) -> u16 {
    const RPL_MASK: u16 = 3;
    let descriptor = gdt_base + (segment_selector & !RPL_MASK) as u64;

    let descriptor = descriptor as *mut u64 as *mut SegmentDescriptor;
    let descriptor = unsafe { descriptor.read_volatile() };

    let mut attribute = SegmentAttribute(0);
    attribute.set_type(descriptor.get_type() as u16);
    attribute.set_system(descriptor.get_system() as u16);
    attribute.set_dpl(descriptor.get_dpl() as u16);
    attribute.set_present(descriptor.get_present() as u16);
    attribute.set_avl(descriptor.get_avl() as u16);
    attribute.set_long_mode(descriptor.get_long_mode() as u16);
    attribute.set_default_bit(descriptor.get_default_bit() as u16);
    attribute.set_granularity(descriptor.get_granularity() as u16);

    attribute.0
}

pub fn segment_limit(selector: u16) -> u32 {
    let limit: u32;
    unsafe {
        asm!("lsl {0:e}, {1:x}", out(reg) limit, in(reg) selector, options(nostack, nomem));
    }
    limit
}
