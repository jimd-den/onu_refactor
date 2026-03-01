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
use inkwell::values::{PointerValue, BasicValueEnum};
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
        _module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        _registry: &RegistryService,
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
                    let cond = builder.build_int_compare(pred, l_val.into_int_value(), r_val.into_int_value(), "cmptmp").unwrap();
                    builder.build_int_z_extend(cond, context.i64_type(), "booltmp").unwrap().into()
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
        _registry: &RegistryService,
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
                    .map(|t| crate::adapters::codegen::OnuCodegen::onu_type_to_llvm_static(context, t).unwrap_or(context.i64_type().as_basic_type_enum()).into())
                    .collect();

                let ret_type_opt = crate::adapters::codegen::OnuCodegen::onu_type_to_llvm_static(context, return_type);
                
                if let Some(ret_type) = ret_type_opt {
                    let fn_type = ret_type.fn_type(&llvm_arg_types, false);
                    module.add_function(&llvm_name, fn_type, Some(inkwell::module::Linkage::External))
                } else {
                    let fn_type = context.void_type().fn_type(&llvm_arg_types, false);
                    module.add_function(&llvm_name, fn_type, Some(inkwell::module::Linkage::External))
                }
            };
            
            let call = builder.build_call(func, &llvm_args, "calltmp").unwrap();
            
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
        _registry: &RegistryService,
        ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
        inst: &MirInstruction,
    ) -> Result<(), OnuError> {
        if let MirInstruction::Emit(op) = inst {
            let val = operand_to_llvm(context, builder, ssa_storage, op);
            
            if val.is_int_value() {
                // If it's an integer, we use printf to print it
                let printf_fn = module.get_function("printf").expect("printf not pre-declared");
                let fmt_str = builder.build_global_string_ptr("%lld\n", "fmt").unwrap();
                builder.build_call(printf_fn, &[fmt_str.as_pointer_value().into(), val.into()], "printf_emit").unwrap();
            } else {
                let arg = if val.is_struct_value() {
                    builder.build_extract_value(val.into_struct_value(), 1, "raw_ptr").unwrap()
                } else {
                    val
                };

                let puts_fn = module.get_function("puts").expect("puts not pre-declared");
                builder.build_call(puts_fn, &[arg.into()], "emit").unwrap();
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
        _registry: &RegistryService,
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
        context: &'ctx Context,
        module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        _registry: &RegistryService,
        ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
        inst: &MirInstruction,
    ) -> Result<(), OnuError> {
        if let MirInstruction::Drop { ssa_var, typ, name, is_dynamic } = inst {
            // ZERO-COST ACHIEVEMENT: If statically known to be non-dynamic at lowering, emit nothing.
            if !is_dynamic {
                return Ok(());
            }

            if typ.is_resource() {
                if let Some(ptr) = ssa_storage.get(ssa_var) {
                    let val = builder.build_load(*ptr, "load_for_drop").unwrap();
                    if let BasicValueEnum::StructValue(s) = val {
                        if typ == &crate::domain::entities::types::OnuType::Strings {
                            let str_ptr = builder.build_extract_value(s, 1, "str_ptr_for_drop").unwrap();
                            let is_dynamic_runtime = builder.build_extract_value(s, 2, "is_dynamic_flag").unwrap().into_int_value();

                            // Check if dynamically allocated before freeing
                            let free_bb = context.append_basic_block(builder.get_insert_block().unwrap().get_parent().unwrap(), "free_bb");
                            let cont_bb = context.append_basic_block(builder.get_insert_block().unwrap().get_parent().unwrap(), "cont_bb");

                            let is_true = builder.build_int_compare(inkwell::IntPredicate::NE, is_dynamic_runtime, context.bool_type().const_int(0, false), "is_dynamic_cmp").unwrap();
                            builder.build_conditional_branch(is_true, free_bb, cont_bb).unwrap();

                            builder.position_at_end(free_bb);
                            
                            // Declare free if it doesn't exist
                            let free_fn = if let Some(f) = module.get_function("free") {
                                f
                            } else {
                                let void_type = context.void_type();
                                let i8_ptr_type = context.i8_type().ptr_type(inkwell::AddressSpace::default());
                                let free_type = void_type.fn_type(&[i8_ptr_type.into()], false);
                                module.add_function("free", free_type, Some(inkwell::module::Linkage::External))
                            };

                            builder.build_call(free_fn, &[str_ptr.into()], "free_call").unwrap();

                            // Prevent double free by zeroing out the flag
                            let false_val = context.bool_type().const_int(0, false);
                            let new_s = builder.build_insert_value(s, false_val, 2, "zero_flag").unwrap().into_struct_value();
                            builder.build_store(*ptr, new_s).unwrap();

                            builder.build_unconditional_branch(cont_bb).unwrap();

                            builder.position_at_end(cont_bb);
                        }
                    }
                }
            }
        }
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
        _registry: &RegistryService,
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
            let size_val = operand_to_llvm(context, builder, ssa_storage, size_bytes);
            let malloc_fn = module.get_function("malloc").expect("malloc not pre-declared");

            let call_val = builder.build_call(malloc_fn, &[size_val.into()], "malloc_call").unwrap();
            match call_val.try_as_basic_value() {
                inkwell::values::ValueKind::Basic(res) => {
                    let ptr = get_or_create_ssa(context, builder, ssa_storage, *dest, res.get_type());
                    builder.build_store(ptr, res).unwrap();
                }
                _ => panic!("malloc call should return a basic value"),
            }
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
        _registry: &RegistryService,
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
        _registry: &RegistryService,
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

pub struct IndexStrategy;
impl<'ctx> InstructionStrategy<'ctx> for IndexStrategy {
    fn generate(
        &self,
        context: &'ctx Context,
        _module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        _registry: &RegistryService,
        ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
        inst: &MirInstruction,
    ) -> Result<(), OnuError> {
        if let MirInstruction::Index { dest, subject, index } = inst {
            let val = operand_to_llvm(context, builder, ssa_storage, subject);
            if let BasicValueEnum::StructValue(s) = val {
                let elem = builder.build_extract_value(s, *index as u32, "index_tmp").unwrap();
                let ptr = get_or_create_ssa(context, builder, ssa_storage, *dest, elem.get_type());
                builder.build_store(ptr, elem).unwrap();
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
