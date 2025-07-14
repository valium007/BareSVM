.global launch_vm

.equ KTRAP_FRAME_SIZE, 0x190
.equ GUEST_REGS_SIZE, 0x80

.macro pushaq
    push    rax
    push    rcx
    push    rdx
    push    rbx
    push    -1      // Dummy for rsp.
    push    rbp
    push    rsi
    push    rdi
    push    r8
    push    r9
    push    r10
    push    r11
    push    r12
    push    r13
    push    r14
    push    r15
.endmacro

.macro popaq
    pop     r15
    pop     r14
    pop     r13
    pop     r12
    pop     r11
    pop     r10
    pop     r9
    pop     r8
    pop     rdi
    pop     rsi
    pop     rbp
    pop     rbx    // Dummy for rsp (this value is destroyed by the next pop).
    pop     rbx
    pop     rdx
    pop     rcx
    pop     rax
.endmacro

launch_vm:
    // rsp = host_rsp
    //
    mov rsp, rcx

guest_loop:

    mov rax, [rsp]          // rax = vcpu.host_stack_layout.guest_vmcb_pa

    vmload rax              // load previous saved guest state from vmcb

    vmrun rax               // switch to guest until #VMEXIT

    vmsave rax              // save current guest state to vmcb

    sub rsp, KTRAP_FRAME_SIZE

    pushaq

    mov rdx, rsp                                                // rdx = guest_registers
    mov rcx, [rsp + GUEST_REGS_SIZE + KTRAP_FRAME_SIZE + 16]    // rcx = vcpu_ctx

    sub rsp, 0x20 + 0x60
    movaps [rsp + 0x20], xmm0
    movaps [rsp + 0x20 + 0x10], xmm1
    movaps [rsp + 0x20 + 0x20], xmm2
    movaps [rsp + 0x20 + 0x30], xmm3
    movaps [rsp + 0x20 + 0x40], xmm4
    movaps [rsp + 0x20 + 0x50], xmm5

    call vmexit_handler

    movaps [rsp + 0x20 + 0x50], xmm5
    movaps [rsp + 0x20 + 0x40], xmm4
    movaps [rsp + 0x20 + 0x30], xmm3
    movaps [rsp + 0x20 + 0x20], xmm2
    movaps [rsp + 0x20 + 0x10], xmm1
    movaps [rsp + 0x20], xmm0
    add rsp, 0x20 + 0x60

    test al, al

    popaq

    jnz exit_loop               
    add rsp, KTRAP_FRAME_SIZE   
    jmp guest_loop              

exit_loop:
    mov rsp, rcx        // rsp = host_rsp
    mov ecx, 0xCAFEBABE
    jmp rbx