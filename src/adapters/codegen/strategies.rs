use crate::application::use_cases::registry_service::RegistryService;
use crate::domain::entities::error::OnuError;
/// Codegen Strategies: Interface Adapter Layer
///
/// This module implements the Strategy Pattern for MIR Instruction generation.
/// Each strategy is responsible for translating a specific MIR instruction
/// into the corresponding LLVM IR.
use crate::domain::entities::mir::{MirBinOp, MirInstruction, MirLiteral, MirOperand};
use crate::domain::entities::types::OnuType;

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::llvm_sys;
use inkwell::module::Module;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{AsValueRef, BasicValueEnum, CallableValue, PointerValue};
use std::collections::HashMap;
use std::convert::TryFrom;

pub trait InstructionStrategy<'ctx> {
    fn generate(
        &self,
        context: &'ctx Context,
        module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        registry: &RegistryService,
        ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
        inst: &MirInstruction,
    ) -> Result<(), OnuError>;
}

pub struct PromoteStrategy;
impl<'ctx> InstructionStrategy<'ctx> for PromoteStrategy {
    fn generate(
        &self,
        context: &'ctx Context,
        _module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        registry: &RegistryService,
        ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
        inst: &MirInstruction,
    ) -> Result<(), OnuError> {
        if let MirInstruction::Promote { dest, src, to_type } = inst {
            let src_val = operand_to_llvm(context, builder, ssa_storage, src);
            let target_llvm_type = crate::adapters::codegen::typemapper::LlvmTypeMapper::onu_to_llvm(context, to_type, registry)
                .unwrap().into_int_type();

            let ptr = get_or_create_ssa(context, builder, ssa_storage, *dest, target_llvm_type.into());

            let promoted = builder
                .build_int_z_extend(src_val.into_int_value(), target_llvm_type, "promote_zext")
                .unwrap();

            builder.build_store(ptr, promoted).unwrap();
        }
        Ok(())
    }
}

