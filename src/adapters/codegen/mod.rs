/// Ọ̀nụ LLVM Codegen Adapter: Infrastructure/Interface Implementation
///
/// This implements the CodegenPort using the Inkwell library
/// to translate MIR into LLVM Bitcode.

pub mod strategies;
pub mod typemapper;

use crate::application::ports::compiler_ports::CodegenPort;
use crate::application::use_cases::registry_service::RegistryService;
use crate::domain::entities::error::OnuError;
use crate::domain::entities::mir::*;
use crate::domain::entities::types::OnuType;
use crate::adapters::codegen::strategies::*;
use crate::adapters::codegen::typemapper::LlvmTypeMapper;
use inkwell::context::Context;
use inkwell::builder::Builder;
use inkwell::module::{Module, Linkage};
use inkwell::values::PointerValue;
use inkwell::types::{BasicTypeEnum, BasicType};
use std::collections::HashMap;

pub struct OnuCodegen {
    pub registry: Option<RegistryService>,
}

impl OnuCodegen {
    pub fn new() -> Self {
        Self { registry: None }
    }
}

impl CodegenPort for OnuCodegen {
    fn generate(&self, program: &MirProgram) -> Result<String, OnuError> {
        let context = Context::create();
        let module = context.create_module("onu_discourse");
        let builder = context.create_builder();
        
        let mut generator = LlvmGenerator {
            context: &context,
            module,
            builder,
            registry: self.registry.as_ref().expect("Registry not provided to codegen"),
            ssa_storage: HashMap::new(),
            blocks: HashMap::new(),
        };

        generator.generate(program)?;

        use inkwell::passes::PassManager;
        let fpm: PassManager<Module> = PassManager::create(());
        fpm.add_promote_memory_to_register_pass();
        fpm.add_instruction_combining_pass();
        fpm.add_reassociate_pass();
        fpm.add_gvn_pass();
        fpm.add_cfg_simplification_pass();
        fpm.run_on(&generator.module);

        Ok(generator.module.print_to_string().to_string())
    }

    fn set_registry(&mut self, registry: RegistryService) {
        self.registry = Some(registry);
    }
}

struct LlvmGenerator<'ctx, 'a> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    registry: &'a RegistryService,
    ssa_storage: HashMap<usize, PointerValue<'ctx>>,
    blocks: HashMap<usize, inkwell::basic_block::BasicBlock<'ctx>>,
}

impl<'ctx, 'a> LlvmGenerator<'ctx, 'a> {
    fn generate(&mut self, program: &MirProgram) -> Result<(), OnuError> {
        for func in &program.functions {
            self.declare_function(func);
        }
        for func in &program.functions {
            self.generate_function(func)?;
        }
        Ok(())
    }

    fn declare_function(&self, func: &MirFunction) {
        let arg_types: Vec<inkwell::types::BasicMetadataTypeEnum> = func.args.iter()
            .map(|arg| LlvmTypeMapper::onu_to_llvm(self.context, &arg.typ).unwrap_or(self.context.i64_type().as_basic_type_enum()).into())
            .collect();
        
        let is_main = func.name == "run" || func.name == "main";
        let llvm_name = if is_main { "main".to_string() } else { func.name.clone() };
        
        let fn_val = if is_main {
            let fn_type = self.context.i32_type().fn_type(&arg_types, false);
            self.module.add_function(&llvm_name, fn_type, Some(Linkage::External))
        } else if let Some(ret_type) = LlvmTypeMapper::onu_to_llvm(self.context, &func.return_type) {
            let fn_type = ret_type.fn_type(&arg_types, false);
            self.module.add_function(&llvm_name, fn_type, Some(Linkage::External))
        } else {
            let fn_type = self.context.void_type().fn_type(&arg_types, false);
            self.module.add_function(&llvm_name, fn_type, Some(Linkage::External))
        };

        if !is_main {
            fn_val.set_call_conventions(inkwell::module::Linkage::External as u32);
            // wait, CallConventions isn't in inkwell::values. It might not exist in inkwell 0.8
            // Let's use a raw u32 for fastcc, which is 8 in LLVM
            fn_val.set_call_conventions(8);
        }
    }

