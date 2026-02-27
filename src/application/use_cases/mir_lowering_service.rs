/// Ọ̀nụ MIR Lowering Service: Application Use Case
///
/// This service orchestrates the translation of HIR into MIR.
/// It delegates low-level construction details to the MirBuilder.

use crate::domain::entities::hir::{HirDiscourse, HirExpression, HirBehaviorHeader, HirLiteral};
use crate::domain::entities::mir::*;
use crate::domain::entities::types::OnuType;
use crate::application::use_cases::mir_builder::MirBuilder;

pub struct MirLoweringService;

impl MirLoweringService {
    pub fn new() -> Self {
        Self
    }

    pub fn lower_program(&self, discourses: &[HirDiscourse]) -> MirProgram {
        let mut functions = Vec::new();
        for d in discourses {
            if let HirDiscourse::Behavior { header, body } = d {
                functions.push(self.lower_function(header, body));
            }
        }
        MirProgram { functions }
    }

    fn lower_function(&self, header: &HirBehaviorHeader, body: &HirExpression) -> MirFunction {
        let mut builder = MirBuilder::new(header.name.clone(), header.return_type.clone());
        
        let mut mir_args = Vec::new();
        for arg in &header.args {
            let ssa_var = builder.new_ssa();
            builder.define_variable(arg.name.clone(), ssa_var);
            mir_args.push(MirArgument {
                name: arg.name.clone(),
                typ: arg.typ.clone(),
                ssa_var,
            });
        }

        let entry_id = builder.create_block();
        builder.switch_to_block(entry_id);

        let result_op = self.lower_expression(body, &mut builder, true);
        
        if let Some(_) = builder.get_current_block_id() {
            builder.terminate(MirTerminator::Return(result_op));
        }
        
        let mut func = builder.build();
        func.args = mir_args;
        func
    }

    fn lower_expression(&self, expr: &HirExpression, builder: &mut MirBuilder, is_tail: bool) -> MirOperand {
        match expr {
            HirExpression::Literal(lit) => MirOperand::Constant(match lit {
                HirLiteral::I64(n) => MirLiteral::I64(*n),
                HirLiteral::F64(n) => MirLiteral::F64(*n),
                HirLiteral::Boolean(b) => MirLiteral::Boolean(*b),
                HirLiteral::Text(s) => MirLiteral::Text(s.clone()),
                HirLiteral::Nothing => MirLiteral::Nothing,
            }),
            HirExpression::Variable(name, is_consuming) => {
                let ssa_var = builder.resolve_variable(name).unwrap_or(0);
                MirOperand::Variable(ssa_var, *is_consuming)
            }
            HirExpression::Call { name, args } => {
                let mut mir_args = Vec::new();
                for arg in args {
                    mir_args.push(self.lower_expression(arg, builder, false));
                }
                
                let bin_op = if mir_args.len() == 2 {
                    match name.as_str() {
                        "added-to" => Some(MirBinOp::Add),
                        "decreased-by" => Some(MirBinOp::Sub),
                        "scales-by" => Some(MirBinOp::Mul),
                        "partitions-by" => Some(MirBinOp::Div),
                        "matches" => Some(MirBinOp::Eq),
                        "exceeds" => Some(MirBinOp::Gt),
                        "falls-short-of" => Some(MirBinOp::Lt),
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
                    builder.emit(MirInstruction::Call { 
                        dest, 
                        name: name.clone(), 
                        args: mir_args 
                    });
                }
                MirOperand::Variable(dest, false)
            }
            HirExpression::Derivation { name, value, body, .. } => {
                let val_op = self.lower_expression(value, builder, false);
                let dest = builder.new_ssa();
                builder.emit(MirInstruction::Assign { dest, src: val_op });
                
                builder.push_scope();
                builder.define_variable(name.clone(), dest);
                let res = self.lower_expression(body, builder, is_tail);
                builder.pop_scope();
                res
            }
            HirExpression::If { condition, then_branch, else_branch } => {
                let cond_op = self.lower_expression(condition, builder, false);
                
                let then_start_id = builder.create_block();
                let else_start_id = builder.create_block();
                
                builder.terminate(MirTerminator::CondBranch { 
                    condition: cond_op, 
                    then_block: then_start_id, 
                    else_block: else_start_id 
                });
                
                // Then
                builder.switch_to_block(then_start_id);
                let then_res = self.lower_expression(then_branch, builder, is_tail);
                let then_end_id = builder.get_current_block_id();
                
                // Else
                builder.switch_to_block(else_start_id);
                let else_res = self.lower_expression(else_branch, builder, is_tail);
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
                    MirOperand::Constant(MirLiteral::Nothing)
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
                    MirOperand::Variable(dest, false)
                }
            }
            HirExpression::Block(exprs) => {
                let mut last_res = MirOperand::Constant(MirLiteral::Nothing);
                let count = exprs.len();
                for (i, e) in exprs.iter().enumerate() { 
                    let is_last = i == count - 1;
                    last_res = self.lower_expression(e, builder, is_tail && is_last);
                    
                    if builder.get_current_block_id() == Some(9999) { break; }
                }
                last_res
            }
            HirExpression::Emit(e) => {
                let op = self.lower_expression(e, builder, false);
                builder.emit(MirInstruction::Emit(op));
                MirOperand::Constant(MirLiteral::Nothing)
            }
            HirExpression::Drop(e) => {
                let op = self.lower_expression(e, builder, false);
                if let MirOperand::Variable(ssa_var, _) = op {
                    builder.emit(MirInstruction::Drop { ssa_var, typ: OnuType::Nothing });
                }
                MirOperand::Constant(MirLiteral::Nothing)
            }
            HirExpression::Index { subject, index } => {
                let op = self.lower_expression(subject, builder, false);
                let dest = builder.new_ssa();
                builder.emit(MirInstruction::Index { dest, subject: op, index: *index });
                MirOperand::Variable(dest, false)
            }
            _ => MirOperand::Constant(MirLiteral::Nothing),
        }
    }
}
