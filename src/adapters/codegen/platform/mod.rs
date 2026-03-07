/// Platform Syscall Abstraction: Architecture-Agnostic IO Port
///
/// This module defines the `PlatformSyscalls` trait — the boundary between
/// the codegen strategies (which express *what* IO to perform) and the
/// platform-specific inline assembly (which expresses *how*).
///
/// Adding a new architecture (e.g. AArch64) requires only a new implementation
/// of this trait; all codegen strategies remain untouched.

pub mod x86_64;

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::values::{IntValue, PointerValue};

/// Architecture-agnostic syscall generation port.
///
/// Each method emits LLVM IR (typically inline assembly) for a single
/// kernel-level IO operation.  The implementations are stateless — all
/// LLVM handles are passed in as parameters.
pub trait PlatformSyscalls {
    /// Emit a *write* syscall: send `len` bytes from `buf` to file descriptor `fd`.
    ///
    /// Returns the number of bytes successfully written (kernel return value).
    fn emit_write<'ctx>(
        &self,
        context: &'ctx Context,
        builder: &Builder<'ctx>,
        fd: IntValue<'ctx>,
        buf: PointerValue<'ctx>,
        len: IntValue<'ctx>,
    ) -> IntValue<'ctx>;

    /// Emit a *read* syscall: read up to `max_len` bytes from file descriptor
    /// `fd` into `buf`.
    ///
    /// Returns the number of bytes actually read (kernel return value).
    fn emit_read<'ctx>(
        &self,
        context: &'ctx Context,
        builder: &Builder<'ctx>,
        fd: IntValue<'ctx>,
        buf: PointerValue<'ctx>,
        max_len: IntValue<'ctx>,
    ) -> IntValue<'ctx>;
}

/// Factory: returns the syscall provider for the current compilation target.
///
/// Currently only x86_64 is supported.  Future architectures are added here.
pub fn create_syscalls() -> Box<dyn PlatformSyscalls> {
    Box::new(x86_64::X86_64Syscalls)
}
