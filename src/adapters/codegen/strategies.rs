/// Codegen Strategies: Interface Adapter Layer
///
/// This module implements the Strategy Pattern for MIR Instruction generation.
/// Each strategy is responsible for translating a specific MIR instruction 
/// into the corresponding LLVM IR.

use crate::domain::entities::mir::{MirInstruction, MirBinOp, MirOperand, MirLiteral};
use crate::domain::entities::error::OnuError;
use crate::application::use_cases::registry_service::RegistryService;

use inkwell::context::Context;
use inkwell::builder::Builder;
use inkwell::module::Module;
use inkwell::values::{PointerValue, BasicValueEnum, AnyValue, CallableValue};
use std::convert::TryFrom;
use inkwell::types::{BasicTypeEnum, BasicType};
use std::collections::HashMap;

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
        if let MirInstruction::BinaryOperation { dest, op, lhs, rhs } = inst {
            let l_val = operand_to_llvm(context, builder, ssa_storage, lhs);
            let r_val = operand_to_llvm(context, builder, ssa_storage, rhs);

            let res: BasicValueEnum = match op {
                MirBinOp::Add => builder.build_int_add(l_val.into_int_value(), r_val.into_int_value(), "addtmp").unwrap().into(),
                MirBinOp::Sub => builder.build_int_sub(l_val.into_int_value(), r_val.into_int_value(), "subtmp").unwrap().into(),
                MirBinOp::Mul => builder.build_int_mul(l_val.into_int_value(), r_val.into_int_value(), "multmp").unwrap().into(),
                MirBinOp::Div => builder.build_int_signed_div(l_val.into_int_value(), r_val.into_int_value(), "divtmp").unwrap().into(),
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
                        
                        let l_len = builder.build_extract_value(l_struct, 0, "l_len").unwrap().into_int_value();
                        let r_len = builder.build_extract_value(r_struct, 0, "r_len").unwrap().into_int_value();
                        
                        let l_ptr = builder.build_extract_value(l_struct, 1, "l_ptr").unwrap().into_pointer_value();
                        let r_ptr = builder.build_extract_value(r_struct, 1, "r_ptr").unwrap().into_pointer_value();

                        // 1. Compare lengths
                        let len_eq = builder.build_int_compare(inkwell::IntPredicate::EQ, l_len, r_len, "len_eq").unwrap();
                        
                        let current_bb = builder.get_insert_block().unwrap();
                        let parent_fn = current_bb.get_parent().unwrap();
                        
                        let cmp_bb = context.append_basic_block(parent_fn, "str_cmp_start");
                        let loop_bb = context.append_basic_block(parent_fn, "str_cmp_loop");
                        let match_bb = context.append_basic_block(parent_fn, "str_match");
                        let fail_bb = context.append_basic_block(parent_fn, "str_fail");
                        let merge_bb = context.append_basic_block(parent_fn, "str_cmp_merge");

                        builder.build_conditional_branch(len_eq, cmp_bb, fail_bb).unwrap();

                        // cmp_bb: if len is 0, they match
                        builder.position_at_end(cmp_bb);
                        let is_empty = builder.build_int_compare(inkwell::IntPredicate::EQ, l_len, context.i64_type().const_zero(), "is_empty").unwrap();
                        builder.build_conditional_branch(is_empty, match_bb, loop_bb).unwrap();

                        // loop_bb: compare chars
                        builder.position_at_end(loop_bb);
                        let index_phi = builder.build_phi(context.i64_type(), "index").unwrap();
                        index_phi.add_incoming(&[(&context.i64_type().const_zero(), cmp_bb)]);
                        
                        let idx = index_phi.as_basic_value().into_int_value();
                        let l_char_ptr = unsafe { builder.build_in_bounds_gep(l_ptr, &[idx], "l_char_ptr").unwrap() };
                        let r_char_ptr = unsafe { builder.build_in_bounds_gep(r_ptr, &[idx], "r_char_ptr").unwrap() };
                        
                        let l_char = builder.build_load(l_char_ptr, "l_char").unwrap().into_int_value();
                        let r_char = builder.build_load(r_char_ptr, "r_char").unwrap().into_int_value();
                        
                        let char_eq = builder.build_int_compare(inkwell::IntPredicate::EQ, l_char, r_char, "char_eq").unwrap();
                        
                        let next_idx = builder.build_int_add(idx, context.i64_type().const_int(1, false), "next_idx").unwrap();
                        let done = builder.build_int_compare(inkwell::IntPredicate::EQ, next_idx, l_len, "done").unwrap();
                        
                        let continue_cmp = builder.build_and(char_eq, builder.build_not(done, "not_done").unwrap(), "continue_cmp").unwrap();
                        let found_match = builder.build_and(char_eq, done, "found_match").unwrap();
                        
                        // If char_eq is false, go to fail. If char_eq is true and not done, loop. If char_eq true and done, match.
                        let not_match = builder.build_not(char_eq, "not_match").unwrap();
                        
                        let next_bb = context.append_basic_block(parent_fn, "str_cmp_next");
                        builder.build_conditional_branch(char_eq, next_bb, fail_bb).unwrap();
                        
                        builder.position_at_end(next_bb);
                        builder.build_conditional_branch(done, match_bb, loop_bb).unwrap();
                        index_phi.add_incoming(&[(&next_idx, next_bb)]);

                        // match_bb
                        builder.position_at_end(match_bb);
                        builder.build_unconditional_branch(merge_bb).unwrap();

                        // fail_bb
                        builder.position_at_end(fail_bb);
                        builder.build_unconditional_branch(merge_bb).unwrap();

                        // merge_bb
                        builder.position_at_end(merge_bb);
                        let final_res = builder.build_phi(context.bool_type(), "final_res").unwrap();
                        final_res.add_incoming(&[(&context.bool_type().const_int(1, false), match_bb), (&context.bool_type().const_int(0, false), fail_bb), (&context.bool_type().const_int(0, false), current_bb)]);
                        
                        let res_i64 = builder.build_int_z_extend(final_res.as_basic_value().into_int_value(), context.i64_type(), "res_i64").unwrap();
                        if op == &MirBinOp::Ne {
                            builder.build_not(res_i64, "not_res").unwrap().into()
                        } else {
                            res_i64.into()
                        }
                    } else {
                        let cond = builder.build_int_compare(pred, l_val.into_int_value(), r_val.into_int_value(), "cmptmp").unwrap();
                        builder.build_int_z_extend(cond, context.i64_type(), "booltmp").unwrap().into()
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
        if let MirInstruction::Call { dest, name, args, return_type, arg_types } = inst {
            let llvm_name = name.clone(); // Use original hyphenated names
            
            let mut llvm_args = Vec::new();
            for arg in args {
                let val = operand_to_llvm(context, builder, ssa_storage, arg);
                llvm_args.push(val.into());
            }

            let func = if let Some(f) = module.get_function(&llvm_name) {
                f
            } else {
                let llvm_arg_types: Vec<inkwell::types::BasicMetadataTypeEnum> = arg_types.iter()
                    .map(|t| crate::adapters::codegen::typemapper::LlvmTypeMapper::onu_to_llvm(context, t, registry).unwrap_or(context.i64_type().as_basic_type_enum()).into())
                    .collect();

                let ret_type_opt = crate::adapters::codegen::typemapper::LlvmTypeMapper::onu_to_llvm(context, return_type, registry);
                
                if let Some(ret_type) = ret_type_opt {
                    let fn_type = ret_type.fn_type(&llvm_arg_types, false);
                    module.add_function(&llvm_name, fn_type, Some(inkwell::module::Linkage::External))
                } else {
                    let fn_type = context.void_type().fn_type(&llvm_arg_types, false);
                    module.add_function(&llvm_name, fn_type, Some(inkwell::module::Linkage::External))
                }
            };
            
            let call = builder.build_call(func, &llvm_args, "calltmp").unwrap();
            
            let is_extern = ["malloc", "free", "printf", "puts", "sprintf", "strlen"].contains(&llvm_name.as_str());
            if !is_extern {
                // LLVM fastcc is calling convention 8
                call.set_call_convention(8);
            }

            match call.try_as_basic_value() {
                inkwell::values::ValueKind::Basic(res) => {
                    let ptr = get_or_create_ssa(context, builder, ssa_storage, *dest, res.get_type());
                    builder.build_store(ptr, res).unwrap();
                }
                _ => {
                    // For void calls, we still satisfy SSA dest with 0
                    let i64_res = context.i64_type().const_int(0, false);
                    let ptr = get_or_create_ssa(context, builder, ssa_storage, *dest, i64_res.get_type().as_basic_type_enum());
                    builder.build_store(ptr, i64_res).unwrap();
                }
            }
        }
        Ok(())
    }
}