pub struct BinaryOpStrategy;
impl<'ctx> InstructionStrategy<'ctx> for BinaryOpStrategy {
    fn generate(
        &self,
        context: &'ctx Context,
        module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        registry: &RegistryService,
        ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
        inst: &MirInstruction,
    ) -> Result<(), OnuError> {
        if let MirInstruction::BinaryOperation { dest, op, lhs, rhs, dest_type } = inst {
            // Width-Aware Strategy: guard against unsupported LLVM backend widths.
            // LLVM's compiler-rt only provides division helpers up to i128.  Any
            // division on WideInt(bits > 128) must have been replaced by the
            // WideDivLegalizationPass before reaching the codegen layer.  If we
            // somehow encounter one here, it is a compiler bug — surface a clear error
            // instead of silently producing a segfault in the LLVM backend.
            if matches!(op, MirBinOp::Div) {
                if let OnuType::WideInt(bits) = dest_type {
                    if *bits > 128 {
                        return Err(OnuError::CodeGenError { message: format!(
                            "Codegen reached an unsupported WideInt({}) division instruction. \
                             This instruction should have been legalized by WideDivLegalizationPass \
                             before reaching the LLVM backend.",
                            bits
                        )});
                    }
                }
            }

            let mut l_val = operand_to_llvm(context, builder, ssa_storage, lhs);
            let mut r_val = operand_to_llvm(context, builder, ssa_storage, rhs);

            // Check if widths match for integers
            if l_val.is_int_value() && r_val.is_int_value() {
                let l_width = l_val.into_int_value().get_type().get_bit_width();
                let r_width = r_val.into_int_value().get_type().get_bit_width();
                if l_width < r_width {
                    l_val = builder.build_int_z_extend(l_val.into_int_value(), r_val.into_int_value().get_type(), "implicit_zext_l").unwrap().into();
                } else if r_width < l_width {
                    r_val = builder.build_int_z_extend(r_val.into_int_value(), l_val.into_int_value().get_type(), "implicit_zext_r").unwrap().into();
                }
            }

            let res: BasicValueEnum = match op {
                MirBinOp::Add => builder
                    .build_int_nsw_add(l_val.into_int_value(), r_val.into_int_value(), "addtmp")
                    .unwrap()
                    .into(),
                MirBinOp::Sub => builder
                    .build_int_nsw_sub(l_val.into_int_value(), r_val.into_int_value(), "subtmp")
                    .unwrap()
                    .into(),
                MirBinOp::Mul => builder
                    .build_int_nsw_mul(l_val.into_int_value(), r_val.into_int_value(), "multmp")
                    .unwrap()
                    .into(),
                MirBinOp::Div => builder
                    .build_int_signed_div(l_val.into_int_value(), r_val.into_int_value(), "divtmp")
                    .unwrap()
                    .into(),
                MirBinOp::And => builder
                    .build_and(l_val.into_int_value(), r_val.into_int_value(), "andtmp")
                    .unwrap()
                    .into(),
                MirBinOp::Eq | MirBinOp::Ne | MirBinOp::Gt | MirBinOp::Lt => {
                    let pred = match op {
                        MirBinOp::Eq => inkwell::IntPredicate::EQ,
                        MirBinOp::Ne => inkwell::IntPredicate::NE,
                        MirBinOp::Gt => inkwell::IntPredicate::SGT,
                        MirBinOp::Lt => inkwell::IntPredicate::SLT,
                        _ => unreachable!(),
                    };

                    if l_val.is_struct_value() && r_val.is_struct_value() {
                        // STRING EQUALITY
                        let l_struct = l_val.into_struct_value();
                        let r_struct = r_val.into_struct_value();

                        let l_len = builder
                            .build_extract_value(l_struct, 0, "l_len")
                            .unwrap()
                            .into_int_value();
                        let r_len = builder
                            .build_extract_value(r_struct, 0, "r_len")
                            .unwrap()
                            .into_int_value();

                        let l_ptr = builder
                            .build_extract_value(l_struct, 1, "l_ptr")
                            .unwrap()
                            .into_pointer_value();
                        let r_ptr = builder
                            .build_extract_value(r_struct, 1, "r_ptr")
                            .unwrap()
                            .into_pointer_value();

                        // 1. Compare lengths
                        let len_eq = builder
                            .build_int_compare(inkwell::IntPredicate::EQ, l_len, r_len, "len_eq")
                            .unwrap();

                        let current_bb = builder.get_insert_block().unwrap();
                        let parent_fn = current_bb.get_parent().unwrap();

                        let cmp_bb = context.append_basic_block(parent_fn, "str_cmp_start");
                        let loop_bb = context.append_basic_block(parent_fn, "str_cmp_loop");
                        let match_bb = context.append_basic_block(parent_fn, "str_match");
                        let fail_bb = context.append_basic_block(parent_fn, "str_fail");
                        let merge_bb = context.append_basic_block(parent_fn, "str_cmp_merge");

                        builder
                            .build_conditional_branch(len_eq, cmp_bb, fail_bb)
                            .unwrap();

                        // cmp_bb: if len is 0, they match
                        builder.position_at_end(cmp_bb);
                        let is_empty = builder
                            .build_int_compare(
                                inkwell::IntPredicate::EQ,
                                l_len,
                                context.i64_type().const_zero(),
                                "is_empty",
                            )
                            .unwrap();
                        builder
                            .build_conditional_branch(is_empty, match_bb, loop_bb)
                            .unwrap();

                        // loop_bb: compare chars
                        builder.position_at_end(loop_bb);
                        let index_phi = builder.build_phi(context.i64_type(), "index").unwrap();
                        index_phi.add_incoming(&[(&context.i64_type().const_zero(), cmp_bb)]);

                        let idx = index_phi.as_basic_value().into_int_value();
                        let l_char_ptr = unsafe {
                            builder
                                .build_in_bounds_gep(l_ptr, &[idx], "l_char_ptr")
                                .unwrap()
                        };
                        let r_char_ptr = unsafe {
                            builder
                                .build_in_bounds_gep(r_ptr, &[idx], "r_char_ptr")
                                .unwrap()
                        };

                        let l_char = builder
                            .build_load(l_char_ptr, "l_char")
                            .unwrap()
                            .into_int_value();
                        let r_char = builder
                            .build_load(r_char_ptr, "r_char")
                            .unwrap()
                            .into_int_value();

                        let char_eq = builder
                            .build_int_compare(inkwell::IntPredicate::EQ, l_char, r_char, "char_eq")
                            .unwrap();

                        let next_idx = builder
                            .build_int_add(idx, context.i64_type().const_int(1, false), "next_idx")
                            .unwrap();
                        let done = builder
                            .build_int_compare(inkwell::IntPredicate::EQ, next_idx, l_len, "done")
                            .unwrap();

                        let continue_cmp = builder
                            .build_and(
                                char_eq,
                                builder.build_not(done, "not_done").unwrap(),
                                "continue_cmp",
                            )
                            .unwrap();
                        let found_match = builder.build_and(char_eq, done, "found_match").unwrap();

                        // If char_eq is false, go to fail. If char_eq is true and not done, loop. If char_eq true and done, match.
                        let not_match = builder.build_not(char_eq, "not_match").unwrap();

                        let next_bb = context.append_basic_block(parent_fn, "str_cmp_next");
                        builder
                            .build_conditional_branch(char_eq, next_bb, fail_bb)
                            .unwrap();

                        builder.position_at_end(next_bb);
                        builder
                            .build_conditional_branch(done, match_bb, loop_bb)
                            .unwrap();
                        index_phi.add_incoming(&[(&next_idx, next_bb)]);

                        // match_bb
                        builder.position_at_end(match_bb);
                        builder.build_unconditional_branch(merge_bb).unwrap();

                        // fail_bb
                        builder.position_at_end(fail_bb);
                        builder.build_unconditional_branch(merge_bb).unwrap();

                        // merge_bb
                        builder.position_at_end(merge_bb);
                        let final_res =
                            builder.build_phi(context.bool_type(), "final_res").unwrap();
                        final_res.add_incoming(&[
                            (&context.bool_type().const_int(1, false), match_bb),
                            (&context.bool_type().const_int(0, false), fail_bb),
                        ]);

                        let res_i64 = builder
                            .build_int_z_extend(
                                final_res.as_basic_value().into_int_value(),
                                context.i64_type(),
                                "res_i64",
                            )
                            .unwrap();
                        if op == &MirBinOp::Ne {
                            builder.build_not(res_i64, "not_res").unwrap().into()
                        } else {
                            res_i64.into()
                        }
                    } else {
                        let cond = builder
                            .build_int_compare(
                                pred,
                                l_val.into_int_value(),
                                r_val.into_int_value(),
                                "cmptmp",
                            )
                            .unwrap();
                        builder
                            .build_int_z_extend(cond, context.i64_type(), "booltmp")
                            .unwrap()
                            .into()
                    }
                }
            };

            let ptr = get_or_create_ssa(context, builder, ssa_storage, *dest, res.get_type());
            builder.build_store(ptr, res).unwrap();
        }
        Ok(())
    }
}

