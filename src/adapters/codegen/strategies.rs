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
                MirBinOp::Eq | MirBinOp::Gt | MirBinOp::Lt => {
                    let pred = match op {
                        MirBinOp::Eq => inkwell::IntPredicate::EQ,
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
            
            let str_val = if val.is_int_value() {
                let as_text_fn = module.get_function("as-text").expect("as-text not pre-declared");
                let call_val = builder.build_call(as_text_fn, &[val.into()], "as_text_tmp").unwrap();
                match call_val.try_as_basic_value() {
                    inkwell::values::ValueKind::Basic(v) => v,
                    _ => panic!("as-text call should return a basic value"),
                }
            } else {
                val
            };

            let arg = if str_val.is_struct_value() {
                builder.build_extract_value(str_val.into_struct_value(), 1, "raw_ptr").unwrap()
            } else {
                str_val
            };

            let broadcasts_fn = module.get_function("broadcasts").expect("broadcasts not pre-declared");
            builder.build_call(broadcasts_fn, &[arg.into()], "emit").unwrap();
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
        _context: &'ctx Context,
        _module: &Module<'ctx>,
        _builder: &Builder<'ctx>,
        _registry: &RegistryService,
        _ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
        _inst: &MirInstruction,
    ) -> Result<(), OnuError> {
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
                let string_struct_type = context.struct_type(&[i64_type.into(), i8_ptr_type.into()], false);
                let mut string_val = string_struct_type.get_undef();
                string_val = builder.build_insert_value(string_val, length, 0, "set_len").unwrap().into_struct_value();
                string_val = builder.build_insert_value(string_val, global_str, 1, "set_ptr").unwrap().into_struct_value();
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
