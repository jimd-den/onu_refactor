use crate::domain::entities::hir::{HirExpression, HirLiteral, HirBinOp};
use crate::domain::entities::mir::{MirOperand, MirLiteral, MirBinOp, MirInstruction};
use crate::domain::entities::types::OnuType;
use crate::application::use_cases::mir_builder::MirBuilder;
use crate::domain::entities::error::OnuError;
use super::super::mir_lowering_service::{MirLoweringService, LoweringContext};
use crate::application::ports::environment::EnvironmentPort;
use super::ExprLowerer;

pub struct LiteralLowerer;
pub struct VariableLowerer;
pub struct BinaryOpLowerer;
pub struct IndexLowerer;
pub struct EmitLowerer;

impl ExprLowerer for IndexLowerer {
    fn lower<'a, E: EnvironmentPort>(
        &self,
        expr: &HirExpression,
        context: &LoweringContext<'a, E>,
        builder: &mut MirBuilder,
        _is_tail: bool,
    ) -> Result<MirOperand, OnuError> {
        if let HirExpression::Index { subject, index } = expr {
            let op = context.lower_expression(subject, builder, false)?;

            let dest = builder.new_ssa();
            builder.emit(MirInstruction::Index { 
                dest, 
                subject: op.clone(), 
                index: *index 
            });
            
            // Parent cleanup: mark consumed and schedule drop if it's a resource variable
            if let MirOperand::Variable(ssa_id, _) = &op {
                if let Some(typ) = builder.resolve_ssa_type(*ssa_id) {
                    if typ.is_resource() {
                        builder.mark_consumed(*ssa_id);
                        builder.schedule_drop(*ssa_id, typ);
                    }
                }
            }

            Ok(MirOperand::Variable(dest, false))
        } else {
            Err(OnuError::GrammarViolation {
                message: "Expected Index expression".to_string(),
                span: Default::default(),
            })
        }
    }
}

impl ExprLowerer for EmitLowerer {
    fn lower<'a, E: EnvironmentPort>(
        &self,
        expr: &HirExpression,
        context: &LoweringContext<'a, E>,
        builder: &mut MirBuilder,
        _is_tail: bool,
    ) -> Result<MirOperand, OnuError> {
        if let HirExpression::Emit(e) = expr {
            let op = context.lower_expression(e, builder, false)?;

            builder.emit(MirInstruction::Emit(op.clone()));
            
            // Parent cleanup: Emit takes custody
            if let MirOperand::Variable(ssa_id, _) = &op {
                if let Some(typ) = builder.resolve_ssa_type(*ssa_id) {
                    if typ.is_resource() {
                        builder.mark_consumed(*ssa_id);
                        builder.schedule_drop(*ssa_id, typ);
                    }
                }
            }

            Ok(MirOperand::Constant(MirLiteral::Nothing))
        } else {
            Err(OnuError::GrammarViolation {
                message: "Expected Emit expression".to_string(),
                span: Default::default(),
            })
        }
    }
}

impl ExprLowerer for BinaryOpLowerer {
    fn lower<'a, E: EnvironmentPort>(
        &self,
        expr: &HirExpression,
        context: &LoweringContext<'a, E>,
        builder: &mut MirBuilder,
        _is_tail: bool,
    ) -> Result<MirOperand, OnuError> {
        if let HirExpression::BinaryOp { op, left, right } = expr {
            let lhs = context.lower_expression(left, builder, false)?;
            let rhs = context.lower_expression(right, builder, false)?;

            let dest = builder.new_ssa();
            let mir_op = match op {
                HirBinOp::Add => MirBinOp::Add,
                HirBinOp::Sub => MirBinOp::Sub,
                HirBinOp::Mul => MirBinOp::Mul,
                HirBinOp::Div => MirBinOp::Div,
                HirBinOp::Equal => MirBinOp::Eq,
                HirBinOp::NotEqual => MirBinOp::Ne,
                HirBinOp::LessThan => MirBinOp::Lt,
                HirBinOp::GreaterThan => MirBinOp::Gt,
            };
            
            builder.emit(MirInstruction::BinaryOperation { 
                dest, 
                op: mir_op, 
                lhs: lhs.clone(), 
                rhs: rhs.clone() 
            });
            
            // Parent cleanup: mark inputs as consumed and schedule drop if they are resource variables
            if let MirOperand::Variable(ssa_id, _) = &lhs {
                if let Some(typ) = builder.resolve_ssa_type(*ssa_id) {
                    if typ.is_resource() {
                        builder.mark_consumed(*ssa_id);
                        builder.schedule_drop(*ssa_id, typ);
                    }
                }
            }
            if let MirOperand::Variable(ssa_id, _) = &rhs {
                if let Some(typ) = builder.resolve_ssa_type(*ssa_id) {
                    if typ.is_resource() {
                        builder.mark_consumed(*ssa_id);
                        builder.schedule_drop(*ssa_id, typ);
                    }
                }
            }

            // Register type for the result (assume I64/Boolean for now as per current binops)
            let res_type = match op {
                HirBinOp::Equal | HirBinOp::NotEqual | HirBinOp::LessThan | HirBinOp::GreaterThan => OnuType::Boolean,
                _ => OnuType::I64,
            };
            builder.set_ssa_type(dest, res_type);

            Ok(MirOperand::Variable(dest, false))
        } else {
            Err(OnuError::GrammarViolation {
                message: "Expected BinaryOp expression".to_string(),
                span: Default::default(),
            })
        }
    }
}