pub struct CallStrategy;
impl<'ctx> InstructionStrategy<'ctx> for CallStrategy {
    fn generate(
        &self,
        context: &'ctx Context,
        module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        registry: &RegistryService,
        ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
        inst: &MirInstruction,
    ) -> Result<(), OnuError> {
        if let MirInstruction::Call {
            dest,
            name,
            args,
            return_type,
            arg_types,
            is_tail_call,
        } = inst
        {
            // ── IO intrinsic interception ─────────────────────────────────
            // These IO effects are implemented with inline x86_64 syscalls
            // (no C / libc dependency).
            match name.as_str() {
                "receives-line" => {
                    return generate_receives_line(context, builder, ssa_storage, *dest);
                }
                "receives-argument" => {
                    let index_val = operand_to_llvm(context, builder, ssa_storage, &args[0]);
                    return generate_receives_argument(context, module, builder, ssa_storage, *dest, index_val);
                }
                "argument-count" => {
                    return generate_argument_count(context, module, builder, ssa_storage, *dest);
                }
                _ => {}
            }

            let llvm_name = name.clone(); // Use original hyphenated names

            // Compute expected LLVM types from the MIR arg_types annotation.
            // Used below to cast arguments that may have a different width
            // (e.g. an I64 constant passed to a WideInt parameter).
            let expected_llvm_types: Vec<Option<inkwell::types::BasicTypeEnum<'ctx>>> = arg_types
                .iter()
                .map(|t| {
                    crate::adapters::codegen::typemapper::LlvmTypeMapper::onu_to_llvm(
                        context, t, registry,
                    )
                })
                .collect();

            let mut llvm_args = Vec::new();
            for (i, arg) in args.iter().enumerate() {
                let val = operand_to_llvm(context, builder, ssa_storage, arg);
                // Width-cast integer arguments to the expected parameter type so
                // that e.g. an i64 constant passed to __onu_wide_div_1024(i1024,i1024)
                // is zero-extended to i1024 rather than causing an LLVM type error.
                let cast_val = if val.is_int_value() {
                    if let Some(Some(expected)) = expected_llvm_types.get(i) {
                        if expected.is_int_type() {
                            let src_w = val.into_int_value().get_type().get_bit_width();
                            let dst_w = expected.into_int_type().get_bit_width();
                            if src_w < dst_w {
                                builder
                                    .build_int_z_extend(
                                        val.into_int_value(),
                                        expected.into_int_type(),
                                        "call_arg_zext",
                                    )
                                    .unwrap()
                                    .into()
                            } else if src_w > dst_w {
                                builder
                                    .build_int_truncate(
                                        val.into_int_value(),
                                        expected.into_int_type(),
                                        "call_arg_trunc",
                                    )
                                    .unwrap()
                                    .into()
                            } else {
                                val
                            }
                        } else {
                            val
                        }
                    } else {
                        val
                    }
                } else {
                    val
                };
                llvm_args.push(cast_val.into());
            }

            let func = if let Some(f) = module.get_function(&llvm_name) {
                f
            } else {
                let llvm_arg_types: Vec<inkwell::types::BasicMetadataTypeEnum> = arg_types
                    .iter()
                    .map(|t| {
                        crate::adapters::codegen::typemapper::LlvmTypeMapper::onu_to_llvm(
                            context, t, registry,
                        )
                        .unwrap_or(context.i64_type().as_basic_type_enum())
                        .into()
                    })
                    .collect();

                let ret_type_opt =
                    crate::adapters::codegen::typemapper::LlvmTypeMapper::onu_to_llvm(
                        context,
                        return_type,
                        registry,
                    );

                if let Some(ret_type) = ret_type_opt {
                    let fn_type = ret_type.fn_type(&llvm_arg_types, false);
                    module.add_function(
                        &llvm_name,
                        fn_type,
                        Some(inkwell::module::Linkage::External),
                    )
                } else {
                    let fn_type = context.void_type().fn_type(&llvm_arg_types, false);
                    module.add_function(
                        &llvm_name,
                        fn_type,
                        Some(inkwell::module::Linkage::External),
                    )
                }
            };

            let call = builder.build_call(func, &llvm_args, "calltmp").unwrap();

            if *is_tail_call {
                eprintln!("DEBUG: Applying musttail to call to {}", name);
                unsafe {
                    // TailCallKind: 0=None, 1=Tail, 2=MustTail, 3=NoTail
                    llvm_sys::core::LLVMSetTailCall(
                        call.as_value_ref(),
                        2, // LLVMTailCallKindMustTail
                    );
                }
            }

            let is_extern = ["malloc", "free", "printf", "puts", "sprintf", "strlen"]
                .contains(&llvm_name.as_str());
            if !is_extern {
                // LLVM fastcc is calling convention 8
                call.set_call_convention(8);
            }

            match call.try_as_basic_value() {
                inkwell::values::ValueKind::Basic(res) => {
                    let ptr =
                        get_or_create_ssa(context, builder, ssa_storage, *dest, res.get_type());
                    builder.build_store(ptr, res).unwrap();
                }
                _ => {
                    // For void calls, we still satisfy SSA dest with 0
                    let i64_res = context.i64_type().const_int(0, false);
                    let ptr = get_or_create_ssa(
                        context,
                        builder,
                        ssa_storage,
                        *dest,
                        i64_res.get_type().as_basic_type_enum(),
                    );
                    builder.build_store(ptr, i64_res).unwrap();
                }
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// IO intrinsic helpers — use the PlatformSyscalls abstraction so that
// strategies remain architecture-agnostic.
// ---------------------------------------------------------------------------

/// `receives-line`: read a line from stdin via the platform read syscall.
/// Returns an Onu string { i64 len, i8* ptr, i1 is_dynamic=false }.
fn generate_receives_line<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
    dest: usize,
) -> Result<(), OnuError> {
    let syscalls = crate::adapters::codegen::platform::create_syscalls();
    let i64_type = context.i64_type();
    let i8_type = context.i8_type();
    let i8_ptr_type = i8_type.ptr_type(inkwell::AddressSpace::default());
    let bool_type = context.bool_type();

    // Stack buffer for reading (4096 bytes).
    let buf_size: u64 = 4096;
    let buf_array_type = i8_type.array_type(buf_size as u32);
    let buf_alloca = builder.build_alloca(buf_array_type, "read_buf").unwrap();
    let buf_ptr = builder
        .build_pointer_cast(buf_alloca, i8_ptr_type, "read_buf_ptr")
        .unwrap();

    let stdin_fd = i64_type.const_int(0, false);
    let max_len = i64_type.const_int(buf_size, false);
    let bytes_read = syscalls.emit_read(context, builder, stdin_fd, buf_ptr, max_len);

    // Strip trailing newline: if last byte == '\n', length -= 1
    let one = i64_type.const_int(1, false);
    let len_minus_1 = builder.build_int_sub(bytes_read, one, "len_m1").unwrap();
    let last_ptr = unsafe {
        builder
            .build_in_bounds_gep(buf_ptr, &[len_minus_1], "last_ptr")
            .unwrap()
    };
    let last_byte = builder
        .build_load(last_ptr, "last_byte")
        .unwrap()
        .into_int_value();
    let is_nl = builder
        .build_int_compare(
            inkwell::IntPredicate::EQ,
            last_byte,
            i8_type.const_int(10, false),
            "is_nl",
        )
        .unwrap();
    let stripped_len = builder
        .build_select(is_nl, len_minus_1, bytes_read, "stripped_len")
        .unwrap()
        .into_int_value();

    // Build Onu string struct { i64 len, i8* ptr, i1 is_dynamic=false }
    let str_type = context.struct_type(&[i64_type.into(), i8_ptr_type.into(), bool_type.into()], false);
    let mut str_val = str_type.get_undef();
    str_val = builder.build_insert_value(str_val, stripped_len, 0, "str_len").unwrap().into_struct_value();
    str_val = builder.build_insert_value(str_val, buf_ptr, 1, "str_ptr").unwrap().into_struct_value();
    str_val = builder.build_insert_value(str_val, bool_type.const_zero(), 2, "str_dyn").unwrap().into_struct_value();

    let ptr = get_or_create_ssa(context, builder, ssa_storage, dest, str_type.as_basic_type_enum());
    builder.build_store(ptr, str_val).unwrap();
    Ok(())
}

/// `receives-argument`: read argv[index] via the `__onu_argv` global.
/// Returns an Onu string { i64 len, i8* ptr, i1 is_dynamic=false }.
fn generate_receives_argument<'ctx>(
    context: &'ctx Context,
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
    dest: usize,
    index_val: BasicValueEnum<'ctx>,
) -> Result<(), OnuError> {
    let i64_type = context.i64_type();
    let i8_type = context.i8_type();
    let i8_ptr_type = i8_type.ptr_type(inkwell::AddressSpace::default());
    let i8_ptr_ptr_type = i8_ptr_type.ptr_type(inkwell::AddressSpace::default());
    let bool_type = context.bool_type();

    // Load __onu_argv global (i8**)
    let argv_global = get_or_declare_global(module, context, "__onu_argv", i8_ptr_ptr_type.as_basic_type_enum());
    let argv_ptr = builder.build_load(argv_global, "argv_val").unwrap().into_pointer_value();

    // GEP to argv[index]
    let idx = if index_val.is_int_value() {
        index_val.into_int_value()
    } else {
        i64_type.const_int(0, false)
    };
    let arg_ptr_ptr = unsafe {
        builder.build_in_bounds_gep(argv_ptr, &[idx], "arg_ptr_ptr").unwrap()
    };
    let arg_ptr = builder.build_load(arg_ptr_ptr, "arg_ptr").unwrap().into_pointer_value();

    // Compute strlen by scanning for '\0' (pure LLVM loop — no libc)
    let current_fn = builder.get_insert_block().unwrap().get_parent().unwrap();
    let strlen_entry = context.append_basic_block(current_fn, "strlen_entry");
    let strlen_loop = context.append_basic_block(current_fn, "strlen_loop");
    let strlen_done = context.append_basic_block(current_fn, "strlen_done");

    builder.build_unconditional_branch(strlen_entry).unwrap();

    builder.position_at_end(strlen_entry);
    builder.build_unconditional_branch(strlen_loop).unwrap();

    builder.position_at_end(strlen_loop);
    let i_phi = builder.build_phi(i64_type, "i").unwrap();
    i_phi.add_incoming(&[(&i64_type.const_int(0, false), strlen_entry)]);
    let i_val = i_phi.as_basic_value().into_int_value();

    let byte_ptr = unsafe {
        builder.build_in_bounds_gep(arg_ptr, &[i_val], "byte_ptr").unwrap()
    };
    let byte = builder.build_load(byte_ptr, "byte").unwrap().into_int_value();
    let is_zero = builder
        .build_int_compare(inkwell::IntPredicate::EQ, byte, i8_type.const_int(0, false), "is_zero")
        .unwrap();
    let i_next = builder
        .build_int_add(i_val, i64_type.const_int(1, false), "i_next")
        .unwrap();
    i_phi.add_incoming(&[(&i_next, strlen_loop)]);
    builder.build_conditional_branch(is_zero, strlen_done, strlen_loop).unwrap();

    builder.position_at_end(strlen_done);
    let str_len = i_phi.as_basic_value().into_int_value();

    // Build Onu string struct
    let str_type = context.struct_type(&[i64_type.into(), i8_ptr_type.into(), bool_type.into()], false);
    let mut str_val = str_type.get_undef();
    str_val = builder.build_insert_value(str_val, str_len, 0, "str_len").unwrap().into_struct_value();
    str_val = builder.build_insert_value(str_val, arg_ptr, 1, "str_ptr").unwrap().into_struct_value();
    str_val = builder.build_insert_value(str_val, bool_type.const_zero(), 2, "str_dyn").unwrap().into_struct_value();

    let ptr = get_or_create_ssa(context, builder, ssa_storage, dest, str_type.as_basic_type_enum());
    builder.build_store(ptr, str_val).unwrap();
    Ok(())
}

/// `argument-count`: return `__onu_argc` (i64) from the global.
fn generate_argument_count<'ctx>(
    context: &'ctx Context,
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
    dest: usize,
) -> Result<(), OnuError> {
    let i64_type = context.i64_type();

    let argc_global = get_or_declare_global(module, context, "__onu_argc", i64_type.as_basic_type_enum());
    let argc_val = builder.build_load(argc_global, "argc_val").unwrap().into_int_value();

    let ptr = get_or_create_ssa(context, builder, ssa_storage, dest, i64_type.as_basic_type_enum());
    builder.build_store(ptr, argc_val).unwrap();
    Ok(())
}

/// Get or declare an internal global variable with the given name and type.
fn get_or_declare_global<'ctx>(
    module: &Module<'ctx>,
    context: &'ctx Context,
    name: &str,
    typ: BasicTypeEnum<'ctx>,
) -> PointerValue<'ctx> {
    if let Some(g) = module.get_global(name) {
        g.as_pointer_value()
    } else {
        let g = module.add_global(typ, None, name);
        g.set_linkage(inkwell::module::Linkage::Internal);
        match typ {
            BasicTypeEnum::IntType(t) => g.set_initializer(&t.const_zero()),
            BasicTypeEnum::PointerType(t) => g.set_initializer(&t.const_null()),
            _ => {}
        }
        g.as_pointer_value()
    }
}

