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

        if name == "as-text" && mir_args.len() == 1 {
            let result = StdlibLowering::lower_as_text(&mir_args[0], builder);
            let pending = builder.take_pending_drops();
            for (var, typ) in pending {
                builder.emit(MirInstruction::Drop { ssa_var: var, typ });
            }
            return Ok(result);
        }

        if name == "joined-with" && mir_args.len() == 2 {
            let result = StdlibLowering::lower_joined_with(&mir_args[0], &mir_args[1], builder);
            let pending = builder.take_pending_drops();
            for (var, typ) in pending {
                builder.emit(MirInstruction::Drop { ssa_var: var, typ });
            }
            return Ok(result);
        }

        let dest = builder.new_ssa();
        let (return_type, arg_types) = if let Some(sig) = self.registry.get_signature(name) {
            (sig.return_type.clone(), sig.input_types.clone())
        } else {
            (OnuType::Nothing, Vec::new())
        };

        builder.emit(MirInstruction::Call {
            dest,
            name: name.to_string(),
            args: mir_args,
            return_type,
            arg_types,
        });

        let pending = builder.take_pending_drops();
        for (var, typ) in pending {
            builder.emit(MirInstruction::Drop { ssa_var: var, typ });
        }

        Ok(MirOperand::Variable(dest, false))
    }
}