pub struct EmitStrategy;
impl<'ctx> InstructionStrategy<'ctx> for EmitStrategy {
    fn generate(
        &self,
        context: &'ctx Context,
        module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        registry: &RegistryService,
        ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
        inst: &MirInstruction,
    ) -> Result<(), OnuError> {
        if let MirInstruction::Emit(op) = inst {
            let val = operand_to_llvm(context, builder, ssa_storage, op);
            
            // Onu Strings are { i64 len, i8* ptr, i1 is_dynamic }
            if val.is_struct_value() {
                let s = val.into_struct_value();
                let len = builder.build_extract_value(s, 0, "emit_len").unwrap().into_int_value();
                let ptr = builder.build_extract_value(s, 1, "emit_ptr").unwrap().into_pointer_value();

                // x86_64 syscall: %rax=1 (write), %rdi=1 (stdout), %rsi=buffer, %rdx=length
                // Clobbers: rcx, r11
                let i64_type = context.i64_type();
                let i8_ptr_type = context.i8_type().ptr_type(inkwell::AddressSpace::default());
                
                let syscall_type = i64_type.fn_type(&[
                    i64_type.into(), // rax
                    i64_type.into(), // rdi
                    i8_ptr_type.into(), // rsi
                    i64_type.into(), // rdx
                ], false);

                let asm_fn = context.create_inline_asm(
                    syscall_type,
                    "syscall".to_string(),
                    "={ax},{ax},{di},{si},{dx},~{rcx},~{r11},~{dirflag},~{fpsr},~{flags}".to_string(),
                    true,
                    false,
                    None,
                    false, // Missing boolean: can_throw?
                );

                builder.build_call(CallableValue::try_from(asm_fn).unwrap(), &[
                    i64_type.const_int(1, false).into(), // sys_write
                    i64_type.const_int(1, false).into(), // stdout
                    ptr.into(),
                    len.into(),
                ], "syscall_res").unwrap();

                // 2. EMIT NEWLINE (\n = ASCII 10)
                // We allocate a small stack buffer for the newline
                let nl_val = context.i8_type().const_int(10, false);
                let nl_ptr = builder.build_alloca(context.i8_type(), "nl_ptr").unwrap();
                builder.build_store(nl_ptr, nl_val).unwrap();

                builder.build_call(CallableValue::try_from(asm_fn).unwrap(), &[
                    i64_type.const_int(1, false).into(), // sys_write
                    i64_type.const_int(1, false).into(), // stdout
                    nl_ptr.into(),
                    i64_type.const_int(1, false).into(), // len 1
                ], "syscall_res").unwrap();

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
                tuple_val = builder.build_insert_value(tuple_val, *val, i as u32, &format!("insert_{}", i)).unwrap().into_struct_value();
            }

            let ptr = get_or_create_ssa(context, builder, ssa_storage, *dest, tuple_val.get_type().as_basic_type_enum());
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
            let size_val = operand_to_llvm(context, builder, ssa_storage, size_bytes).into_int_value();

            // Simple Bump Allocator from @onu_arena
            let arena_ptr_global = module.get_global("onu_arena_ptr").unwrap().as_pointer_value();
            
            // 1. Load current pointer
            let current_ptr = builder.build_load(arena_ptr_global, "current_arena_ptr").unwrap().into_pointer_value();
            
            // 2. Calculate next pointer (current + size)
            let next_ptr = unsafe { builder.build_in_bounds_gep(current_ptr, &[size_val], "next_arena_ptr").unwrap() };
            
            // 3. Store next pointer back to global
            builder.build_store(arena_ptr_global, next_ptr).unwrap();

            // 4. Return the ORIGINAL current_ptr as the allocated address
            let ptr = get_or_create_ssa(context, builder, ssa_storage, *dest, current_ptr.get_type().as_basic_type_enum());
            builder.build_store(ptr, current_ptr).unwrap();
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

            let memcpy_type = context.void_type().fn_type(&[
                i8_ptr_type.into(),
                i8_ptr_type.into(),
                i64_type.into(),
                bool_type.into()
            ], false);

            let memcpy_fn = module.get_function("llvm.memcpy.p0i8.p0i8.i64").unwrap_or_else(|| {
                module.add_function("llvm.memcpy.p0i8.p0i8.i64", memcpy_type, Some(inkwell::module::Linkage::External))
            });

            builder.build_call(memcpy_fn, &[
                dest_val.into(),
                src_val.into(),
                size_val.into(),
                context.bool_type().const_int(0, false).into() // isvolatile = false
            ], "memcpy_call").unwrap();
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
            let offset_val = operand_to_llvm(context, builder, ssa_storage, offset).into_int_value();

            // GEPI (GetElementPtr) for i8*
            let offset_ptr = unsafe { builder.build_in_bounds_gep(ptr_val, &[offset_val], "offset_ptr").unwrap() };

            let ptr_ssa = get_or_create_ssa(context, builder, ssa_storage, *dest, offset_ptr.get_type().as_basic_type_enum());
            builder.build_store(ptr_ssa, offset_ptr).unwrap();
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
                    builder.build_int_truncate(val.into_int_value(), target_type, "store_trunc").unwrap().into()
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
        if let MirInstruction::Index { dest, subject, index } = inst {
            let val = operand_to_llvm(context, builder, ssa_storage, subject);
            if let BasicValueEnum::StructValue(s) = val {
                let elem = builder.build_extract_value(s, *index as u32, "index_tmp").unwrap();
                let ptr = get_or_create_ssa(context, builder, ssa_storage, *dest, elem.get_type());
                builder.build_store(ptr, elem).unwrap();
            } else if let BasicValueEnum::PointerValue(p) = val {
                // Heuristic: if it's a pointer and we're indexing it, it's likely a load
                // For now, assume it's a load of i64 (char code) if index is 0.
                // In a more robust implementation, we'd use the SSA type.
                let elem_type = if let Some(typ) = builder.get_insert_block().and_then(|bb| bb.get_parent()).and_then(|f| {
                    // This is hard to get from here. Let's use i64 as default for Onu chars.
                    Some(context.i64_type().as_basic_type_enum())
                }) { typ } else { context.i64_type().as_basic_type_enum() };

                // GEPI if index > 0, then load
                let target_ptr = if *index > 0 {
                    unsafe { builder.build_in_bounds_gep(p, &[context.i64_type().const_int(*index as u64, false)], "idx_ptr").unwrap() }
                } else {
                    p
                };
                
                let elem = builder.build_load(target_ptr, "index_load").unwrap();
                // Special case for byte load: extend to i64
                let final_elem = if elem.get_type().is_int_type() && elem.into_int_value().get_type().get_bit_width() == 8 {
                    builder.build_int_z_extend(elem.into_int_value(), context.i64_type(), "char_ext").unwrap().into()
                } else {
                    elem
                };

                let ptr = get_or_create_ssa(context, builder, ssa_storage, *dest, final_elem.get_type());
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
            MirLiteral::Boolean(b) => context.bool_type().const_int(if *b { 1 } else { 0 }, false).into(),
            MirLiteral::Text(s) => {
                let length = context.i64_type().const_int(s.len() as u64, false);
                let global_str = builder.build_global_string_ptr(s, "strtmp").unwrap();
                let i64_type = context.i64_type();
                let i8_ptr_type = context.i8_type().ptr_type(inkwell::AddressSpace::default());
                let bool_type = context.bool_type();
                let is_dynamic = bool_type.const_int(0, false); // Literal strings are not dynamic

                let string_struct_type = context.struct_type(&[i64_type.into(), i8_ptr_type.into(), bool_type.into()], false);
                
                // Achievement: Use const_named_struct to ensure this is a compile-time constant
                let string_val = string_struct_type.const_named_struct(&[
                    length.into(),
                    global_str.as_pointer_value().into(),
                    is_dynamic.into(),
                ]);
                
                string_val.into()
            }
            MirLiteral::Nothing => context.i64_type().const_int(0, false).into(),
        },
        MirOperand::Variable(id, _) => {
            let ptr = ssa_storage.get(id).expect(&format!("SSA variable {} not found", id));
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