pub struct EmitStrategy;
impl<'ctx> InstructionStrategy<'ctx> for EmitStrategy {
    fn generate(
        &self,
        context: &'ctx Context,
        _module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        _registry: &RegistryService,
        ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
        inst: &MirInstruction,
    ) -> Result<(), OnuError> {
        if let MirInstruction::Emit(op) = inst {
            let syscalls = crate::adapters::codegen::platform::create_syscalls();
            let val = operand_to_llvm(context, builder, ssa_storage, op);

            // Onu Strings are { i64 len, i8* ptr, i1 is_dynamic }
            if val.is_struct_value() {
                let s = val.into_struct_value();
                let len = builder.build_extract_value(s, 0, "emit_len").unwrap().into_int_value();
                let ptr = builder.build_extract_value(s, 1, "emit_ptr").unwrap().into_pointer_value();

                let i64_type = context.i64_type();
                let stdout_fd = i64_type.const_int(1, false);

                // 1. Write the string to stdout
                syscalls.emit_write(context, builder, stdout_fd, ptr, len);

                // 2. Write a trailing newline (\n = ASCII 10)
                let nl_val = context.i8_type().const_int(10, false);
                let nl_ptr = builder.build_alloca(context.i8_type(), "nl_ptr").unwrap();
                builder.build_store(nl_ptr, nl_val).unwrap();
                let one = i64_type.const_int(1, false);
                syscalls.emit_write(context, builder, stdout_fd, nl_ptr, one);
            }
        }
        Ok(())
    }
}

