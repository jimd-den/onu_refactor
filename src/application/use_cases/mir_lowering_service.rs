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
        
        // GLOBAL POLICY: The parent caller is responsible for the result of this evaluation.
        // We schedule it for drop so the caller's next `take_pending_drops` emits it.
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
                if is_dyn && !builder.is_consumed(var_id) {
                    builder.emit(MirInstruction::Drop { ssa_var: var_id, typ: var_typ, name: var_name, is_dynamic: is_dyn });
                }
            }
            builder.terminate(MirTerminator::Return(result_op));
        }

        Ok(builder.build())
    }

    pub(crate) fn collect_resource_drop(&self, op: &MirOperand, builder: &mut MirBuilder) {
        if let MirOperand::Variable(ssa_id, is_consuming) = op {
            if *is_consuming {
                if let Some(typ) = builder.resolve_ssa_type(*ssa_id) {
                    if typ.is_resource() {
                        builder.schedule_drop(*ssa_id, typ);
                    }
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
                    builder.emit(MirInstruction::Drop { ssa_var: ssa_var, typ, name: "manual_drop".to_string(), is_dynamic: is_dyn });
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

        // CENTRALIZED POLICY: Sub-expressions evaluation is complete.
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
            if is_dyn && !builder.is_consumed(var) {
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
    fn test_nested_joined_with_leak() {
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
        
        // Manual cleanup simulation
        service.collect_resource_drop(&res, &mut builder);
        let pending = builder.take_pending_drops();
        for (var, typ, name, is_dyn) in pending {
            if is_dyn && !builder.is_consumed(var) {
                builder.emit(MirInstruction::Drop { ssa_var: var, typ, name, is_dynamic: is_dyn });
            }
        }

        let func = builder.build();
        let instructions = &func.blocks[0].instructions;

        let dynamic_ssas: Vec<_> = instructions.iter().filter_map(|inst| {
            if let MirInstruction::Tuple { dest, elements } = inst {
                if let MirOperand::Constant(MirLiteral::Boolean(true)) = elements[2] {
                    return Some(*dest);
                }
            }
            None
        }).collect();

        assert_eq!(dynamic_ssas.len(), 2, "Should have created 2 dynamic strings");

        for ssa_id in dynamic_ssas {
            let drop_count = instructions.iter().filter(|inst| {
                if let MirInstruction::Drop { ssa_var, .. } = inst {
                    *ssa_var == ssa_id
                } else {
                    false
                }
            }).count();

            assert_eq!(drop_count, 1, "Resource SSA {} should be dropped exactly once", ssa_id);
        }
    }

    #[test]
    fn test_mark_consumed_pending_drops() {
        let mut builder = MirBuilder::new("test".to_string(), OnuType::Nothing);
        let ssa_id = 77;
        builder.set_ssa_type(ssa_id, OnuType::Strings);
        builder.set_ssa_is_dynamic(ssa_id, true);
        
        // 1. Schedule a drop
        builder.schedule_drop(ssa_id, OnuType::Strings);
        assert_eq!(builder.take_pending_drops().len(), 1);
        
        // 2. Schedule again, then mark consumed
        builder.schedule_drop(ssa_id, OnuType::Strings);
        builder.mark_consumed(ssa_id);
        
        // Pending drops should be cleared when marked consumed
        let remaining = builder.take_pending_drops();
        assert_eq!(remaining.len(), 0, "Pending drops should be cleared when marked consumed!");
    }
}
