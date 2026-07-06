#![no_std]
#![no_main]
#![allow(dead_code, unused)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use crate::log::serial_initialize;
use core::panic::PanicInfo;
use wdk::{nt_success, println};
use wdk_alloc::WdkAllocator;
use wdk_sys::{DRIVER_OBJECT, NTSTATUS, PUNICODE_STRING, STATUS_SUCCESS};

extern crate wdk_panic;

mod handler;
mod hv;
mod inline;
mod log;
mod msr;
mod segments;
mod string;
mod structs;
mod utils;
mod vmcb;
mod vmexit;
mod vmlaunch;

#[global_allocator]
static GLOBAL_ALLOCATOR: WdkAllocator = WdkAllocator;

#[unsafe(export_name = "DriverEntry")]
pub unsafe extern "system" fn driver_entry(
    driver: &mut DRIVER_OBJECT,
    registry_path: PUNICODE_STRING,
) -> NTSTATUS {
    unsafe {
        println!("DriverEntry from Rust!");

        if serial_initialize(log::SERIAL_WIN_COM_PORT_1) != STATUS_SUCCESS {
            println!("failed to initialize serial logger");
        } else {
            hv::logger = true;
            println!("initialized serial logger")
        }

        if utils::is_svm_supported() == true {
            hv::virtualize();
        }
        driver.DriverUnload = Some(driver_unload);
        STATUS_SUCCESS
    }
}

unsafe extern "C" fn driver_unload(driver: *mut DRIVER_OBJECT) {
    hv::devirtualize();
    println!("bye bye from driver!");
}
