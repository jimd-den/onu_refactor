/// x86_64 Syscall Implementation
///
/// Implements `PlatformSyscalls` using the Linux x86_64 syscall ABI:
///   - `%rax` = syscall number  (0 = read, 1 = write)
///   - `%rdi` = arg 1           (file descriptor)
///   - `%rsi` = arg 2           (buffer pointer)
///   - `%rdx` = arg 3           (byte count)
///   - Clobbers: `%rcx`, `%r11`, flags
///
/// No C runtime or libc dependency — pure inline assembly via LLVM.

use super::PlatformSyscalls;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::values::{CallableValue, IntValue, PointerValue};
use std::convert::TryFrom;

pub struct X86_64Syscalls;

impl X86_64Syscalls {
    /// Build the shared syscall inline-asm function type and constraint string.
    fn build_syscall_asm<'ctx>(
        context: &'ctx Context,
        builder: &Builder<'ctx>,
        syscall_nr: u64,
        fd: IntValue<'ctx>,
        buf: PointerValue<'ctx>,
        count: IntValue<'ctx>,
        call_name: &str,
    ) -> IntValue<'ctx> {
        let i64_type = context.i64_type();
        let i8_ptr_type = context.i8_type().ptr_type(inkwell::AddressSpace::default());

        let syscall_type = i64_type.fn_type(
            &[
                i64_type.into(),    // rax — syscall number
                i64_type.into(),    // rdi — file descriptor
                i8_ptr_type.into(), // rsi — buffer pointer
                i64_type.into(),    // rdx — byte count
            ],
            false,
        );

        let asm_fn = context.create_inline_asm(
            syscall_type,
            "syscall".to_string(),
            "={ax},{ax},{di},{si},{dx},~{rcx},~{r11},~{dirflag},~{fpsr},~{flags}".to_string(),
            true,  // has side effects
            false, // align stack
            None,
            false,
        );

        let call_result = builder
            .build_call(
                CallableValue::try_from(asm_fn).unwrap(),
                &[
                    i64_type.const_int(syscall_nr, false).into(),
                    fd.into(),
                    buf.into(),
                    count.into(),
                ],
                call_name,
            )
            .unwrap();

        match call_result.try_as_basic_value() {
            inkwell::values::ValueKind::Basic(v) => v.into_int_value(),
            _ => {
                eprintln!("[WARNING] Platform syscall returned unexpected non-basic value");
                i64_type.const_int(0, false)
            }
        }
    }
}

impl PlatformSyscalls for X86_64Syscalls {
    fn emit_write<'ctx>(
        &self,
        context: &'ctx Context,
        builder: &Builder<'ctx>,
        fd: IntValue<'ctx>,
        buf: PointerValue<'ctx>,
        len: IntValue<'ctx>,
    ) -> IntValue<'ctx> {
        // sys_write = 1
        Self::build_syscall_asm(context, builder, 1, fd, buf, len, "syscall_write")
    }

    fn emit_read<'ctx>(
        &self,
        context: &'ctx Context,
        builder: &Builder<'ctx>,
        fd: IntValue<'ctx>,
        buf: PointerValue<'ctx>,
        max_len: IntValue<'ctx>,
    ) -> IntValue<'ctx> {
        // sys_read = 0
        Self::build_syscall_asm(context, builder, 0, fd, buf, max_len, "syscall_read")
    }
}
