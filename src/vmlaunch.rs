#[unsafe(naked)]
pub extern "win64" fn vmlaunch(host_rsp: *mut u64) {
    core::arch::naked_asm!(
        "

.equ GUEST_REGS_SIZE, 0x180

launch_vm:
    // rsp = host_rsp
    mov rsp, rcx

guest_loop:

    mov rax, [rsp]          // rax = vcpu.host_stack_layout.guest_vmcb_pa

    vmload rax              // load previous saved guest state from vmcb

    vmrun rax               
							// vmexit happened
							// after vmexit the rax and rsp registers are restored to the host one, but the other gprs are the same
							// the set of registers stored/restored by the vmsave/vmload are also of the guest after vmexit
							
    vmsave rax              // so we save them again to the guest vmcb
							
							// saving guest gprs
    push rax
	push rbx
	push rcx
	push rdx
	push rsi
	push rdi
	push rbp
	push r8
	push r9
	push r10
	push r11
	push r12
	push r13
	push r14
	push r15

	sub rsp, 0x108 // 16 xmm registers... and +8 bytes for alignment...
	movaps [rsp], xmm0
	movaps [rsp + 0x10], xmm1
	movaps [rsp + 0x20], xmm2
	movaps [rsp + 0x30], xmm3
	movaps [rsp + 0x40], xmm4
	movaps [rsp + 0x50], xmm5
	movaps [rsp + 0x60], xmm6
	movaps [rsp + 0x70], xmm7
	movaps [rsp + 0x80], xmm8
	movaps [rsp + 0x90], xmm9
	movaps [rsp + 0xA0], xmm10
	movaps [rsp + 0xB0], xmm11
	movaps [rsp + 0xC0], xmm12
	movaps [rsp + 0xD0], xmm13
	movaps [rsp + 0xE0], xmm14
	movaps [rsp + 0xF0], xmm15

	mov rdx, rsp
    mov rcx, [rsp + GUEST_REGS_SIZE + 8]

	sub rsp, 20
	call vmexit_handler
	add rsp, 20

	movups xmm0, [rsp]
	movups xmm1, [rsp + 0x10]
	movups xmm2, [rsp + 0x20]
	movups xmm3, [rsp + 0x30]
	movups xmm4, [rsp + 0x40]
	movups xmm5, [rsp + 0x50]
	movups xmm6, [rsp + 0x60]
	movups xmm7, [rsp + 0x70]
	movups xmm8, [rsp + 0x80]
	movups xmm9, [rsp + 0x90]
	movups xmm10, [rsp + 0xA0]
	movups xmm11, [rsp + 0xB0]
	movups xmm12, [rsp + 0xC0]
	movups xmm13, [rsp + 0xD0]
	movups xmm14, [rsp + 0xE0]
	movups xmm15, [rsp + 0xF0]
	add rsp, 0x108 // 16 xmm registers... and +8 bytes for alignment...

    test al, al

	pop r15
	pop r14
	pop r13
	pop r12
	pop r11
	pop r10
	pop r9
	pop r8
	pop rbp 
	pop rdi
	pop rsi
	pop rdx
	pop rcx
	pop rbx
	pop rax

    jnz guest_loop
	add rsp, 24
	iretq
	"
    );
}