pub struct AssignStrategy;
impl<'ctx> InstructionStrategy<'ctx> for AssignStrategy {
    fn generate(
        &self,
        context: &'ctx Context,
        _module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        registry: &RegistryService,
        ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
        inst: &MirInstruction,
    ) -> Result<(), OnuError> {
        if let MirInstruction::Assign { dest, src } = inst {
            let val = operand_to_llvm(context, builder, ssa_storage, src);
            let ptr = get_or_create_ssa(context, builder, ssa_storage, *dest, val.get_type());
            builder.build_store(ptr, val).unwrap();
        }
        Ok(())
    }
}

pub struct DropStrategy;
impl<'ctx> InstructionStrategy<'ctx> for DropStrategy {
    fn generate(
        &self,
        _context: &'ctx Context,
        _module: &Module<'ctx>,
        _builder: &Builder<'ctx>,
        _registry: &RegistryService,
        _ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
        _inst: &MirInstruction,
    ) -> Result<(), OnuError> {
        // ZERO-COST ACHIEVEMENT: Pure LLVM stack allocations clean themselves up via the call stack automatically.
        // Therefore, Drop logic is a true no-op.
        Ok(())
    }
}

pub struct TupleStrategy;
impl<'ctx> InstructionStrategy<'ctx> for TupleStrategy {
    fn generate(
        &self,
        context: &'ctx Context,
        _module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        registry: &RegistryService,
        ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
        inst: &MirInstruction,
    ) -> Result<(), OnuError> {
        if let MirInstruction::Tuple { dest, elements } = inst {
            let mut element_vals = Vec::new();
            let mut element_types = Vec::new();

            for el in elements {
                let val = operand_to_llvm(context, builder, ssa_storage, el);
                element_vals.push(val);
                element_types.push(val.get_type());
            }

            let tuple_type = context.struct_type(&element_types, false);
            let mut tuple_val = tuple_type.get_undef();

            for (i, val) in element_vals.iter().enumerate() {
                tuple_val = builder
                    .build_insert_value(tuple_val, *val, i as u32, &format!("insert_{}", i))
                    .unwrap()
                    .into_struct_value();
            }

            let ptr = get_or_create_ssa(
                context,
                builder,
                ssa_storage,
                *dest,
                tuple_val.get_type().as_basic_type_enum(),
            );
            builder.build_store(ptr, tuple_val).unwrap();
        }
        Ok(())
    }
}

pub struct AllocStrategy;
impl<'ctx> InstructionStrategy<'ctx> for AllocStrategy {
    fn generate(
        &self,
        context: &'ctx Context,
        module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        _registry: &RegistryService,
        ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
        inst: &MirInstruction,
    ) -> Result<(), OnuError> {
        if let MirInstruction::Alloc { dest, size_bytes } = inst {
            let size_val =
                operand_to_llvm(context, builder, ssa_storage, size_bytes).into_int_value();

            // Simple Bump Allocator from @onu_arena
            let arena_ptr_global = module
                .get_global("onu_arena_ptr")
                .unwrap()
                .as_pointer_value();

            // 1. Load current pointer
            let current_ptr = builder
                .build_load(arena_ptr_global, "current_arena_ptr")
                .unwrap()
                .into_pointer_value();

            // 2. Calculate next pointer (current + size)
            let next_ptr = unsafe {
                builder
                    .build_in_bounds_gep(current_ptr, &[size_val], "next_arena_ptr")
                    .unwrap()
            };

            // 3. Store next pointer back to global
            builder.build_store(arena_ptr_global, next_ptr).unwrap();

            // 4. Return the ORIGINAL current_ptr as the allocated address
            let ptr = get_or_create_ssa(
                context,
                builder,
                ssa_storage,
                *dest,
                current_ptr.get_type().as_basic_type_enum(),
            );
            builder.build_store(ptr, current_ptr).unwrap();
        }
        Ok(())
    }
}

/// Emits (or re-uses) a module-level zeroed byte-array global and yields a
/// pointer to its first element.  The global is zero-initialised once by the
/// OS/loader and persists for the program lifetime, making it safe to use as
/// a memoisation cache backing that survives across multiple wrapper calls.
pub struct GlobalAllocStrategy;
impl<'ctx> InstructionStrategy<'ctx> for GlobalAllocStrategy {
    fn generate(
        &self,
        context: &'ctx Context,
        module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        _registry: &RegistryService,
        ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
        inst: &MirInstruction,
    ) -> Result<(), OnuError> {
        if let MirInstruction::GlobalAlloc { dest, size_bytes, name } = inst {
            let i8_type = context.i8_type();
            // Guard against extremely large (> 4 GiB) allocations that would
            // truncate silently when cast to u32.  In practice memo caches are
            // bounded by ARENA_SIZE_BYTES (16 MiB), so this is a safety net.
            assert!(
                *size_bytes <= u32::MAX as usize,
                "GlobalAlloc: size_bytes {} exceeds u32::MAX; cannot create LLVM array type",
                size_bytes
            );
            let array_type = i8_type.array_type(*size_bytes as u32);

            // Reuse an existing global if one with this name was already emitted
            // (e.g. if the same function appears in multiple compilation units).
            let global = if let Some(g) = module.get_global(name) {
                g
            } else {
                let g = module.add_global(array_type, None, name);
                g.set_initializer(&array_type.const_zero());
                g.set_linkage(inkwell::module::Linkage::Internal);
                g
            };

            // GEP to get an i8* pointer to element 0.
            let zero = context.i64_type().const_zero();
            let ptr = unsafe {
                builder
                    .build_in_bounds_gep(
                        global.as_pointer_value(),
                        &[zero, zero],
                        &format!("{}_ptr", name),
                    )
                    .unwrap()
            };

            let slot = get_or_create_ssa(
                context,
                builder,
                ssa_storage,
                *dest,
                ptr.get_type().as_basic_type_enum(),
            );
            builder.build_store(slot, ptr).unwrap();
        }
        Ok(())
    }
}

