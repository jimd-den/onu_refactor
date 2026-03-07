/// Ọ̀nụ Codegen Compatibility Layer — Typed vs Opaque Pointers
///
/// LLVM 14/15 uses *typed pointers*: every `i8*` carries its element type.
/// LLVM 16+  uses *opaque pointers*: all pointers share a single `ptr` type.
///
/// This module wraps the six call-patterns that differ between the two models,
/// letting the rest of the codegen stay readable with a single API surface.
///
/// # Feature gating
/// Build with `--features llvm14` (default) or `--features llvm15` to select
/// the typed-pointer path.  `--features llvm16` (or 17/18/19/20) selects the
/// opaque-pointer path.  The `typed-pointers` feature in `Cargo.toml` is a
/// local relay that is set to `true` for llvm14 and llvm15 and `false`
/// for llvm16+.

use inkwell::{
    AddressSpace,
    builder::Builder,
    context::Context,
    types::{BasicType, BasicTypeEnum, PointerType},
    values::{BasicValueEnum, IntValue, PointerValue},
};

// ---------------------------------------------------------------------------
// onu_i8ptr — the canonical "raw byte pointer" type
// ---------------------------------------------------------------------------

/// Return the canonical raw-byte-pointer type used throughout the Onu codegen.
///
/// | LLVM version | Result         |
/// |-------------|----------------|
/// | 14 / 15     | `i8*`          |
/// | 16+         | `ptr` (opaque) |
#[cfg(feature = "typed-pointers")]
#[inline(always)]
pub fn onu_i8ptr<'ctx>(context: &'ctx Context) -> PointerType<'ctx> {
    context.i8_type().ptr_type(AddressSpace::default())
}

#[cfg(not(feature = "typed-pointers"))]
#[inline(always)]
pub fn onu_i8ptr<'ctx>(context: &'ctx Context) -> PointerType<'ctx> {
    context.ptr_type(AddressSpace::default())
}

// ---------------------------------------------------------------------------
// onu_ptr_to — typed pointer-to-T (e.g. i64*)
// ---------------------------------------------------------------------------

/// Return a pointer-to-`elem` type.
///
/// | LLVM version | Result              |
/// |-------------|---------------------|
/// | 14 / 15     | `elem*` (typed ptr) |
/// | 16+         | `ptr`   (opaque)    |
#[cfg(feature = "typed-pointers")]
#[inline(always)]
pub fn onu_ptr_to<'ctx>(elem: impl BasicType<'ctx>) -> PointerType<'ctx> {
    elem.ptr_type(AddressSpace::default())
}

#[cfg(not(feature = "typed-pointers"))]
#[inline(always)]
pub fn onu_ptr_to<'ctx>(context: &'ctx Context, _elem: impl BasicType<'ctx>) -> PointerType<'ctx> {
    context.ptr_type(AddressSpace::default())
}

// ---------------------------------------------------------------------------
// build_byte_gep — GEP into a byte array (arena / string buffer)
// ---------------------------------------------------------------------------

/// Emit an in-bounds GEP treating `ptr` as an `i8` array.
///
/// | LLVM version | Inkwell call                                  |
/// |-------------|-----------------------------------------------|
/// | 14 / 15     | `build_in_bounds_gep(ptr, &[idx], name)`      |
/// | 16+         | `build_in_bounds_gep(i8_type, ptr, &[idx], name)` |
///
/// # Safety
/// The caller must ensure `idx` stays within the allocated object.
#[cfg(feature = "typed-pointers")]
#[inline(always)]
pub unsafe fn build_byte_gep<'ctx>(
    _context: &'ctx Context,
    builder: &Builder<'ctx>,
    ptr: PointerValue<'ctx>,
    idx: IntValue<'ctx>,
    name: &str,
) -> PointerValue<'ctx> {
    builder.build_in_bounds_gep(ptr, &[idx], name).unwrap()
}

#[cfg(not(feature = "typed-pointers"))]
#[inline(always)]
pub unsafe fn build_byte_gep<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    ptr: PointerValue<'ctx>,
    idx: IntValue<'ctx>,
    name: &str,
) -> PointerValue<'ctx> {
    builder.build_in_bounds_gep(context.i8_type(), ptr, &[idx], name).unwrap()
}

// ---------------------------------------------------------------------------
// build_typed_load — load a value of a known type from a pointer
// ---------------------------------------------------------------------------

