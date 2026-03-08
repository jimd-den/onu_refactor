/// Ọ̀nụ LLVM Codegen Adapter: Infrastructure/Interface Implementation
///
/// This implements the CodegenPort using the Inkwell library
/// to translate MIR into LLVM Bitcode.
pub mod compat;
pub mod platform;
pub mod strategies;
pub mod typemapper;

use crate::adapters::codegen::strategies::*;
use crate::adapters::codegen::compat::{arena_ptr_initializer, onu_i8ptr};
use crate::adapters::codegen::typemapper::LlvmTypeMapper;
use crate::application::ports::compiler_ports::CodegenPort;
use crate::application::use_cases::registry_service::RegistryService;
use crate::domain::entities::error::OnuError;
use crate::domain::entities::mir::*;
use crate::domain::entities::ARENA_SIZE_BYTES;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::types::BasicType;
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

        // 1. Declare Global Arena — size is driven by ARENA_SIZE_BYTES from the domain
        // so that it stays in sync with CompoundMemoStrategy's CACHE_MEMORY_LIMIT.
        // At 16 MiB this gives a 1024×1024 cache window for 2-dim/I64 functions.
        let arena_size = ARENA_SIZE_BYTES;
        let arena_type = context.i8_type().array_type(arena_size as u32);
        let arena = module.add_global(arena_type, None, "onu_arena");
        arena.set_linkage(Linkage::Internal);
        arena.set_initializer(&arena_type.const_zero());

        // 2. Declare Global Arena Pointer
        let i8ptr_type = onu_i8ptr(&context);
        let arena_ptr = module.add_global(i8ptr_type, None, "onu_arena_ptr");
        arena_ptr.set_linkage(Linkage::Internal);
        arena_ptr.set_initializer(&arena_ptr_initializer(&context, arena.as_pointer_value()));

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
        // Before generating MIR functions, emit any wide-integer division helpers
        // that are referenced by the program.  These helpers implement the
        // binary long-division algorithm using operations (shifts, comparisons,
        // add/sub) that LLVM can always lower, bypassing the missing compiler-rt
        // entries for sdiv on types wider than i128.
        self.emit_wide_div_helpers(program);

        for func in &program.functions {
            self.declare_function(func);
        }
        for func in &program.functions {
            self.generate_function(func)?;
        }
        Ok(())
    }

    /// Scan the program for calls to `__onu_wide_div_N` helpers and emit their
    /// LLVM IR implementations if they have not yet been emitted.
    ///
    /// Each helper implements unsigned binary long-division (bit-by-bit restoring
    /// division) using only shift, OR, comparison and subtraction — all of which
    /// LLVM can lower at any bit-width without an external runtime library.
    ///
    /// Note: WideInt values used in Ọ̀nụ represent non-negative mathematical
    /// quantities (e.g. Fibonacci numbers), so unsigned division is correct here.
    fn emit_wide_div_helpers(&self, program: &MirProgram) {
        use std::collections::HashSet;
        let mut emitted: HashSet<u32> = HashSet::new();

        for func in &program.functions {
            for block in &func.blocks {
                for inst in &block.instructions {
                    if let crate::domain::entities::mir::MirInstruction::Call { name, .. } = inst {
                        if let Some(bits_str) = name.strip_prefix("__onu_wide_div_") {
                            if let Ok(bits) = bits_str.parse::<u32>() {
                                if emitted.insert(bits) {
                                    self.emit_wide_div_helper(bits);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Emit the LLVM IR for `__onu_wide_div_<bits>(dividend, divisor) -> quotient`.
    ///
    /// Algorithm: restoring binary long-division.
    ///
    ///   quotient  = 0
    ///   remainder = 0
    ///   for i = (bits-1) downto 0:
    ///     remainder = (remainder << 1) | ((dividend >> i) & 1)
    ///     if remainder >= divisor:
    ///       remainder -= divisor
    ///       quotient |= (1 << i)
    ///   return quotient
    ///
    /// All constituent operations (shl, lshr, or, icmp uge, sub, zext) are
    /// fully supported by LLVM for any integer width.
    fn emit_wide_div_helper(&self, bits: u32) {
        use inkwell::attributes::{Attribute, AttributeLoc};
        use inkwell::module::Linkage;
        use inkwell::IntPredicate;

        let helper_name = format!("__onu_wide_div_{}", bits);

        // Only emit once.
        if self.module.get_function(&helper_name).is_some() {
            return;
        }

        let wide_type = self.context.custom_width_int_type(bits);
        let i64_type = self.context.i64_type();

        // declare internal fastcc iN @__onu_wide_div_N(iN %dividend, iN %divisor)
        let fn_type = wide_type.fn_type(&[wide_type.into(), wide_type.into()], false);
        let fn_val = self
            .module
            .add_function(&helper_name, fn_type, Some(Linkage::Internal));
        fn_val.set_call_conventions(8); // fastcc

        // nounwind + readnone + nofree attributes — the function has no side effects.
        for attr_name in &["nounwind", "readnone", "nofree", "nosync"] {
            let kind_id = Attribute::get_named_enum_kind_id(attr_name);
            fn_val.add_attribute(
                AttributeLoc::Function,
                self.context.create_enum_attribute(kind_id, 0),
            );
        }

        let dividend = fn_val.get_nth_param(0).unwrap().into_int_value();
        let divisor = fn_val.get_nth_param(1).unwrap().into_int_value();

        let zero_wide = wide_type.const_zero();
        let one_wide = wide_type.const_int(1, false);
        let bits_minus_1 = i64_type.const_int((bits - 1) as u64, false);

        // Basic blocks
        let entry_bb = self.context.append_basic_block(fn_val, "entry");
        let div_zero_bb = self.context.append_basic_block(fn_val, "div_zero");
        let loop_init_bb = self.context.append_basic_block(fn_val, "loop_init");
        let loop_check_bb = self.context.append_basic_block(fn_val, "loop_check");
        let loop_body_bb = self.context.append_basic_block(fn_val, "loop_body");
        let do_subtract_bb = self.context.append_basic_block(fn_val, "do_subtract");
        let loop_merge_bb = self.context.append_basic_block(fn_val, "loop_merge");
        let exit_bb = self.context.append_basic_block(fn_val, "exit");

        let b = &self.builder;

        // ── entry: handle divide-by-zero (return 0) ──────────────────────────
        b.position_at_end(entry_bb);
        let is_zero = b
            .build_int_compare(IntPredicate::EQ, divisor, zero_wide, "is_zero")
            .unwrap();
        b.build_conditional_branch(is_zero, div_zero_bb, loop_init_bb)
            .unwrap();

        // ── div_zero: return 0 ────────────────────────────────────────────────
        b.position_at_end(div_zero_bb);
        b.build_return(Some(&zero_wide)).unwrap();

        // ── loop_init: branch into loop header ───────────────────────────────
        b.position_at_end(loop_init_bb);
        b.build_unconditional_branch(loop_check_bb).unwrap();

        // ── loop_check: phi nodes + termination test ─────────────────────────
        b.position_at_end(loop_check_bb);
        let i_phi = b.build_phi(i64_type, "i").unwrap();
        let quotient_phi = b.build_phi(wide_type, "quotient").unwrap();
        let remainder_phi = b.build_phi(wide_type, "remainder").unwrap();

        // Initialize phis from loop_init
        i_phi.add_incoming(&[(&bits_minus_1, loop_init_bb)]);
        quotient_phi.add_incoming(&[(&zero_wide, loop_init_bb)]);
        remainder_phi.add_incoming(&[(&zero_wide, loop_init_bb)]);

        let i_val = i_phi.as_basic_value().into_int_value();
        let quotient_val = quotient_phi.as_basic_value().into_int_value();
        let remainder_val = remainder_phi.as_basic_value().into_int_value();

        // Loop continuation condition: keep looping while i <= bits-1.
        // i is stored as an unsigned i64 counter.  After the final iteration
        // (i == 0), we compute i_next = 0 - 1, which wraps to u64::MAX.
        // Because u64::MAX > bits-1, the ULE check fails and the loop exits.
        let loop_cond = b
            .build_int_compare(IntPredicate::ULE, i_val, bits_minus_1, "loop_cond")
            .unwrap();
        b.build_conditional_branch(loop_cond, loop_body_bb, exit_bb)
            .unwrap();

        // ── loop_body: shift remainder and conditionally subtract ─────────────
        b.position_at_end(loop_body_bb);

        // i_wide = zext i64 %i to iN
        let i_wide = b
            .build_int_z_extend(i_val, wide_type, "i_wide")
            .unwrap();

        // remainder_shifted = remainder << 1
        let remainder_shifted = b
            .build_left_shift(remainder_val, one_wide, "rem_shifted")
            .unwrap();

        // bit = (dividend >> i_wide) & 1
        let bit_shifted = b
            .build_right_shift(dividend, i_wide, false, "bit_shifted")
            .unwrap();
        let bit = b
            .build_and(bit_shifted, one_wide, "bit")
            .unwrap();

        // remainder_new = remainder_shifted | bit
        let remainder_new = b
            .build_or(remainder_shifted, bit, "rem_new")
            .unwrap();

        // if remainder_new >= divisor → do_subtract, else → loop_merge
        let cmp_uge = b
            .build_int_compare(IntPredicate::UGE, remainder_new, divisor, "uge")
            .unwrap();
        b.build_conditional_branch(cmp_uge, do_subtract_bb, loop_merge_bb)
            .unwrap();

        // ── do_subtract: remainder -= divisor; quotient |= (1 << i) ──────────
        b.position_at_end(do_subtract_bb);
        let rem_after_sub = b
            .build_int_sub(remainder_new, divisor, "rem_sub")
            .unwrap();
        let one_shifted = b
            .build_left_shift(one_wide, i_wide, "one_shifted")
            .unwrap();
        let quotient_updated = b
            .build_or(quotient_val, one_shifted, "quot_updated")
            .unwrap();
        b.build_unconditional_branch(loop_merge_bb).unwrap();

        // ── loop_merge: update phis, decrement i ──────────────────────────────
        b.position_at_end(loop_merge_bb);
        let quotient_next_phi = b.build_phi(wide_type, "quotient_next").unwrap();
        quotient_next_phi.add_incoming(&[
            (&quotient_updated, do_subtract_bb),
            (&quotient_val, loop_body_bb),
        ]);
        let remainder_next_phi = b.build_phi(wide_type, "remainder_next").unwrap();
        remainder_next_phi.add_incoming(&[
            (&rem_after_sub, do_subtract_bb),
            (&remainder_new, loop_body_bb),
        ]);

        let quotient_next = quotient_next_phi.as_basic_value().into_int_value();
        let remainder_next = remainder_next_phi.as_basic_value().into_int_value();

        // i_next = i - 1  (wraps to u64::MAX when i == 0, which terminates loop)
        let one_i64 = i64_type.const_int(1, false);
        let i_next = b
            .build_int_sub(i_val, one_i64, "i_next")
            .unwrap();

        // Back-edge phis
        i_phi.add_incoming(&[(&i_next, loop_merge_bb)]);
        quotient_phi.add_incoming(&[(&quotient_next, loop_merge_bb)]);
        remainder_phi.add_incoming(&[(&remainder_next, loop_merge_bb)]);

        b.build_unconditional_branch(loop_check_bb).unwrap();

        // ── exit: return quotient ─────────────────────────────────────────────
        b.position_at_end(exit_bb);
        b.build_return(Some(&quotient_val)).unwrap();
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

            // For the entry function, store __argc and __argv into globals
            // so that IO intrinsics (argument-count, receives-argument) can
            // access them without threading parameters through every call.
            let is_main = func.name == "run" || func.name == "main";
            if is_main {
                self.store_entry_point_globals(func);
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

    /// Store the entry-point `__argc` (i32→i64) and `__argv` (i64→ptr)
    /// parameters into internal globals so that IO intrinsics can access
    /// them from any call depth.
    fn store_entry_point_globals(&self, func: &MirFunction) {
        use inkwell::types::BasicType;
        use crate::adapters::codegen::compat::{onu_i8ptr, build_typed_load};

        let i64_type = self.context.i64_type();
        let i8_ptr_type = onu_i8ptr(self.context);
        let i8_ptr_ptr_type = onu_i8ptr(self.context); // opaque: same ptr type

        // __argc → __onu_argc (i64)
        if let Some(argc_mir) = func.args.iter().find(|a| a.name == "__argc") {
            if let Some(argc_alloca) = self.ssa_storage.get(&argc_mir.ssa_var) {
                let argc_i32 = build_typed_load(self.context, &self.builder, i64_type, *argc_alloca, "argc_i32").into_int_value();
                let argc_i64 = self.builder.build_int_z_extend(argc_i32, i64_type, "argc_i64").unwrap();

                let g = self.get_or_declare_global("__onu_argc", i64_type.as_basic_type_enum());
                self.builder.build_store(g, argc_i64).unwrap();
            }
        }

        // __argv → __onu_argv (ptr)
        if let Some(argv_mir) = func.args.iter().find(|a| a.name == "__argv") {
            if let Some(argv_alloca) = self.ssa_storage.get(&argv_mir.ssa_var) {
                let argv_i64 = build_typed_load(self.context, &self.builder, i64_type, *argv_alloca, "argv_i64").into_int_value();
                let argv_ptr = self.builder.build_int_to_ptr(argv_i64, i8_ptr_ptr_type, "argv_ptr").unwrap();

                let g = self.get_or_declare_global("__onu_argv", i8_ptr_ptr_type.as_basic_type_enum());
                self.builder.build_store(g, argv_ptr).unwrap();
            }
        }
        let _ = i8_ptr_type; // suppress unused warning on opaque-pointer builds
    }

    /// Get or declare an internal global variable with the given name and type.
    fn get_or_declare_global(&self, name: &str, typ: inkwell::types::BasicTypeEnum<'ctx>) -> PointerValue<'ctx> {
        if let Some(g) = self.module.get_global(name) {
            g.as_pointer_value()
        } else {
            let g = self.module.add_global(typ, None, name);
            g.set_linkage(Linkage::Internal);
            match typ {
                inkwell::types::BasicTypeEnum::IntType(t) => g.set_initializer(&t.const_zero()),
                inkwell::types::BasicTypeEnum::PointerType(t) => g.set_initializer(&t.const_null()),
                _ => {}
            }
            g.as_pointer_value()
        }
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
            MirInstruction::GlobalAlloc { .. } => GlobalAllocStrategy.generate(
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
            MirInstruction::MemSet { .. } => MemSetStrategy.generate(
                self.context,
                &self.module,
                &self.builder,
                self.registry,
                &mut self.ssa_storage,
                inst,
            ),
            MirInstruction::Promote { .. } => PromoteStrategy.generate(
                self.context,
                &self.module,
                &self.builder,
                self.registry,
                &mut self.ssa_storage,
                inst,
            ),
            MirInstruction::BitCast { .. } => BitCastStrategy.generate(
                self.context,
                &self.module,
                &self.builder,
                self.registry,
                &mut self.ssa_storage,
                inst,
            ),
            MirInstruction::ConstantTableLoad { .. } => ConstantTableLoadStrategy.generate(
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