pub struct MemCopyStrategy;
impl<'ctx> InstructionStrategy<'ctx> for MemCopyStrategy {
    fn generate(
        &self,
        context: &'ctx Context,
        module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        registry: &RegistryService,
        ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
        inst: &MirInstruction,
    ) -> Result<(), OnuError> {
        if let MirInstruction::MemCopy { dest, src, size } = inst {
            let dest_val = operand_to_llvm(context, builder, ssa_storage, dest);
            let src_val = operand_to_llvm(context, builder, ssa_storage, src);
            let size_val = operand_to_llvm(context, builder, ssa_storage, size);

            // LLVM intrinsic for memcpy: @llvm.memcpy.p0i8.p0i8.i64(i8* align 1 %dest, i8* align 1 %src, i64 %size, i1 %isvolatile)
            let i8_ptr_type = context.i8_type().ptr_type(inkwell::AddressSpace::default());
            let i64_type = context.i64_type();
            let bool_type = context.bool_type();

            let memcpy_type = context.void_type().fn_type(
                &[
                    i8_ptr_type.into(),
                    i8_ptr_type.into(),
                    i64_type.into(),
                    bool_type.into(),
                ],
                false,
            );

            let memcpy_fn = module
                .get_function("llvm.memcpy.p0i8.p0i8.i64")
                .unwrap_or_else(|| {
                    module.add_function(
                        "llvm.memcpy.p0i8.p0i8.i64",
                        memcpy_type,
                        Some(inkwell::module::Linkage::External),
                    )
                });

            builder
                .build_call(
                    memcpy_fn,
                    &[
                        dest_val.into(),
                        src_val.into(),
                        size_val.into(),
                        context.bool_type().const_int(0, false).into(), // isvolatile = false
                    ],
                    "memcpy_call",
                )
                .unwrap();
        }
        Ok(())
    }
}

pub struct PointerOffsetStrategy;
impl<'ctx> InstructionStrategy<'ctx> for PointerOffsetStrategy {
    fn generate(
        &self,
        context: &'ctx Context,
        _module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        registry: &RegistryService,
        ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
        inst: &MirInstruction,
    ) -> Result<(), OnuError> {
        if let MirInstruction::PointerOffset { dest, ptr, offset } = inst {
            let ptr_val = operand_to_llvm(context, builder, ssa_storage, ptr).into_pointer_value();
            let offset_val =
                operand_to_llvm(context, builder, ssa_storage, offset).into_int_value();

            // GEPI (GetElementPtr) for i8*
            let offset_ptr = unsafe {
                builder
                    .build_in_bounds_gep(ptr_val, &[offset_val], "offset_ptr")
                    .unwrap()
            };

            let ptr_ssa = get_or_create_ssa(
                context,
                builder,
                ssa_storage,
                *dest,
                offset_ptr.get_type().as_basic_type_enum(),
            );
            builder.build_store(ptr_ssa, offset_ptr).unwrap();
        }
        Ok(())
    }
}

/// Typed load from a raw pointer (i8*) produced by PointerOffset.
///
/// The memoization cache is an i8 byte array.  After PointerOffset navigates
/// to the correct 8-byte-aligned slot, we need to load an i64.  LLVM requires
/// the pointer type to match the load width, so we bitcast the i8* to i64*
/// before loading.
///
/// Design Pattern: Strategy — same interface as every other instruction
/// strategy, selected by the codegen dispatcher based on the MirInstruction
/// variant.
pub struct LoadStrategy;
impl<'ctx> InstructionStrategy<'ctx> for LoadStrategy {
    fn generate(
        &self,
        context: &'ctx Context,
        _module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        registry: &RegistryService,
        ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
        inst: &MirInstruction,
    ) -> Result<(), OnuError> {
        if let MirInstruction::Load { dest, ptr, typ } = inst {
            let ptr_val = operand_to_llvm(context, builder, ssa_storage, ptr).into_pointer_value();

            // Cast the raw i8* to the target element pointer type.
            // For i64 values: i8* → i64* so that build_load reads 8 bytes.
            // For WideInt(N): i8* → iN* so that build_load reads N/8 bytes.
            let dest_llvm_type = crate::adapters::codegen::typemapper::LlvmTypeMapper::onu_to_llvm(context, typ, registry)
                .unwrap_or_else(|| context.i64_type().as_basic_type_enum());
            let typed_ptr = builder
                .build_pointer_cast(
                    ptr_val,
                    dest_llvm_type.ptr_type(inkwell::AddressSpace::default()),
                    "typed_ptr",
                )
                .unwrap();

            let loaded_val = builder.build_load(typed_ptr, "loaded_val").unwrap();
            if let inkwell::values::BasicValueEnum::IntValue(iv) = loaded_val {
                if iv.get_type().get_bit_width() == 64 {
                    unsafe {
                        let load_inst = loaded_val.as_value_ref();
                        llvm_sys::core::LLVMSetAlignment(load_inst, 8);
                    }
                }
            }
            let ssa_ptr = get_or_create_ssa(context, builder, ssa_storage, *dest, dest_llvm_type);
            builder.build_store(ssa_ptr, loaded_val).unwrap();
        }
        Ok(())
    }
}

pub struct StoreStrategy;

