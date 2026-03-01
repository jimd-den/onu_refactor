use crate::domain::entities::hir::HirExpression;
use crate::domain::entities::mir::{MirInstruction, MirOperand};
use crate::domain::entities::types::OnuType;
use crate::application::use_cases::mir_builder::MirBuilder;
use crate::domain::entities::error::OnuError;
use super::super::mir_lowering_service::MirLoweringService;
use crate::application::use_cases::stdlib_lowering::StdlibLowering;
use crate::application::ports::environment::EnvironmentPort;

impl<'a, E: EnvironmentPort> MirLoweringService<'a, E> {
    pub fn lower_call(&self, name: &str, args: &[HirExpression], builder: &mut MirBuilder) -> Result<MirOperand, OnuError> {
        let mut mir_args = Vec::new();
        for arg in args {
            mir_args.push(self.lower_expression(arg, builder, false)?);
        }

        let (return_type, arg_types, arg_is_observation) = if let Some(sig) = self.registry.get_signature(name) {
            (sig.return_type.clone(), sig.input_types.clone(), sig.arg_is_observation.clone())
        } else {
            (OnuType::Nothing, Vec::new(), Vec::new())
        };

        // Mark resource arguments as consumed if they are not observations
        // This applies to both built-in and general calls.
        for (i, arg_op) in mir_args.iter().enumerate() {
            if let MirOperand::Variable(ssa_id, _) = arg_op {
                let is_observation = arg_is_observation.get(i).copied().unwrap_or(false);
                let typ = arg_types.get(i).cloned().unwrap_or(OnuType::Nothing);
                if !is_observation && typ.is_resource() {
                    builder.mark_consumed(*ssa_id);
                    builder.schedule_drop(*ssa_id, typ);
                }
            }
        }

        if name == "as-text" && mir_args.len() == 1 {
            let res = StdlibLowering::lower_as_text(&mir_args[0], builder);
            if let MirOperand::Variable(ssa_id, _) = &res {
                builder.set_ssa_type(*ssa_id, OnuType::Strings);
            }
            return Ok(res);
        }

        if name == "joined-with" && mir_args.len() == 2 {
            let res = StdlibLowering::lower_joined_with(&mir_args[0], &mir_args[1], builder);
            if let MirOperand::Variable(ssa_id, _) = &res {
                builder.set_ssa_type(*ssa_id, OnuType::Strings);
            }
            return Ok(res);
        }

        let dest = builder.new_ssa();
        builder.emit(MirInstruction::Call {
            dest,
            name: name.to_string(),
            args: mir_args,
            return_type,
            arg_types,
        });

        Ok(MirOperand::Variable(dest, false))
    }
}
