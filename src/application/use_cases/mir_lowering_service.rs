/// Ọ̀nụ MIR Lowering Service: Application Use Case
///
/// This service orchestrates the translation of HIR into MIR.
/// It delegates low-level construction details to the MirBuilder.

use crate::domain::entities::hir::{HirDiscourse, HirExpression, HirBehaviorHeader, HirLiteral};
use crate::domain::entities::mir::*;
use crate::domain::entities::types::OnuType;
use crate::application::use_cases::mir_builder::MirBuilder;
use crate::domain::entities::error::OnuError;
use crate::application::ports::environment::EnvironmentPort;
use crate::application::options::LogLevel;

pub struct MirLoweringService<'a, E: EnvironmentPort> {
    pub env: &'a E,
}

impl<'a, E: EnvironmentPort> MirLoweringService<'a, E> {
    pub fn new(env: &'a E) -> Self {
        Self { env }
    }

    fn log(&self, level: LogLevel, message: &str) {
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
            builder.define_variable(&arg.name, ssa_var);
            builder.add_arg(arg.name.clone(), arg.typ.clone(), ssa_var);
        }

        let result_op = self.lower_expression(body, &mut builder, true)?;
        
        if builder.get_current_block_id().is_some() {
            builder.terminate(MirTerminator::Return(result_op));
        }

        Ok(builder.build())
    }

    fn lower_expression(&self, expr: &HirExpression, builder: &mut MirBuilder, is_tail: bool) -> Result<MirOperand, OnuError> {
        self.log(LogLevel::Trace, &format!("Lowering expression: {:?}", expr));
        let res = match expr {
            HirExpression::Literal(lit) => {
                let mir_lit = match lit {
                    HirLiteral::I64(n) => MirLiteral::I64(*n),
                    HirLiteral::F64(n) => MirLiteral::F64(*n), 
                    HirLiteral::Boolean(b) => MirLiteral::Boolean(*b),
                    HirLiteral::Text(s) => MirLiteral::Text(s.clone()),
                    HirLiteral::Nothing => MirLiteral::Nothing,
                };
                Result::<MirOperand, OnuError>::Ok(MirOperand::Constant(mir_lit))
            }
            HirExpression::Variable(name, is_consuming) => {
                let ssa_var = builder.resolve_variable(name)
                    .ok_or_else(|| OnuError::GrammarViolation { 
                        message: format!("Unresolved variable: {}", name), 
                        span: crate::domain::entities::error::Span::default() 
                    })?;
                Result::<MirOperand, OnuError>::Ok(MirOperand::Variable(ssa_var, *is_consuming))
            }
            HirExpression::Call { name, args } => {
                let mut mir_args = Vec::new();
                for arg in args {
                    mir_args.push(self.lower_expression(arg, builder, false)?);
                }
                
                let bin_op = if mir_args.len() == 2 {
                    match name.as_str() {
                        "added-to" | "added_to" => Some(MirBinOp::Add),
                        "decreased-by" | "decreased_by" => Some(MirBinOp::Sub),
                        "scales-by" | "scales_by" => Some(MirBinOp::Mul),
                        "partitions-by" | "partitions_by" => Some(MirBinOp::Div),
                        "matches" => Some(MirBinOp::Eq),
                        "exceeds" => Some(MirBinOp::Gt),
                        "falls-short-of" | "falls_short_of" => Some(MirBinOp::Lt),
                        _ => None,
                    }
                } else { None };

                let dest = builder.new_ssa();
                if let Some(op) = bin_op {
                    builder.emit(MirInstruction::BinaryOperation {
                        dest,
                        op,
                        lhs: mir_args[0].clone(),
                        rhs: mir_args[1].clone(),
                    });
                } else {
                    builder.emit(MirInstruction::Call { dest, name: name.clone(), args: mir_args });
                }
                Ok(MirOperand::Variable(dest, false))
            }
            HirExpression::Derivation { name, value, body, .. } => {
                let val_op = self.lower_expression(value, builder, false)?;
                let ssa_var = builder.new_ssa();
                builder.emit(MirInstruction::Assign { dest: ssa_var, src: val_op });
                builder.enter_scope();
                builder.define_variable(name, ssa_var);
                let res = self.lower_expression(body, builder, is_tail)?;
                builder.exit_scope();
                Ok(res)
            }
            HirExpression::If { condition, then_branch, else_branch } => {
                let cond_op = self.lower_expression(condition, builder, false)?;
                let then_start_id = builder.create_block();
                let else_start_id = builder.create_block();
                
                builder.terminate(MirTerminator::CondBranch { 
                    condition: cond_op, 
                    then_block: then_start_id, 
                    else_block: else_start_id 
                });

                // Then branch
                builder.switch_to_block(then_start_id);
                let then_res = self.lower_expression(then_branch, builder, is_tail)?;
                let then_end_id = builder.get_current_block_id();

                // Else branch
                builder.switch_to_block(else_start_id);
                let else_res = self.lower_expression(else_branch, builder, is_tail)?;
                let else_end_id = builder.get_current_block_id();

                if is_tail {
                    if let Some(id) = then_end_id {
                        builder.switch_to_block(id);
                        builder.terminate(MirTerminator::Return(then_res));
                    }
                    if let Some(id) = else_end_id {
                        builder.switch_to_block(id);
                        builder.terminate(MirTerminator::Return(else_res));
                    }
                    builder.switch_to_block(9999);
                    Ok(MirOperand::Constant(MirLiteral::Nothing))
                } else {
                    let merge_id = builder.create_block();
                    let dest = builder.new_ssa();
                    
                    if let Some(id) = then_end_id {
                        builder.switch_to_block(id);
                        builder.emit(MirInstruction::Assign { dest, src: then_res });
                        builder.terminate(MirTerminator::Branch(merge_id));
                    }
                    if let Some(id) = else_end_id {
                        builder.switch_to_block(id);
                        builder.emit(MirInstruction::Assign { dest, src: else_res });
                        builder.terminate(MirTerminator::Branch(merge_id));
                    }
                    
                    builder.switch_to_block(merge_id);
                    Ok(MirOperand::Variable(dest, false))
                }
            }
            HirExpression::Block(exprs) => {
                let mut last_op = MirOperand::Constant(MirLiteral::Nothing);
                let len = exprs.len();
                for (i, expr) in exprs.iter().enumerate() {
                    let is_last = i == len - 1;
                    last_op = self.lower_expression(expr, builder, is_tail && is_last)?;
                    if builder.get_current_block_id() == Some(9999) { break; }
                }
                Ok(last_op)
            }
            HirExpression::Emit(e) => {
                let op = self.lower_expression(e, builder, false)?;
                builder.emit(MirInstruction::Emit(op));
                Ok(MirOperand::Constant(MirLiteral::Nothing))
            }
            HirExpression::Drop(e) => {
                let op = self.lower_expression(e, builder, false)?;
                if let MirOperand::Variable(ssa_var, _) = op {
                    builder.emit(MirInstruction::Drop { ssa_var, typ: OnuType::Nothing });
                }
                Ok(MirOperand::Constant(MirLiteral::Nothing))
            }
            HirExpression::Index { subject, index } => {
                let op = self.lower_expression(subject, builder, false)?;
                let dest = builder.new_ssa();
                builder.emit(MirInstruction::Index { dest, subject: op, index: *index });
                Ok(MirOperand::Variable(dest, false))
            }
            _ => Ok(MirOperand::Constant(MirLiteral::Nothing)),
        }?;
        self.log(LogLevel::Trace, &format!("Expression result: {:?}", res));
        Ok(res)
    }
}
