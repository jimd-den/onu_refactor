/// Ọ̀nụ LLVM Codegen Adapter: Infrastructure/Interface Implementation
///
/// This implements the CodegenPort using the Inkwell library
/// to translate MIR into LLVM Bitcode.
pub mod strategies;
pub mod typemapper;

use crate::adapters::codegen::strategies::*;
use crate::adapters::codegen::typemapper::LlvmTypeMapper;
use crate::application::ports::compiler_ports::CodegenPort;
use crate::application::use_cases::registry_service::RegistryService;
use crate::domain::entities::error::OnuError;
use crate::domain::entities::mir::*;
use crate::domain::entities::types::OnuType;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::PointerValue;
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

        // 1. Declare Global Arena (1 MB for now)
        let arena_size = 1024 * 1024;
        let arena_type = context.i8_type().array_type(arena_size as u32);
        let arena = module.add_global(arena_type, None, "onu_arena");
        arena.set_linkage(Linkage::Internal);
        arena.set_initializer(&arena_type.const_zero());

        // 2. Declare Global Arena Pointer
        let i8ptr_type = context.i8_type().ptr_type(inkwell::AddressSpace::default());
        let arena_ptr = module.add_global(i8ptr_type, None, "onu_arena_ptr");
        arena_ptr.set_linkage(Linkage::Internal);
        arena_ptr.set_initializer(&arena.as_pointer_value().const_cast(i8ptr_type));

        let mut generator = LlvmGenerator {
            context: &context,
            module,
            builder,
            registry: self
                .registry
                .as_ref()
                .expect("Registry not provided to codegen"),
            ssa_storage: HashMap::new(),
            blocks: HashMap::new(),
        };

        generator.generate(program)?;

        use inkwell::module::Module;
        use inkwell::passes::{PassManager, PassManagerBuilder};
        use inkwell::values::FunctionValue;

        let pass_manager_builder = PassManagerBuilder::create();
        pass_manager_builder.set_optimization_level(inkwell::OptimizationLevel::Aggressive);

        let fpm: PassManager<FunctionValue> = PassManager::create(&generator.module);
        pass_manager_builder.populate_function_pass_manager(&fpm);

        let mpm: PassManager<Module> = PassManager::create(());
        pass_manager_builder.populate_module_pass_manager(&mpm);

        fpm.initialize();
        for func in generator.module.get_functions() {
            fpm.run_on(&func);
        }
        fpm.finalize();

        // Run the AlwaysInliner as a separate dedicated pass BEFORE the full MPM.
        // Rationale: the legacy PassManagerBuilder's populate_module_pass_manager
        // includes an inliner, but its cost model can override `alwaysinline` when
        // calling conventions or instruction counts trigger heuristics. Running
        // add_always_inliner_pass explicitly guarantees all alwaysinline sites are
        // expanded unconditionally, regardless of cost.
        let always_inliner: PassManager<Module> = PassManager::create(());
        always_inliner.add_always_inliner_pass();
        always_inliner.run_on(&generator.module);

        mpm.run_on(&generator.module);

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
        use crate::application::use_cases::codegen_profile::{
            CallingConvention, FunctionLinkage, OptimizerHint, derive_profile,
        };
        use inkwell::attributes::{Attribute, AttributeLoc};

        // ── Step 1: Ask the Application layer what this function's profile is.
        // All optimization policy lives in `derive_profile` — zero business logic here.
        let profile = derive_profile(func);

        // ── Step 2: Map linkage enum → LLVM linkage and function name.
        let is_main = func.name == "run" || func.name == "main";
        let llvm_name = if is_main {
            "main".to_string()
        } else {
            func.name.clone()
        };

        let llvm_linkage = match profile.linkage {
            FunctionLinkage::Public => Linkage::External,
            FunctionLinkage::Internal => Linkage::Internal,
        };

        // ── Step 3: Build the LLVM function type and declare it in the module.
        let arg_types: Vec<inkwell::types::BasicMetadataTypeEnum> = func
            .args
            .iter()
            .map(|arg| {
                LlvmTypeMapper::onu_to_llvm(self.context, &arg.typ, self.registry)
                    .unwrap_or(self.context.i64_type().as_basic_type_enum())
                    .into()
            })
            .collect();

        let fn_val = if is_main {
            let fn_type = self.context.i32_type().fn_type(&arg_types, false);
            self.module
                .add_function(&llvm_name, fn_type, Some(llvm_linkage))
        } else if let Some(ret_type) =
            LlvmTypeMapper::onu_to_llvm(self.context, &func.return_type, self.registry)
        {
            let fn_type = ret_type.fn_type(&arg_types, false);
            self.module
                .add_function(&llvm_name, fn_type, Some(llvm_linkage))
        } else {
            let fn_type = self.context.void_type().fn_type(&arg_types, false);
            self.module
                .add_function(&llvm_name, fn_type, Some(llvm_linkage))
        };

        // ── Step 4: Apply calling convention from profile.
        match profile.calling_convention {
            CallingConvention::Fast => {
                fn_val.set_call_conventions(8); // LLVM fastcc numeric id
                // local_unnamed_addr: the symbol's address is not significant within
                // this file, enabling link-time cloning and further optimisations.
                fn_val
                    .as_global_value()
                    .set_unnamed_address(inkwell::values::UnnamedAddress::Local);
            }
            CallingConvention::CDefault => {} // C ABI is the default — no action needed.
        }

        // ── Step 5: Translate each OptimizerHint to its LLVM attribute name.
        // Pure mechanical mapping — adding a new hint only requires a new arm here.
        for hint in &profile.optimizer_hints {
            let attr_name = match hint {
                OptimizerHint::ReadNone => "readnone",
                OptimizerHint::NoUnwind => "nounwind",
                OptimizerHint::NoFree => "nofree",
                OptimizerHint::NoSync => "nosync",
                OptimizerHint::AlwaysInline => "alwaysinline",
            };
            let kind_id = Attribute::get_named_enum_kind_id(attr_name);
            fn_val.add_attribute(
                AttributeLoc::Function,
                self.context.create_enum_attribute(kind_id, 0),
            );
        }
    }

    fn generate_function(&mut self, func: &MirFunction) -> Result<(), OnuError> {
        let llvm_name = if func.name == "run" || func.name == "main" {
            "main".to_string()
        } else {
            func.name.clone()
        };
        let function = self.module.get_function(&llvm_name).unwrap();
        self.ssa_storage.clear();
        self.blocks.clear();

        for block in &func.blocks {
            let llvm_block = self
                .context
                .append_basic_block(function, &format!("bb{}", block.id));
            self.blocks.insert(block.id, llvm_block);
        }

        if let Some(first_block) = func.blocks.first() {
            let entry_bb = self.blocks.get(&first_block.id).unwrap();
            self.builder.position_at_end(*entry_bb);

            for (i, arg) in function.get_param_iter().enumerate() {
                let mir_arg = &func.args[i];
                let ptr = self
                    .builder
                    .build_alloca(arg.get_type(), &mir_arg.name)
                    .unwrap();
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
        eprintln!("[DEBUG] Generating LLVM for instruction: {:?}", inst);
        match inst {
            MirInstruction::BinaryOperation { .. } => BinaryOpStrategy.generate(
                self.context,
                &self.module,
                &self.builder,
                self.registry,
                &mut self.ssa_storage,
                inst,
            ),
            MirInstruction::Call { .. } => CallStrategy.generate(
                self.context,
                &self.module,
                &self.builder,
                self.registry,
                &mut self.ssa_storage,
                inst,
            ),
            MirInstruction::Emit { .. } => EmitStrategy.generate(
                self.context,
                &self.module,
                &self.builder,
                self.registry,
                &mut self.ssa_storage,
                inst,
            ),
            MirInstruction::Assign { .. } => AssignStrategy.generate(
                self.context,
                &self.module,
                &self.builder,
                self.registry,
                &mut self.ssa_storage,
                inst,
            ),
            MirInstruction::Drop { .. } => DropStrategy.generate(
                self.context,
                &self.module,
                &self.builder,
                self.registry,
                &mut self.ssa_storage,
                inst,
            ),
            MirInstruction::Index { .. } => IndexStrategy.generate(
                self.context,
                &self.module,
                &self.builder,
                self.registry,
                &mut self.ssa_storage,
                inst,
            ),
            MirInstruction::Tuple { .. } => TupleStrategy.generate(
                self.context,
                &self.module,
                &self.builder,
                self.registry,
                &mut self.ssa_storage,
                inst,
            ),
            MirInstruction::Alloc { .. } => AllocStrategy.generate(
                self.context,
                &self.module,
                &self.builder,
                self.registry,
                &mut self.ssa_storage,
                inst,
            ),
            MirInstruction::MemCopy { .. } => MemCopyStrategy.generate(
                self.context,
                &self.module,
                &self.builder,
                self.registry,
                &mut self.ssa_storage,
                inst,
            ),
            MirInstruction::PointerOffset { .. } => PointerOffsetStrategy.generate(
                self.context,
                &self.module,
                &self.builder,
                self.registry,
                &mut self.ssa_storage,
                inst,
            ),
            MirInstruction::Load { .. } => LoadStrategy.generate(
                self.context,
                &self.module,
                &self.builder,
                self.registry,
                &mut self.ssa_storage,
                inst,
            ),
            MirInstruction::Store { .. } => StoreStrategy.generate(
                self.context,
                &self.module,
                &self.builder,
                self.registry,
                &mut self.ssa_storage,
                inst,
            ),
            MirInstruction::TypedStore { .. } => TypedStoreStrategy.generate(
                self.context,
                &self.module,
                &self.builder,
                self.registry,
                &mut self.ssa_storage,
                inst,
            ),
        }
    }

    fn generate_terminator(&mut self, term: &MirTerminator) -> Result<(), OnuError> {
        let current_bb = self.builder.get_insert_block().unwrap();
        let function = current_bb.get_parent().unwrap();
        let is_main = function.get_name().to_str().unwrap() == "main";
        let is_void = function.get_type().get_return_type().is_none();

        match term {
            MirTerminator::Return(op) => {
                let val = strategies::operand_to_llvm(
                    self.context,
                    &self.builder,
                    &mut self.ssa_storage,
                    op,
                );
                if is_main {
                    // Truncate to i32 for main return code
                    let i32_val = if val.get_type().is_int_type()
                        && val.into_int_value().get_type().get_bit_width() > 32
                    {
                        self.builder
                            .build_int_truncate(
                                val.into_int_value(),
                                self.context.i32_type(),
                                "main_ret",
                            )
                            .unwrap()
                    } else {
                        val.into_int_value()
                    };
                    self.builder.build_return(Some(&i32_val)).unwrap();
                } else if is_void {
                    self.builder.build_return(None).unwrap();
                } else {
                    self.builder.build_return(Some(&val)).unwrap();
                }
            }
            MirTerminator::Branch(id) => {
                if let Some(target) = self.blocks.get(id) {
                    self.builder.build_unconditional_branch(*target).unwrap();
                }
            }
            MirTerminator::CondBranch {
                condition,
                then_block,
                else_block,
            } => {
                let cond_val = strategies::operand_to_llvm(
                    self.context,
                    &self.builder,
                    &mut self.ssa_storage,
                    condition,
                );
                let cond = if cond_val.get_type() == self.context.bool_type().as_basic_type_enum() {
                    cond_val.into_int_value()
                } else {
                    self.builder
                        .build_int_compare(
                            inkwell::IntPredicate::NE,
                            cond_val.into_int_value(),
                            self.context.i64_type().const_int(0, false),
                            "bool_cast",
                        )
                        .unwrap()
                };
                let then_bb = self.blocks.get(then_block).unwrap();
                let else_bb = self.blocks.get(else_block).unwrap();
                self.builder
                    .build_conditional_branch(cond, *then_bb, *else_bb)
                    .unwrap();
            }
            MirTerminator::Unreachable => {
                self.builder.build_unreachable().unwrap();
            }
        }
        Ok(())
    }
}