impl ExprLowerer for LiteralLowerer {
    fn lower<'a, E: EnvironmentPort>(
        &self,
        expr: &HirExpression,
        _context: &LoweringContext<'a, E>,
        _builder: &mut MirBuilder,
        _is_tail: bool,
    ) -> Result<MirOperand, OnuError> {
        if let HirExpression::Literal(lit) = expr {
            let mir_lit = match lit {
                HirLiteral::I64(n) => MirLiteral::I64(*n),
                HirLiteral::F64(n) => MirLiteral::F64(*n),
                HirLiteral::Boolean(b) => MirLiteral::Boolean(*b),
                HirLiteral::Text(s) => MirLiteral::Text(s.clone()),
                HirLiteral::Nothing => MirLiteral::Nothing,
            };
            Ok(MirOperand::Constant(mir_lit))
        } else {
            Err(OnuError::GrammarViolation {
                message: "Expected Literal expression".to_string(),
                span: Default::default(),
            })
        }
    }
}

impl ExprLowerer for VariableLowerer {
    fn lower<'a, E: EnvironmentPort>(
        &self,
        expr: &HirExpression,
        _context: &LoweringContext<'a, E>,
        builder: &mut MirBuilder,
        _is_tail: bool,
    ) -> Result<MirOperand, OnuError> {
        if let HirExpression::Variable(name, is_consuming) = expr {
            let ssa_var = builder.resolve_variable(name)
                .ok_or_else(|| OnuError::GrammarViolation {
                    message: format!("Unresolved variable: {}", name),
                    span: crate::domain::entities::error::Span::default()
                })?;
            let typ = builder.resolve_variable_type(name).unwrap_or(OnuType::Nothing);
            if *is_consuming && typ.is_resource() {
                builder.schedule_drop(ssa_var, typ.clone());
                builder.mark_consumed(ssa_var);
            }
            Ok(MirOperand::Variable(ssa_var, *is_consuming))
        } else {
            Err(OnuError::GrammarViolation {
                message: "Expected Variable expression".to_string(),
                span: Default::default(),
            })
        }
    }
}

// --- Legacy Compatibility ---
impl<'a, E: EnvironmentPort> MirLoweringService<'a, E> {
    pub fn lower_literal(&self, lit: &HirLiteral) -> Result<MirOperand, OnuError> {
        LiteralLowerer.lower(&HirExpression::Literal(lit.clone()), &self.context, &mut MirBuilder::new("tmp".to_string(), OnuType::Nothing), false)
    }

    pub fn lower_variable(&self, name: &str, is_consuming: bool, builder: &mut MirBuilder) -> Result<MirOperand, OnuError> {
        VariableLowerer.lower(&HirExpression::Variable(name.to_string(), is_consuming), &self.context, builder, false)
    }

    pub fn lower_index(&self, subject: &HirExpression, index: usize, builder: &mut MirBuilder) -> Result<MirOperand, OnuError> {
        IndexLowerer.lower(&HirExpression::Index { subject: Box::new(subject.clone()), index }, &self.context, builder, false)
    }

    pub fn lower_emit(&self, e: &HirExpression, builder: &mut MirBuilder) -> Result<MirOperand, OnuError> {
        EmitLowerer.lower(&HirExpression::Emit(Box::new(e.clone())), &self.context, builder, false)
    }
}
