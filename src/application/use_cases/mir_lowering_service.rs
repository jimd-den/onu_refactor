/// Ọ̀nụ MIR Lowering Service: Application Use Case
///
/// This service orchestrates the translation of HIR into MIR.
/// It delegates low-level construction details to the MirBuilder.

use crate::domain::entities::hir::{HirDiscourse, HirExpression, HirBehaviorHeader, HirBinOp};
use crate::domain::entities::mir::*;
use crate::domain::entities::types::OnuType;
use crate::application::use_cases::mir_builder::MirBuilder;
use crate::application::use_cases::registry_service::RegistryService;
use crate::domain::entities::error::OnuError;
use crate::application::ports::environment::EnvironmentPort;
use crate::application::options::LogLevel;

pub struct MirLoweringService<'a, E: EnvironmentPort> {
    pub env: &'a E,
    pub registry: &'a RegistryService,
}

impl<'a, E: EnvironmentPort> MirLoweringService<'a, E> {
    pub fn new(env: &'a E, registry: &'a RegistryService) -> Self {
        Self { env, registry }
    }

    pub(crate) fn log(&self, level: LogLevel, message: &str) {
        self.env.log(level, &format!("[MirLowering] {}", message));
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
            builder.terminate(MirTerminator::Return(result_op));
        }

        Ok(builder.build())
    }

    pub(crate) fn lower_expression(&self, expr: &HirExpression, builder: &mut MirBuilder, is_tail: bool) -> Result<MirOperand, OnuError> {
        self.log(LogLevel::Trace, &format!("Lowering expression: {:?}", expr));
        let res = match expr {
            HirExpression::Literal(lit) => self.lower_literal(lit),
            HirExpression::Variable(name, is_consuming) => self.lower_variable(name, *is_consuming, builder),
            HirExpression::BinaryOp { op, left, right } => {
                let lhs = self.lower_expression(left, builder, false)?;
                let rhs = self.lower_expression(right, builder, false)?;
                let dest = builder.new_ssa();
                let mir_op = match op {
                    HirBinOp::Add => MirBinOp::Add,
                    HirBinOp::Sub => MirBinOp::Sub,
                    HirBinOp::Mul => MirBinOp::Mul,
                    HirBinOp::Div => MirBinOp::Div,
                    HirBinOp::Equal => MirBinOp::Eq,
                    HirBinOp::NotEqual => MirBinOp::Eq,
                    HirBinOp::LessThan => MirBinOp::Lt,
                    HirBinOp::GreaterThan => MirBinOp::Gt,
                };
                builder.emit(MirInstruction::BinaryOperation { dest, op: mir_op, lhs, rhs });
                let pending = builder.take_pending_drops();
                for (var, typ) in pending { builder.emit(MirInstruction::Drop { ssa_var: var, typ }); }
                Ok(MirOperand::Variable(dest, false))
            }
            HirExpression::Call { name, args } => self.lower_call(name, args, builder),
            HirExpression::Derivation { name, typ, value, body } => self.lower_derivation(name, typ, value, body, builder, is_tail),
            HirExpression::If { condition, then_branch, else_branch } => self.lower_if(condition, then_branch, else_branch, builder, is_tail),
            HirExpression::Block(exprs) => self.lower_block(exprs, builder, is_tail),
            HirExpression::Emit(e) => {
                let op = self.lower_expression(e, builder, false)?;
                builder.emit(MirInstruction::Emit(op));
                let pending = builder.take_pending_drops();
                for (var, typ) in pending { builder.emit(MirInstruction::Drop { ssa_var: var, typ }); }
                Ok(MirOperand::Constant(MirLiteral::Nothing))
            }
            HirExpression::Drop(e) => {
                let op = self.lower_expression(e, builder, false)?;
                if let MirOperand::Variable(ssa_var, _) = op {
                    builder.emit(MirInstruction::Drop { ssa_var, typ: OnuType::Nothing });
                }
                let pending = builder.take_pending_drops();
                for (var, typ) in pending { builder.emit(MirInstruction::Drop { ssa_var: var, typ }); }
                Ok(MirOperand::Constant(MirLiteral::Nothing))
            }
            HirExpression::Index { subject, index } => {
                let op = self.lower_expression(subject, builder, false)?;
                let dest = builder.new_ssa();
                builder.emit(MirInstruction::Index { dest, subject: op, index: *index });
                let pending = builder.take_pending_drops();
                for (var, typ) in pending { builder.emit(MirInstruction::Drop { ssa_var: var, typ }); }
                Ok(MirOperand::Variable(dest, false))
            }
            _ => Ok(MirOperand::Constant(MirLiteral::Nothing)),
        }?;
        self.log(LogLevel::Trace, &format!("Expression result: {:?}", res));
        Ok(res)
    }
}
