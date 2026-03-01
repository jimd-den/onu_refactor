/// Ọ̀nụ MIR Lowering Service: Application Use Case
///
/// This service orchestrates the translation of HIR into MIR.
/// It delegates low-level construction details to the MirBuilder.

use crate::domain::entities::hir::{HirDiscourse, HirExpression, HirBehaviorHeader};
use crate::domain::entities::mir::*;
use crate::domain::entities::types::OnuType;
use crate::application::use_cases::mir_builder::MirBuilder;
use crate::application::use_cases::registry_service::RegistryService;
use crate::domain::entities::error::OnuError;
use crate::application::ports::environment::EnvironmentPort;
use crate::application::options::LogLevel;

use super::mir_lowering::ExprLowerer;
use super::mir_lowering::lower_expr::{LiteralLowerer, VariableLowerer, BinaryOpLowerer, IndexLowerer, EmitLowerer};
use super::mir_lowering::lower_calls::CallLowerer;
use super::mir_lowering::lower_blocks::{BlockLowerer, DerivationLowerer};
use super::mir_lowering::lower_control_flow::IfLowerer;

pub struct LoweringContext<'a, E: EnvironmentPort> {
    pub env: &'a E,
    pub registry: &'a RegistryService,
}

impl<'a, E: EnvironmentPort> LoweringContext<'a, E> {
    pub fn lower_expression(&self, expr: &HirExpression, builder: &mut MirBuilder, is_tail: bool) -> Result<MirOperand, OnuError> {
        let service = MirLoweringService::new(self.env, self.registry);
        let res = service.lower_expression(expr, builder, is_tail)?;
        
        // Parent-Cleans-Up-Children policy: schedule drop for the intermediate result.
        // The caller of this context.lower_expression() is responsible for emitting the drop.
        service.collect_resource_drop(&res, builder);
        
        Ok(res)
    }
}

pub struct MirLoweringService<'a, E: EnvironmentPort> {
    pub context: LoweringContext<'a, E>,
}

impl<'a, E: EnvironmentPort> MirLoweringService<'a, E> {
    pub fn new(env: &'a E, registry: &'a RegistryService) -> Self {
        Self { context: LoweringContext { env, registry } }
    }

    pub(crate) fn log(&self, level: LogLevel, message: &str) {
        self.context.env.log(level, &format!("[MirLowering] {}", message));
    }

    pub fn lower_program(&self, discourses: &[HirDiscourse]) -> Result<MirProgram, OnuError> {
        self.log(LogLevel::Info, "Starting MIR lowering for program");
        let mut functions = Vec::new();
        for discourse in discourses {
            if let HirDiscourse::Behavior { header, body } = discourse {
                functions.push(self.lower_function(header, body)?);
            }
        }
        self.log(LogLevel::Info, &format!("MIR lowering successful: {} functions", functions.len()));
        Ok(MirProgram { functions })
    }

    fn lower_function(&self, header: &HirBehaviorHeader, body: &HirExpression) -> Result<MirFunction, OnuError> {
        self.log(LogLevel::Debug, &format!("Lowering behavior: {}", header.name));
        let mut builder = MirBuilder::new(header.name.clone(), header.return_type.clone());
        
        for arg in &header.args {
            let ssa_var = builder.new_ssa();
            builder.define_variable(&arg.name, ssa_var, arg.typ.clone());
            builder.add_arg(arg.name.clone(), arg.typ.clone(), ssa_var);
        }

        let result_op = self.lower_expression(body, &mut builder, true)?;
        
        if builder.get_current_block_id().is_some() {
            // Drop all surviving resources (intermediate results, unconsumed arguments, etc.)
            for (var_id, var_typ, var_name, is_dyn) in builder.get_surviving_resources() {
                if is_dyn {
                    builder.emit(MirInstruction::Drop { ssa_var: var_id, typ: var_typ, name: var_name, is_dynamic: is_dyn });
                }
            }
            builder.terminate(MirTerminator::Return(result_op));
        }

        Ok(builder.build())
    }

