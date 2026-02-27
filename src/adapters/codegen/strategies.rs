/// Codegen Strategies: Interface Adapter Layer
///
/// This module implements the Strategy Pattern for MIR Instruction generation.
/// Each strategy is responsible for translating a specific MIR instruction 
/// into the corresponding LLVM IR.

use crate::domain::entities::mir::{MirInstruction, MirBinOp, MirOperand, MirLiteral};
use crate::domain::entities::error::OnuError;
use crate::domain::entities::types::OnuType;
use crate::application::use_cases::registry_service::RegistryService;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::{BasicValueEnum, PointerValue, BasicValue};
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

// --- Specific Strategies ---

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
            let l = operand_to_llvm(context, builder, ssa_storage, lhs).into_int_value();
            let r = operand_to_llvm(context, builder, ssa_storage, rhs).into_int_value();
            
            let res: BasicValueEnum = match op {
                MirBinOp::Add => builder.build_int_add(l, r, "addtmp").unwrap().into(),
                MirBinOp::Sub => builder.build_int_sub(l, r, "subtmp").unwrap().into(),
                MirBinOp::Mul => builder.build_int_mul(l, r, "multmp").unwrap().into(),
                MirBinOp::Div => builder.build_int_signed_div(l, r, "divtmp").unwrap().into(),
                MirBinOp::Eq => builder.build_int_compare(inkwell::IntPredicate::EQ, l, r, "eqtmp").unwrap().into(),
                MirBinOp::Gt => builder.build_int_compare(inkwell::IntPredicate::SGT, l, r, "gttmp").unwrap().into(),
                MirBinOp::Lt => builder.build_int_compare(inkwell::IntPredicate::SLT, l, r, "lttmp").unwrap().into(),
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
        if let MirInstruction::Call { dest, name, args } = inst {
            let c_name = name.replace('-', "_");
            
            // Determine arity from registry
            let arity = registry.get_signature(name).map(|s| s.input_types.len()).unwrap_or(args.len());

            let function = if let Some(f) = module.get_function(&c_name) {
                f
            } else {
                let i64_type = context.i64_type();
                let mut arg_types = Vec::new();
                for _ in 0..arity { arg_types.push(i64_type.as_basic_type_enum().into()); }
                let fn_type = i64_type.fn_type(&arg_types, false);
                module.add_function(&c_name, fn_type, Some(inkwell::module::Linkage::External))
            };

            // Only pass the required number of arguments
            let llvm_args: Vec<inkwell::values::BasicMetadataValueEnum> = args.iter().take(arity)
                .map(|arg| {
                    let mut val = operand_to_llvm(context, builder, ssa_storage, arg);
                    // If passing a string struct to an i64 (as-text style), we might need to extract?
                    // Original code passed {i64, i8*} as two i64s if bitcasted, but let's keep it simple.
                    val.into()
                })
                .collect();

            let call = builder.build_call(function, &llvm_args, "calltmp").unwrap();
            let res_kind = call.try_as_basic_value();
            if let inkwell::values::ValueKind::Basic(res) = res_kind {
                let ptr = get_or_create_ssa(context, builder, ssa_storage, *dest, res.get_type());
                builder.build_store(ptr, res).unwrap();
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
            
            let i8_ptr = context.i8_type().ptr_type(inkwell::AddressSpace::default());
            let void_type = context.void_type();
            let broadcast_fn = if let Some(f) = module.get_function("onu_broadcast") {
                f
            } else {
                let fn_type = void_type.fn_type(&[i8_ptr.into()], false);
                module.add_function("onu_broadcast", fn_type, Some(inkwell::module::Linkage::External))
            };

            let arg = if val.is_struct_value() {
                // To pass by pointer, we need the address of the SSA variable
                if let MirOperand::Variable(id, _) = op {
                    ssa_storage.get(id).unwrap().as_basic_value_enum()
                } else {
                    // Constant text: create a temp alloca
                    let ptr = builder.build_alloca(val.get_type(), "emit_tmp").unwrap();
                    builder.build_store(ptr, val).unwrap();
                    ptr.into()
                }
            } else {
                // For non-struct (like int as-text result), create a fake struct or pass as is?
                // Runtime heuristic expects a pointer to a struct.
                let i64_type = context.i64_type();
                let string_struct_type = context.struct_type(&[i64_type.into(), i8_ptr.into()], false);
                let mut fake_str = string_struct_type.get_undef();
                fake_str = builder.build_insert_value(fake_str, i64_type.const_int(0, false), 0, "len").unwrap().into_struct_value();
                fake_str = builder.build_insert_value(fake_str, builder.build_int_to_ptr(val.into_int_value(), i8_ptr, "ptr").unwrap(), 1, "data").unwrap().into_struct_value();
                
                let ptr = builder.build_alloca(string_struct_type, "fake_emit_tmp").unwrap();
                builder.build_store(ptr, fake_str).unwrap();
                ptr.into()
            };

            builder.build_call(broadcast_fn, &[arg.into()], "emit").unwrap();
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
        inst: &MirInstruction,
    ) -> Result<(), OnuError> {
        if let MirInstruction::Drop { .. } = inst {
            // Drop is a hint for memory management.
            // In the current LLVM backend, we rely on automatic stack allocation
            // or explicit runtime calls if we had a GC. For now, it's a no-op.
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
    ssa_storage: &mut HashMap<usize, PointerValue<'ctx>>,
    op: &MirOperand,
) -> BasicValueEnum<'ctx> {
    match op {
        MirOperand::Constant(lit) => match lit {
            MirLiteral::I64(n) => context.i64_type().const_int(*n as u64, true).into(),
            MirLiteral::F64(n) => context.f64_type().const_float(*n as f64).into(),
            MirLiteral::Boolean(b) => context.bool_type().const_int(*b as u64, false).into(),
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
            if let Some(ptr) = ssa_storage.get(id) {
                builder.build_load(*ptr, &format!("v{}", id)).unwrap()
            } else {
                let i64_type = context.i64_type().as_basic_type_enum();
                let ptr = get_or_create_ssa(context, builder, ssa_storage, *id, i64_type);
                builder.build_load(ptr, &format!("v{}", id)).unwrap()
            }
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
