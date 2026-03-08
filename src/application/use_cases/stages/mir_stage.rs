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
        // Stage 2: Automatically promote doubly-recursive pure functions from I64
        // to WideInt(bits) when call-site literals imply overflow.  Must run before
        // MemoPass (so the wrapper caches WideInt values) and before TcoPass (so the
        // doubly-recursive call structure is still visible for candidate detection).
        let mir_program = IntegerUpgradePass::run(mir_program);
        // Stage 3: Memoization for recursive algorithms based on diminishing hints.
        // Must run BEFORE TcoPass: TcoPass erases tail-recursive Call instructions
        // into Branch loops, so any memoizable call that is also tail-recursive
        // would be invisible to MemoPass if the order were reversed.
        let mir_program = MemoPass::run(mir_program, self.registry);
        // Stage 4: Loop-lower self-tail-calls AFTER memoization.
        // A recursive function cannot be safely inlined into its caller because the
        // inlined body would contain another call to itself, causing infinite expansion.
        // TcoPass rewrites self-recursion into a loop, making the body finite and
        // inlineable.  It now also acts on the .inner functions produced by MemoPass.
        let mir_program = TcoPass::run(mir_program);
        // Stage 5: Expand loop-shaped pure callees inline into their callers.
        // Now that collatz-steps is a loop (not recursive), InlinePass can safely
        // expand it into collatz-range, fusing the two loops into one for LLVM
        // to optimize holistically.
        let mir_program = InlinePass::run(mir_program);
        // Stage 6: Run TcoPass again to catch any new self-tail-calls that emerged.
        let mir_program = TcoPass::run(mir_program);
        // Stage 7: Operation Legalization — replace any WideInt (> 128-bit) division or
        // modulo with a call to a compiler-internal helper function (__onu_wide_div_N /
        // __onu_wide_mod_N).  This prevents the LLVM backend from attempting to lower
        // an `sdiv iN` for which no runtime library entry exists, which would otherwise
        // cause a segmentation fault during code generation.
        let mir_program = WideDivLegalizationPass::run(mir_program);
        Ok(mir_program)
    }
}
