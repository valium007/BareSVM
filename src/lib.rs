#![no_std]
#![no_main]
#![allow(dead_code, unused)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use wdk_alloc::WdkAllocator;
use wdk_sys::{NTSTATUS, PUNICODE_STRING, DRIVER_OBJECT, STATUS_SUCCESS};
use wdk::println;
use core::panic::PanicInfo;
extern crate wdk_panic;

mod segments;
mod utils;
mod vmcb;
mod hv;
mod vmexit;
mod vmmcall;
mod structs;

#[global_allocator]
static GLOBAL_ALLOCATOR: WdkAllocator = WdkAllocator;

 #[unsafe(export_name = "DriverEntry")]
 pub unsafe extern "system" fn driver_entry(
    driver: &mut DRIVER_OBJECT,
    registry_path: PUNICODE_STRING,
) -> NTSTATUS {
    println!("DriverEntry from Rust!");
    if utils::is_svm_supported() == true {
        hv::virtualize();
    }
    driver.DriverUnload = Some(driver_unload);
    STATUS_SUCCESS
}

unsafe extern "C" fn driver_unload(driver: *mut DRIVER_OBJECT) {
    hv::devirtualize();
    println!("bye bye from driver!");
}