/// Emit a load instruction returning a value of type `pointee_ty`.
///
/// | LLVM version | Inkwell call                                   |
/// |-------------|------------------------------------------------|
/// | 14 / 15     | `build_load(ptr, name)` (type from ptr)        |
/// | 16+         | `build_load(pointee_ty, ptr, name)` (explicit) |
#[cfg(feature = "typed-pointers")]
#[inline(always)]
pub fn build_typed_load<'ctx>(
    _context: &'ctx Context,
    builder: &Builder<'ctx>,
    _pointee_ty: impl BasicType<'ctx>,
    ptr: PointerValue<'ctx>,
    name: &str,
) -> BasicValueEnum<'ctx> {
    builder.build_load(ptr, name).unwrap()
}

#[cfg(not(feature = "typed-pointers"))]
#[inline(always)]
pub fn build_typed_load<'ctx>(
    _context: &'ctx Context,
    builder: &Builder<'ctx>,
    pointee_ty: impl BasicType<'ctx>,
    ptr: PointerValue<'ctx>,
    name: &str,
) -> BasicValueEnum<'ctx> {
    builder.build_load(pointee_ty, ptr, name).unwrap()
}

// ---------------------------------------------------------------------------
// cast_to_typed_ptr — reinterpret a byte pointer as a typed pointer
// ---------------------------------------------------------------------------

/// Reinterpret `ptr` (byte ptr) as a pointer to `dest_type`.
///
/// | LLVM version | Effect                                                      |
/// |-------------|-------------------------------------------------------------|
/// | 14 / 15     | `build_pointer_cast(ptr, dest_type.ptr_type(..), name)`     |
/// | 16+         | No-op — all pointers are already the same opaque `ptr` type |
#[cfg(feature = "typed-pointers")]
#[inline(always)]
pub fn cast_to_typed_ptr<'ctx>(
    _context: &'ctx Context,
    builder: &Builder<'ctx>,
    ptr: PointerValue<'ctx>,
    dest_type: impl BasicType<'ctx>,
    name: &str,
) -> PointerValue<'ctx> {
    builder
        .build_pointer_cast(ptr, dest_type.ptr_type(AddressSpace::default()), name)
        .unwrap()
}

#[cfg(not(feature = "typed-pointers"))]
#[inline(always)]
pub fn cast_to_typed_ptr<'ctx>(
    _context: &'ctx Context,
    _builder: &Builder<'ctx>,
    ptr: PointerValue<'ctx>,
    _dest_type: impl BasicType<'ctx>,
    _name: &str,
) -> PointerValue<'ctx> {
    ptr // All pointers are the same opaque type — no cast needed.
}

// ---------------------------------------------------------------------------
// store_target_type — infer the integer type to truncate to before a Store
// ---------------------------------------------------------------------------

/// Return the integer type that a store through `ptr` should truncate to.
///
/// For opaque pointers the element type is unknown from the pointer alone;
/// we return `None` and let the caller decide (typically no truncation needed
/// when the value type already matches the store width).
#[cfg(feature = "typed-pointers")]
#[inline(always)]
pub fn store_int_element_type<'ctx>(
    ptr: PointerValue<'ctx>,
) -> Option<inkwell::types::IntType<'ctx>> {
    let elem = ptr.get_type().get_element_type();
    if elem.is_int_type() {
        Some(elem.into_int_type())
    } else {
        None
    }
}

#[cfg(not(feature = "typed-pointers"))]
#[inline(always)]
pub fn store_int_element_type<'ctx>(
    _ptr: PointerValue<'ctx>,
) -> Option<inkwell::types::IntType<'ctx>> {
    // Opaque pointer: element type is not encoded in the pointer type.
    // The caller (StoreStrategy) will fall through to the else-branch and
    // store without truncation.
    None
}

// ---------------------------------------------------------------------------
// arena_ptr_initializer — initial value for the global onu_arena_ptr
// ---------------------------------------------------------------------------

/// Build the constant initializer for the `onu_arena_ptr` global.
///
/// | LLVM version | Technique                                                     |
/// |-------------|---------------------------------------------------------------|
/// | 14 / 15     | `arena.as_pointer_value().const_cast(i8ptr_type)` — typed cast |
/// | 16+         | `arena.as_pointer_value()` — opaque ptr is already compatible |
#[cfg(feature = "typed-pointers")]
#[inline(always)]
pub fn arena_ptr_initializer<'ctx>(
    context: &'ctx Context,
    arena_ptr_val: PointerValue<'ctx>,
) -> PointerValue<'ctx> {
    let i8ptr = onu_i8ptr(context);
    arena_ptr_val.const_cast(i8ptr)
}

#[cfg(not(feature = "typed-pointers"))]
#[inline(always)]
pub fn arena_ptr_initializer<'ctx>(
    _context: &'ctx Context,
    arena_ptr_val: PointerValue<'ctx>,
) -> PointerValue<'ctx> {
    arena_ptr_val // Already the right opaque type.
}