impl<'ctx> InstructionStrategy<'ctx> for StoreStrategy {
    fn generate(
        &self,
        context: &'ctx Context,
        _module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        registry: &RegistryService,
        ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
        inst: &MirInstruction,
    ) -> Result<(), OnuError> {
        if let MirInstruction::Store { ptr, value } = inst {
            let ptr_val = operand_to_llvm(context, builder, ssa_storage, ptr).into_pointer_value();
            let val = operand_to_llvm(context, builder, ssa_storage, value);

            // If storing i64 to i8*, we might need a cast if LLVM is strict,
            // but usually build_store handles basic values.
            // However, we want to store it as a byte if it's for itoa.
            let val_to_store = if ptr_val.get_type().get_element_type().is_int_type() {
                let target_type = ptr_val.get_type().get_element_type().into_int_type();
                if val.get_type().into_int_type().get_bit_width() > target_type.get_bit_width() {
                    builder
                        .build_int_truncate(val.into_int_value(), target_type, "store_trunc")
                        .unwrap()
                        .into()
                } else {
                    val
                }
            } else {
                val
            };

            builder.build_store(ptr_val, val_to_store).unwrap();
        }
        Ok(())
    }
}

/// Typed store to a raw pointer (i8*) produced by PointerOffset.
///
/// The memoization cache is an i8 byte array. After computing a result we need
/// to store an i64.  The regular StoreStrategy would truncate the i64 to i8
/// because the target pointer is i8*.  TypedStoreStrategy explicitly bitcasts
/// to typ* before calling build_store, preserving all 64 bits.
pub struct TypedStoreStrategy;
impl<'ctx> InstructionStrategy<'ctx> for TypedStoreStrategy {
    fn generate(
        &self,
        context: &'ctx Context,
        _module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        registry: &RegistryService,
        ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
        inst: &MirInstruction,
    ) -> Result<(), OnuError> {
        if let MirInstruction::TypedStore { ptr, value, typ } = inst {
            let ptr_val = operand_to_llvm(context, builder, ssa_storage, ptr).into_pointer_value();
            let val = operand_to_llvm(context, builder, ssa_storage, value);

            // Cast the i8* to the element pointer type matching the value width.
            // WideInt(N) → iN* so that all N/8 bytes are written.
            let dest_llvm_type =
                crate::adapters::codegen::typemapper::LlvmTypeMapper::onu_to_llvm(
                    context, typ, registry,
                )
                .unwrap_or_else(|| context.i64_type().as_basic_type_enum());
            let typed_ptr = builder
                .build_pointer_cast(
                    ptr_val,
                    dest_llvm_type.ptr_type(inkwell::AddressSpace::default()),
                    "typed_store_ptr",
                )
                .unwrap();

            // Guard: if the source value is wider than the destination type, truncate
            // before storing.  This is the key case for the occupancy flag: the MIR
            // emits MirLiteral::I64(1) (i64) but the destination is typed as I8 (i8*).
            // LLVM requires the stored value width to match the pointer element type.
            let val_to_store = if val.is_int_value() && dest_llvm_type.is_int_type() {
                let src_bits = val.into_int_value().get_type().get_bit_width();
                let dst_bits = dest_llvm_type.into_int_type().get_bit_width();
                if src_bits > dst_bits {
                    builder
                        .build_int_truncate(
                            val.into_int_value(),
                            dest_llvm_type.into_int_type(),
                            "typed_store_trunc",
                        )
                        .unwrap()
                        .into()
                } else {
                    val
                }
            } else {
                val
            };
            let store_inst = builder.build_store(typed_ptr, val_to_store).unwrap();
            if val_to_store.is_int_value() && val_to_store.into_int_value().get_type().get_bit_width() == 64 {
                unsafe {
                    llvm_sys::core::LLVMSetAlignment(store_inst.as_value_ref(), 8);
                }
            }
        }
        Ok(())
    }
}

pub struct MemSetStrategy;
impl<'ctx> InstructionStrategy<'ctx> for MemSetStrategy {
    fn generate(
        &self,
        context: &'ctx Context,
        module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        _registry: &RegistryService,
        ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
        inst: &MirInstruction,
    ) -> Result<(), OnuError> {
        if let MirInstruction::MemSet { ptr, value, size } = inst {
            let ptr_val = operand_to_llvm(context, builder, ssa_storage, ptr).into_pointer_value();
            let value_val = operand_to_llvm(context, builder, ssa_storage, value).into_int_value();
            let size_val = operand_to_llvm(context, builder, ssa_storage, size).into_int_value();

            // LLVM intrinsic for memset: @llvm.memset.p0i8.i64(i8* align 1 %ptr, i8 %value, i64 %size, i1 %isvolatile)
            let i8_ptr_type = context.i8_type().ptr_type(inkwell::AddressSpace::default());
            let i8_type = context.i8_type();
            let i64_type = context.i64_type();
            let bool_type = context.bool_type();

            let memset_type = context.void_type().fn_type(
                &[
                    i8_ptr_type.into(),
                    i8_type.into(),
                    i64_type.into(),
                    bool_type.into(),
                ],
                false,
            );

            let memset_fn = module
                .get_function("llvm.memset.p0i8.i64")
                .unwrap_or_else(|| {
                    module.add_function(
                        "llvm.memset.p0i8.i64",
                        memset_type,
                        Some(inkwell::module::Linkage::External),
                    )
                });

            // Ensure value is i8
            let val_i8 = if value_val.get_type().get_bit_width() > 8 {
                builder
                    .build_int_truncate(value_val, i8_type, "memset_val_trunc")
                    .unwrap()
            } else {
                value_val
            };

            builder
                .build_call(
                    memset_fn,
                    &[
                        ptr_val.into(),
                        val_i8.into(),
                        size_val.into(),
                        context.bool_type().const_int(0, false).into(), // isvolatile = false
                    ],
                    "memset_call",
                )
                .unwrap();
        }
        Ok(())
    }
}

pub struct IndexStrategy;

