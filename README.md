# BareSVM

## AMD Hypervisor written in Rust

Written for learning purposes, very bare and minimal to act as a base to build upon

Sample program to do a hypercall from usermode

```rust
#[unsafe(naked)]
unsafe extern "win64" fn hypercall() -> u64 {
    core::arch::naked_asm!(
        "
        mov rcx, 1
        vmmcall
        ret
        "
    );
}

fn main() {
    println!("hypercall response: {}",unsafe {hypercall()});
}
```
When the hypervisor is running it will return 4919/0x1337 as response