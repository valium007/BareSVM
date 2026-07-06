use crate::{dbg, hv};
use core::ptr::null;
use wdk_sys::{ntddk::*, *};

#[macro_export]
macro_rules! widestr {
    ($s:literal) => {{
        const W: &[u16] = &{
            let bytes = $s.as_bytes();
            let mut out = [0u16; $s.len() + 1];
            let mut i = 0usize;
            while i < bytes.len() {
                out[i] = bytes[i] as u16;
                i += 1;
            }
            out
        };
        W.as_ptr()
    }};
}

pub const SERIAL_COM_1: u16 = 0x3F8;

pub const SERIAL_WIN_COM_PORT_1: *const u16 = widestr!("\\Device\\Serial0");

const SERIAL_TRANSMIT_HOLDING_REGISTER: u16 = 0x00;
const SERIAL_LINE_STATUS_REGISTER: u16 = 0x05;

pub fn InitializeObjectAttributes(
    p: POBJECT_ATTRIBUTES,
    n: PUNICODE_STRING,
    a: u32,
    r: *mut core::ffi::c_void,
    s: *mut core::ffi::c_void,
) {
    use core::mem::size_of;
    unsafe {
        (*p).Length = size_of::<OBJECT_ATTRIBUTES>() as u32;
        (*p).RootDirectory = r;
        (*p).Attributes = a;
        (*p).ObjectName = n;
        (*p).SecurityDescriptor = s as _;
        (*p).SecurityQualityOfService = core::ptr::null_mut();
    }
}

pub fn serial_initialize(port_name: *const u16) -> NTSTATUS {
    unsafe {
        let mut port: UNICODE_STRING = core::mem::zeroed();

        RtlInitUnicodeString(&mut port, port_name);

        let mut obj: OBJECT_ATTRIBUTES = core::mem::zeroed();
        InitializeObjectAttributes(
            &mut obj,
            &mut port,
            OBJ_KERNEL_HANDLE,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        );

        let mut handle: HANDLE = core::mem::zeroed();
        let mut iosb: IO_STATUS_BLOCK = core::mem::zeroed();

        unsafe {
            ZwCreateFile(
                &mut handle,
                GENERIC_READ | GENERIC_WRITE,
                &mut obj,
                &mut iosb,
                core::ptr::null_mut(),
                FILE_ATTRIBUTE_NORMAL,
                0,
                FILE_OPEN,
                FILE_NON_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT,
                core::ptr::null_mut(),
                0,
            )
        }
    }
}

unsafe fn inbyte(port: u16) -> u8 {
    unsafe {
        core::arch::asm!("in al, dx", in("dx") port, out("al") _, options(nomem, nostack, preserves_flags));
        let mut value: u8;
        core::arch::asm!("in al, dx", in("dx") port, out("al") value, options(nomem, nostack, preserves_flags));
        value
    }
}

unsafe fn outbyte(port: u16, value: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
    }
}

pub unsafe fn serial_write_byte(port_base: u16, data: u8) {
    unsafe {
        while (inbyte(port_base + SERIAL_LINE_STATUS_REGISTER) & 0x20) == 0 {}
        outbyte(port_base + SERIAL_TRANSMIT_HOLDING_REGISTER, data);
    }
}

pub unsafe fn puts(s: &str) {
    unsafe {
        for b in s.bytes() {
            serial_write_byte(SERIAL_COM_1, b);
        }
    }
}

use spin::Mutex;
struct Writer;

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        unsafe { puts(s) };
        Ok(())
    }
}

static WRITER: Mutex<Writer> = Mutex::new(Writer);
pub fn print_fmt(args: core::fmt::Arguments) {
    let mut writer = WRITER.lock();
    <Writer as core::fmt::Write>::write_fmt(&mut writer, args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($t:tt)*) => { $crate::log::print_fmt(format_args!($($t)*)) };
}

#[macro_export]
macro_rules! dbg {
    () => { $crate::print!("\n") };
    ($($t:tt)*) => { $crate::log::print_fmt(format_args!("{}\n", format_args!($($t)*))) };
}