impl<'ctx> InstructionStrategy<'ctx> for IndexStrategy {
    fn generate(
        &self,
        context: &'ctx Context,
        _module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        registry: &RegistryService,
        ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
        inst: &MirInstruction,
    ) -> Result<(), OnuError> {
        if let MirInstruction::Index {
            dest,
            subject,
            index,
        } = inst
        {
            let val = operand_to_llvm(context, builder, ssa_storage, subject);
            if let BasicValueEnum::StructValue(s) = val {
                let elem = builder
                    .build_extract_value(s, *index as u32, "index_tmp")
                    .unwrap();
                let ptr = get_or_create_ssa(context, builder, ssa_storage, *dest, elem.get_type());
                builder.build_store(ptr, elem).unwrap();
            } else if let BasicValueEnum::PointerValue(p) = val {
                // Heuristic: if it's a pointer and we're indexing it, it's likely a load
                // For now, assume it's a load of i64 (char code) if index is 0.
                // In a more robust implementation, we'd use the SSA type.
                let elem_type = if let Some(typ) = builder
                    .get_insert_block()
                    .and_then(|bb| bb.get_parent())
                    .and_then(|f| {
                        // This is hard to get from here. Let's use i64 as default for Onu chars.
                        Some(context.i64_type().as_basic_type_enum())
                    }) {
                    typ
                } else {
                    context.i64_type().as_basic_type_enum()
                };

                // GEPI if index > 0, then load
                let target_ptr = if *index > 0 {
                    unsafe {
                        builder
                            .build_in_bounds_gep(
                                p,
                                &[context.i64_type().const_int(*index as u64, false)],
                                "idx_ptr",
                            )
                            .unwrap()
                    }
                } else {
                    p
                };

                let elem = builder.build_load(target_ptr, "index_load").unwrap();
                // Special case for byte load: extend to i64
                let final_elem = if elem.get_type().is_int_type()
                    && elem.into_int_value().get_type().get_bit_width() == 8
                {
                    builder
                        .build_int_z_extend(elem.into_int_value(), context.i64_type(), "char_ext")
                        .unwrap()
                        .into()
                } else {
                    elem
                };

                let ptr =
                    get_or_create_ssa(context, builder, ssa_storage, *dest, final_elem.get_type());
                builder.build_store(ptr, final_elem).unwrap();
            }
        }
        Ok(())
    }
}

// --- Internal Helper Functions ---

pub fn operand_to_llvm<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    ssa_storage: &HashMap<usize, PointerValue<'ctx>>,
    op: &MirOperand,
) -> BasicValueEnum<'ctx> {
    match op {
        MirOperand::Constant(lit) => match lit {
            MirLiteral::I64(n) => context.i64_type().const_int(*n as u64, true).into(),
            MirLiteral::F64(bits) => context.f64_type().const_float(f64::from_bits(*bits)).into(),
            MirLiteral::Boolean(b) => context
                .bool_type()
                .const_int(if *b { 1 } else { 0 }, false)
                .into(),
            MirLiteral::Text(s) => {
                let length = context.i64_type().const_int(s.len() as u64, false);
                let global_str = builder.build_global_string_ptr(s, "strtmp").unwrap();
                let i64_type = context.i64_type();
                let i8_ptr_type = context.i8_type().ptr_type(inkwell::AddressSpace::default());
                let bool_type = context.bool_type();
                let is_dynamic = bool_type.const_int(0, false); // Literal strings are not dynamic

                let string_struct_type = context.struct_type(
                    &[i64_type.into(), i8_ptr_type.into(), bool_type.into()],
                    false,
                );

                // Achievement: Use const_named_struct to ensure this is a compile-time constant
                let string_val = string_struct_type.const_named_struct(&[
                    length.into(),
                    global_str.as_pointer_value().into(),
                    is_dynamic.into(),
                ]);

                string_val.into()
            }
            MirLiteral::Nothing => context.i64_type().const_int(0, false).into(),
            MirLiteral::WideInt(val_str, bits) => {
                context.custom_width_int_type(*bits).const_int_from_string(val_str, inkwell::types::StringRadix::Decimal).unwrap().into()
            }
        },
        MirOperand::Variable(id, _) => {
            let ptr = ssa_storage
                .get(id)
                .expect(&format!("SSA variable {} not found", id));
            builder.build_load(*ptr, &format!("v{}", id)).unwrap()
        }
    }
}

pub fn get_or_create_ssa<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
    id: usize,
    typ: BasicTypeEnum<'ctx>,
) -> PointerValue<'ctx> {
    if let Some(ptr) = ssa_storage.get(&id) {
        return *ptr;
    }

    let current_bb = builder.get_insert_block().unwrap();
    let function = current_bb.get_parent().unwrap();
    let entry_bb = function.get_first_basic_block().unwrap();

    let temp_builder = context.create_builder();
    if let Some(first_inst) = entry_bb.get_first_instruction() {
        temp_builder.position_before(&first_inst);
    } else {
        temp_builder.position_at_end(entry_bb);
    }
    let ptr = temp_builder.build_alloca(typ, &format!("v{}", id)).unwrap();
    ssa_storage.insert(id, ptr);
    ptr
}

/// Strategy for the `BitCast` MIR instruction.
///
/// Reinterprets the bit-pattern of the source operand as the target type,
/// using LLVM's `bitcast` instruction.  This is the codegen implementation
/// of the Clean Architecture boundary described in the problem statement:
/// it allows the compiler to safely transition from a "Mathematical Integer"
/// (e.g. WideInt(1024)) to a "Memory Detail" (e.g. an array of i64 limbs).
pub struct BitCastStrategy;
impl<'ctx> InstructionStrategy<'ctx> for BitCastStrategy {
    fn generate(
        &self,
        context: &'ctx Context,
        _module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        registry: &RegistryService,
        ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
        inst: &MirInstruction,
    ) -> Result<(), OnuError> {
        if let MirInstruction::BitCast { dest, src, to_type } = inst {
            let src_val = operand_to_llvm(context, builder, ssa_storage, src);
            let target_llvm_type =
                crate::adapters::codegen::typemapper::LlvmTypeMapper::onu_to_llvm(
                    context, to_type, registry,
                )
                .ok_or_else(|| OnuError::CodeGenError {
                    message: format!("BitCast: cannot map target type {:?} to LLVM", to_type),
                })?;

            let cast_val = builder
                .build_bit_cast(src_val, target_llvm_type, "bitcast_tmp")
                .map_err(|e| OnuError::CodeGenError {
                    message: format!("BitCast build failed: {:?}", e),
                })?;

            let ptr = get_or_create_ssa(context, builder, ssa_storage, *dest, cast_val.get_type());
            builder.build_store(ptr, cast_val).unwrap();
        }
        Ok(())
    }
}