    pub(crate) fn collect_resource_drop(&self, op: &MirOperand, builder: &mut MirBuilder) {
        if let MirOperand::Variable(ssa_id, _) = op {
            if let Some(typ) = builder.resolve_ssa_type(*ssa_id) {
                if typ.is_resource() {
                    builder.schedule_drop(*ssa_id, typ);
                }
            }
        }
    }

    pub(crate) fn lower_expression(&self, expr: &HirExpression, builder: &mut MirBuilder, is_tail: bool) -> Result<MirOperand, OnuError> {
        self.log(LogLevel::Trace, &format!("Lowering expression: {:?}", expr));
        
        let res = match expr {
            HirExpression::Literal(_) => LiteralLowerer.lower(expr, &self.context, builder, is_tail),
            HirExpression::Variable(_, _) => VariableLowerer.lower(expr, &self.context, builder, is_tail),
            HirExpression::BinaryOp { .. } => BinaryOpLowerer.lower(expr, &self.context, builder, is_tail),
            HirExpression::Call { .. } => CallLowerer.lower(expr, &self.context, builder, is_tail),
            HirExpression::Derivation { .. } => DerivationLowerer.lower(expr, &self.context, builder, is_tail),
            HirExpression::If { .. } => IfLowerer.lower(expr, &self.context, builder, is_tail),
            HirExpression::Block(_) => BlockLowerer.lower(expr, &self.context, builder, is_tail),
            HirExpression::Emit(_) => EmitLowerer.lower(expr, &self.context, builder, is_tail),
            HirExpression::Drop(e) => {
                let op = self.lower_expression(e, builder, false)?;
                if let MirOperand::Variable(ssa_var, _) = op {
                    let typ = builder.resolve_ssa_type(ssa_var).unwrap_or(OnuType::Nothing);
                    let is_dyn = builder.resolve_ssa_is_dynamic(ssa_var);
                    builder.emit(MirInstruction::Drop { ssa_var, typ, name: "manual_drop".to_string(), is_dynamic: is_dyn });
                }
                Ok(MirOperand::Constant(MirLiteral::Nothing))
            }
            HirExpression::Index { .. } => IndexLowerer.lower(expr, &self.context, builder, is_tail),
            _ => {
                Err(OnuError::GrammarViolation {
                    message: format!("Unsupported HIR expression type for MIR lowering: {:?}", expr),
                    span: Default::default(),
                })
            }
        }?;

        // CENTRALIZED POLICY: Evaluation is complete.
        // We DO NOT emit drops here, as parents need the result.
        // Drops are emitted at scope boundaries.
        
        self.log(LogLevel::Trace, &format!("Expression result: {:?}", res));
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::hir::{HirBinOp, HirLiteral};
    use crate::infrastructure::os::NativeOsEnvironment;
    use crate::application::options::LogLevel;
    use crate::application::use_cases::registry_service::RegistryService;

    #[test]
    fn test_double_free_regression() {
        let env = NativeOsEnvironment::new(LogLevel::Error);
        let registry = RegistryService::new();
        let service = MirLoweringService::new(&env, &registry);
        let mut builder = MirBuilder::new("test".to_string(), OnuType::Boolean);

        // 1. Define a resource variable (String)
        let ssa_id = 100;
        builder.define_variable("my_resource", ssa_id, OnuType::Strings);
        builder.set_ssa_type(ssa_id, OnuType::Strings);
        builder.set_ssa_is_dynamic(ssa_id, true);

        // 2. Create a BinaryOp that uses it: (my_resource == "other")
        let expr = HirExpression::BinaryOp {
            op: HirBinOp::Equal,
            left: Box::new(HirExpression::Variable("my_resource".to_string(), true)),
            right: Box::new(HirExpression::Literal(HirLiteral::Text("other".to_string()))),
        };

        // 3. Lower the expression
        let res = service.lower_expression(&expr, &mut builder, false).unwrap();
        
        // 4. Trigger manual cleanup (simulation of parent or function exit)
        service.collect_resource_drop(&res, &mut builder);
        let pending = builder.take_pending_drops();
        for (var, typ, name, is_dyn) in pending {
            if is_dyn {
                builder.emit(MirInstruction::Drop { ssa_var: var, typ, name, is_dynamic: is_dyn });
            }
        }

        // 5. Inspect the MIR instructions
        let func = builder.build();
        let instructions = &func.blocks[0].instructions;
        
        let drop_count = instructions.iter().filter(|inst| {
            if let MirInstruction::Drop { ssa_var, .. } = inst {
                *ssa_var == ssa_id
            } else {
                false
            }
        }).count();

        // One drop only
        assert_eq!(drop_count, 1, "Resource SSA {} was dropped {} times, expected exactly once. Instructions: {:?}", ssa_id, drop_count, instructions);
    }

    #[test]
    fn test_alias_double_free() {
        let env = NativeOsEnvironment::new(LogLevel::Error);
        let registry = RegistryService::new();
        let service = MirLoweringService::new(&env, &registry);
        let mut builder = MirBuilder::new("test".to_string(), OnuType::Strings);

        // Lower: ("a" joined-with "b")
        let expr = HirExpression::Call {
            name: "joined-with".to_string(),
            args: vec![
                HirExpression::Literal(HirLiteral::Text("a".to_string())),
                HirExpression::Literal(HirLiteral::Text("b".to_string())),
            ],
        };

        let res = service.lower_expression(&expr, &mut builder, false).unwrap();
        service.collect_resource_drop(&res, &mut builder);

        // Manual trigger
        let pending = builder.take_pending_drops();
        for (var, typ, name, is_dyn) in pending {
            if is_dyn {
                builder.emit(MirInstruction::Drop { ssa_var: var, typ, name, is_dynamic: is_dyn });
            }
        }

        let func = builder.build();
        let instructions = &func.blocks[0].instructions;

        // Check for any Drop instructions
        let drops: Vec<_> = instructions.iter().filter(|inst| matches!(inst, MirInstruction::Drop { .. })).collect();
        
        // With literal strings (static), there should be NO drops at all (Zero Cost)
        for drop in &drops {
            if let MirInstruction::Drop { is_dynamic, .. } = drop {
                assert!(*is_dynamic, "Zero-cost violation: emitted IR for static resource drop: {:?}", drop);
            }
        }
    }

    #[test]
    fn test_chained_joined_with_double_free() {
        let env = NativeOsEnvironment::new(LogLevel::Error);
        let registry = RegistryService::new();
        let service = MirLoweringService::new(&env, &registry);
        let mut builder = MirBuilder::new("test".to_string(), OnuType::Strings);

        // Lower: ("a" joined-with "b") joined-with "c"
        let expr = HirExpression::Call {
            name: "joined-with".to_string(),
            args: vec![
                HirExpression::Call {
                    name: "joined-with".to_string(),
                    args: vec![
                        HirExpression::Literal(HirLiteral::Text("a".to_string())),
                        HirExpression::Literal(HirLiteral::Text("b".to_string())),
                    ],
                },
                HirExpression::Literal(HirLiteral::Text("c".to_string())),
            ],
        };

        let res = service.lower_expression(&expr, &mut builder, false).unwrap();
        service.collect_resource_drop(&res, &mut builder);

        // Manual trigger
        let pending = builder.take_pending_drops();
        for (var, typ, name, is_dyn) in pending {
            if is_dyn {
                builder.emit(MirInstruction::Drop { ssa_var: var, typ, name, is_dynamic: is_dyn });
            }
        }

        let func = builder.build();
        let instructions = &func.blocks[0].instructions;

        // Collect all dynamic SSA IDs from Tuple instructions
        let mut dynamic_ssas = Vec::new();
        for inst in instructions {
            if let MirInstruction::Tuple { dest, elements } = inst {
                if let MirOperand::Constant(MirLiteral::Boolean(true)) = elements[2] {
                    dynamic_ssas.push(*dest);
                }
            }
        }

        // For each dynamic SSA, it should be dropped exactly ONCE if it was an intermediate.
        for ssa_id in dynamic_ssas {
            let drop_count = instructions.iter().filter(|inst| {
                if let MirInstruction::Drop { ssa_var, .. } = inst {
                    *ssa_var == ssa_id
                } else {
                    false
                }
            }).count();

            assert_eq!(drop_count, 1, "Resource SSA {} was dropped {} times, expected exactly once. Instructions: {:?}", ssa_id, drop_count, instructions);
        }
    }
}
