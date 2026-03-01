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
        service.lower_expression(expr, builder, is_tail)
    }
}

pub struct LoweringContext<'a, E: EnvironmentPort> {
    pub env: &'a E,
    pub registry: &'a RegistryService,
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
            for (var_id, var_typ) in builder.get_surviving_resources() {
                builder.emit(MirInstruction::Drop { ssa_var: var_id, typ: var_typ });
            }
            builder.terminate(MirTerminator::Return(result_op));
        }

        Ok(builder.build())
    }

    fn collect_resource_drop(&self, op: &MirOperand, builder: &mut MirBuilder) {
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
                    builder.emit(MirInstruction::Drop { ssa_var, typ });
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

        let pending = builder.take_pending_drops();
        for (var, typ) in pending { builder.emit(MirInstruction::Drop { ssa_var: var, typ }); }
        
        self.log(LogLevel::Trace, &format!("Expression result: {:?}", res));
        Ok(res)
    }
}