    fn generate_function(&mut self, func: &MirFunction) -> Result<(), OnuError> {
        let llvm_name = if func.name == "run" || func.name == "main" { "main".to_string() } else { func.name.clone() };
        let function = self.module.get_function(&llvm_name).unwrap();
        self.ssa_storage.clear();
        self.blocks.clear();

        for block in &func.blocks {
            let llvm_block = self.context.append_basic_block(function, &format!("bb{}", block.id));
            self.blocks.insert(block.id, llvm_block);
        }

        if let Some(first_block) = func.blocks.first() {
            let entry_bb = self.blocks.get(&first_block.id).unwrap();
            self.builder.position_at_end(*entry_bb);

            for (i, arg) in function.get_param_iter().enumerate() {
                let mir_arg = &func.args[i];
                let ptr = self.builder.build_alloca(arg.get_type(), &mir_arg.name).unwrap();
                self.builder.build_store(ptr, arg).unwrap();
                self.ssa_storage.insert(mir_arg.ssa_var, ptr);
            }
        }

        for block in &func.blocks {
            let llvm_block = self.blocks.get(&block.id).unwrap();
            self.builder.position_at_end(*llvm_block);
            for inst in &block.instructions {
                self.generate_instruction(inst)?;
            }
            self.generate_terminator(&block.terminator)?;
        }
        Ok(())
    }

    fn generate_instruction(&mut self, inst: &MirInstruction) -> Result<(), OnuError> {
        match inst {
            MirInstruction::BinaryOperation { .. } => {
                BinaryOpStrategy.generate(self.context, &self.module, &self.builder, self.registry, &mut self.ssa_storage, inst)
            }
            MirInstruction::Call { .. } => {
                CallStrategy.generate(self.context, &self.module, &self.builder, self.registry, &mut self.ssa_storage, inst)
            }
            MirInstruction::Emit { .. } => {
                EmitStrategy.generate(self.context, &self.module, &self.builder, self.registry, &mut self.ssa_storage, inst)
            }
            MirInstruction::Assign { .. } => {
                AssignStrategy.generate(self.context, &self.module, &self.builder, self.registry, &mut self.ssa_storage, inst)
            }
            MirInstruction::Drop { .. } => {
                DropStrategy.generate(self.context, &self.module, &self.builder, self.registry, &mut self.ssa_storage, inst)
            }
            MirInstruction::Index { .. } => {
                IndexStrategy.generate(self.context, &self.module, &self.builder, self.registry, &mut self.ssa_storage, inst)
            }
            MirInstruction::Tuple { .. } => {
                TupleStrategy.generate(self.context, &self.module, &self.builder, self.registry, &mut self.ssa_storage, inst)
            }
            MirInstruction::Alloc { .. } => {
                AllocStrategy.generate(self.context, &self.module, &self.builder, self.registry, &mut self.ssa_storage, inst)
            }
            MirInstruction::MemCopy { .. } => {
                MemCopyStrategy.generate(self.context, &self.module, &self.builder, self.registry, &mut self.ssa_storage, inst)
            }
            MirInstruction::PointerOffset { .. } => {
                PointerOffsetStrategy.generate(self.context, &self.module, &self.builder, self.registry, &mut self.ssa_storage, inst)
            }
        }
    }

    fn generate_terminator(&mut self, term: &MirTerminator) -> Result<(), OnuError> {
        let current_bb = self.builder.get_insert_block().unwrap();
        let function = current_bb.get_parent().unwrap();
        let is_main = function.get_name().to_str().unwrap() == "main";
        let is_void = function.get_type().get_return_type().is_none();

        match term {
            MirTerminator::Return(op) => {
                if is_main {
                    self.builder.build_return(Some(&self.context.i32_type().const_int(0, false))).unwrap();
                } else if is_void {
                    self.builder.build_return(None).unwrap();
                } else {
                    let val = strategies::operand_to_llvm(self.context, &self.builder, &mut self.ssa_storage, op);
                    self.builder.build_return(Some(&val)).unwrap();
                }
            }
            MirTerminator::Branch(id) => {
                if let Some(target) = self.blocks.get(id) {
                    self.builder.build_unconditional_branch(*target).unwrap();
                }
            }
            MirTerminator::CondBranch { condition, then_block, else_block } => {
                let cond_val = strategies::operand_to_llvm(self.context, &self.builder, &mut self.ssa_storage, condition);
                let cond = if cond_val.get_type() == self.context.bool_type().as_basic_type_enum() {
                    cond_val.into_int_value()
                } else {
                    self.builder.build_int_compare(inkwell::IntPredicate::NE, cond_val.into_int_value(), self.context.i64_type().const_int(0, false), "bool_cast").unwrap()
                };
                let then_bb = self.blocks.get(then_block).unwrap();
                let else_bb = self.blocks.get(else_block).unwrap();
                self.builder.build_conditional_branch(cond, *then_bb, *else_bb).unwrap();
            }
            MirTerminator::Unreachable => {
                self.builder.build_unreachable().unwrap();
            }
        }
        Ok(())
    }

}
