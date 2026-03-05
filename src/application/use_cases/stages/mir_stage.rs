use super::PipelineStage;
use crate::application::ports::environment::EnvironmentPort;
use crate::application::use_cases::inline_pass::InlinePass;
use crate::application::use_cases::integer_upgrade_pass::IntegerUpgradePass;
use crate::application::use_cases::memo_pass::MemoPass;
use crate::application::use_cases::mir_lowering_service::MirLoweringService;
use crate::application::use_cases::registry_service::RegistryService;
use crate::application::use_cases::tco_pass::TcoPass;
use crate::application::use_cases::wide_div_legalization_pass::WideDivLegalizationPass;
use crate::domain::entities::error::OnuError;
use crate::domain::entities::hir::HirDiscourse;
use crate::domain::entities::mir::MirProgram;

pub struct MirStage<'a, E: EnvironmentPort> {
    env: &'a E,
    registry: &'a RegistryService,
}

impl<'a, E: EnvironmentPort> MirStage<'a, E> {
    pub fn new(env: &'a E, registry: &'a RegistryService) -> Self {
        Self { env, registry }
    }
}

impl<'a, E: EnvironmentPort> PipelineStage for MirStage<'a, E> {
    type Input = Vec<HirDiscourse>;
    type Output = MirProgram;

    fn execute(&mut self, hir_discourses: Vec<HirDiscourse>) -> Result<MirProgram, OnuError> {
        let mir_lowering_service = MirLoweringService::new(self.env, self.registry);
        // Stage 1: Lower HIR → MIR (recursive call structure, raw SSA).
        let mir_program = mir_lowering_service.lower_program(&hir_discourses)?;
        // Stage 2: Loop-lower self-tail-calls FIRST.
        // A recursive function cannot be safely inlined into its caller because the inlined
        // body would contain another call to itself, causing infinite expansion. TcoPass
        // rewrites self-recursion into a loop, making the body finite and therefore inlineable.
        let mir_program = TcoPass::run(mir_program);
        // Stage 3: Expand loop-shaped pure callees inline into their callers.
        // Now that collatz-steps is a loop (not recursive), InlinePass can safely expand it
        // into collatz-range, fusing the two loops into one for LLVM to optimize holistically.
        let mir_program = InlinePass::run(mir_program);
        // Stage 4: Run TcoPass again to catch any new self-tail-calls that emerged.
        let mir_program = TcoPass::run(mir_program);
        // Stage 4.5: Automatically promote doubly-recursive pure functions from I64
        // to WideInt(bits) when call-site literals imply overflow.  This gives the
        // correct full-precision answer (e.g. fib(1000) as a 209-digit integer)
        // using native LLVM wide integers — no external BigInt library required.
        // MemoPass (Stage 5) will then memoize the widened function for O(n) time.
        let mir_program = IntegerUpgradePass::run(mir_program);
        // Stage 5: Memoization for recursive algorithms based on diminishing hints.
        let mir_program = MemoPass::run(mir_program, self.registry);
        // Stage 6: Operation Legalization — replace any WideInt (> 128-bit) division or
        // modulo with a call to a compiler-internal helper function (__onu_wide_div_N /
        // __onu_wide_mod_N).  This prevents the LLVM backend from attempting to lower
        // an `sdiv iN` for which no runtime library entry exists, which would otherwise
        // cause a segmentation fault during code generation.
        let mir_program = WideDivLegalizationPass::run(mir_program);
        Ok(mir_program)
    }
}
