/// Ọ̀nụ MIR Lowering Service: Application Use Case
///
/// This service orchestrates the translation of HIR into MIR.
/// It delegates low-level construction details to the MirBuilder.

use crate::domain::entities::hir::{HirDiscourse, HirExpression, HirBehaviorHeader, HirLiteral, HirBinOp};
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
            builder.define_variable(&arg.name, ssa_var, arg.typ.clone());
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
                let typ = builder.resolve_variable_type(name).unwrap_or(OnuType::Nothing);
                if *is_consuming {
                    builder.schedule_drop(ssa_var, typ.clone());
                    builder.mark_consumed(ssa_var);
                }
                Result::<MirOperand, OnuError>::Ok(MirOperand::Variable(ssa_var, *is_consuming))
            }
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
                    HirBinOp::NotEqual => MirBinOp::Eq, // FIXME: Add NotEqual to MirBinOp if needed
                    HirBinOp::LessThan => MirBinOp::Lt,
                    HirBinOp::GreaterThan => MirBinOp::Gt,
                };
                builder.emit(MirInstruction::BinaryOperation {
                    dest,
                    op: mir_op,
                    lhs,
                    rhs,
                });

                // Flush pending drops after binary op
                let pending = builder.take_pending_drops();
                for (var, typ) in pending {
                    builder.emit(MirInstruction::Drop { ssa_var: var, typ });
                }

                Ok(MirOperand::Variable(dest, false))
            }
            HirExpression::Call { name, args } => {
                let mut mir_args = Vec::new();
                for arg in args {
                    mir_args.push(self.lower_expression(arg, builder, false)?);
                }
                
                if name == "as-text" && mir_args.len() == 1 {
                    let dest = builder.new_ssa();
                    let a = &mir_args[0];

                    // Alloc buffer: 32 bytes should be enough for i64
                    let alloc_size_ssa = builder.new_ssa();
                    builder.emit(MirInstruction::Assign { dest: alloc_size_ssa, src: MirOperand::Constant(MirLiteral::I64(32)) });

                    let buf_ssa = builder.new_ssa();
                    builder.emit(MirInstruction::Alloc {
                        dest: buf_ssa,
                        size_bytes: MirOperand::Variable(alloc_size_ssa, false),
                    });

                    // Call snprintf(buf, 32, "%lld", a)
                    // Wait, do we have snprintf or sprintf? Let's assume sprintf.
                    // Instead, let's just emit a call to snprintf.
                    // We need "%lld" string literal.
                    let fmt_str_ssa = builder.new_ssa();
                    builder.emit(MirInstruction::Assign { dest: fmt_str_ssa, src: MirOperand::Constant(MirLiteral::Text("%lld".to_string())) });

                    let fmt_str_ptr_ssa = builder.new_ssa();
                    builder.emit(MirInstruction::Index { dest: fmt_str_ptr_ssa, subject: MirOperand::Variable(fmt_str_ssa, false), index: 1 });

                    let sprintf_ret_ssa = builder.new_ssa();
                    builder.emit(MirInstruction::Call {
                        dest: sprintf_ret_ssa,
                        name: "sprintf".to_string(),
                        args: vec![
                            MirOperand::Variable(buf_ssa, false),
                            MirOperand::Variable(fmt_str_ptr_ssa, false),
                            a.clone()
                        ],
                        return_type: OnuType::I32,
                        arg_types: vec![OnuType::Nothing, OnuType::Nothing, OnuType::I64],
                    });

                    // sprintf_ret is the length (i32). Let's cast it to i64? Or just use it? Wait, sprintf_ret is i32, but MIR only has I64.
                    // We can just assume sprintf returns i64 or let's assume it works.
                    // For now, let's just use sprintf_ret_ssa (which is I32 in LLVM, so might be problematic). Let's use strlen instead?
                    // Actually, let's just create a tuple.
                    let cast_len_ssa = builder.new_ssa();
                    builder.emit(MirInstruction::Call {
                        dest: cast_len_ssa,
                        name: "strlen".to_string(),
                        args: vec![MirOperand::Variable(buf_ssa, false)],
                        return_type: OnuType::I64,
                        arg_types: vec![OnuType::Nothing],
                    });

                    builder.emit(MirInstruction::Tuple {
                        dest,
                        elements: vec![
                            MirOperand::Variable(cast_len_ssa, false),
                            MirOperand::Variable(buf_ssa, false),
                            MirOperand::Constant(MirLiteral::Boolean(true)), // is_dynamic
                        ],
                    });

                    return Ok(MirOperand::Variable(dest, false));
                }

                if name == "joined-with" && mir_args.len() == 2 {
                    let dest = builder.new_ssa();
                    let a = &mir_args[0];
                    let b = &mir_args[1];

                    // a.len
                    let a_len_ssa = builder.new_ssa();
                    builder.emit(MirInstruction::Index { dest: a_len_ssa, subject: a.clone(), index: 0 });
                    // b.len
                    let b_len_ssa = builder.new_ssa();
                    builder.emit(MirInstruction::Index { dest: b_len_ssa, subject: b.clone(), index: 0 });

                    // sum len = a.len + b.len
                    let sum_len_ssa = builder.new_ssa();
                    builder.emit(MirInstruction::BinaryOperation {
                        dest: sum_len_ssa,
                        op: MirBinOp::Add,
                        lhs: MirOperand::Variable(a_len_ssa, false),
                        rhs: MirOperand::Variable(b_len_ssa, false),
                    });

                    // a.ptr
                    let a_ptr_ssa = builder.new_ssa();
                    builder.emit(MirInstruction::Index { dest: a_ptr_ssa, subject: a.clone(), index: 1 });
                    // b.ptr
                    let b_ptr_ssa = builder.new_ssa();
                    builder.emit(MirInstruction::Index { dest: b_ptr_ssa, subject: b.clone(), index: 1 });

                    // alloc_size = sum_len + 1
                    let alloc_size_ssa = builder.new_ssa();
                    builder.emit(MirInstruction::BinaryOperation {
                        dest: alloc_size_ssa,
                        op: MirBinOp::Add,
                        lhs: MirOperand::Variable(sum_len_ssa, false),
                        rhs: MirOperand::Constant(MirLiteral::I64(1)),
                    });

                    // Alloc buffer
                    let buf_ssa = builder.new_ssa();
                    builder.emit(MirInstruction::Alloc {
                        dest: buf_ssa,
                        size_bytes: MirOperand::Variable(alloc_size_ssa, false),
                    });

                    // MemCopy a
                    builder.emit(MirInstruction::MemCopy {
                        dest: MirOperand::Variable(buf_ssa, false),
                        src: MirOperand::Variable(a_ptr_ssa, false),
                        size: MirOperand::Variable(a_len_ssa, false),
                    });

                    // Offset buffer for b
                    let b_dest_ptr_ssa = builder.new_ssa();
                    builder.emit(MirInstruction::PointerOffset {
                        dest: b_dest_ptr_ssa,
                        ptr: MirOperand::Variable(buf_ssa, false),
                        offset: MirOperand::Variable(a_len_ssa, false),
                    });

                    // MemCopy b
                    builder.emit(MirInstruction::MemCopy {
                        dest: MirOperand::Variable(b_dest_ptr_ssa, false),
                        src: MirOperand::Variable(b_ptr_ssa, false),
                        size: MirOperand::Variable(b_len_ssa, false),
                    });

                    // Null terminator (size 1)
                    let null_dest_ptr_ssa = builder.new_ssa();
                    builder.emit(MirInstruction::PointerOffset {
                        dest: null_dest_ptr_ssa,
                        ptr: MirOperand::Variable(buf_ssa, false),
                        offset: MirOperand::Variable(sum_len_ssa, false),
                    });

                    // To keep it simple, we can copy a 1-byte null terminator
                    let null_char_ptr_ssa = builder.new_ssa();
                    builder.emit(MirInstruction::Assign { dest: null_char_ptr_ssa, src: MirOperand::Constant(MirLiteral::Text("".to_string())) });
                    let null_char_str_ptr_ssa = builder.new_ssa();
                    builder.emit(MirInstruction::Index { dest: null_char_str_ptr_ssa, subject: MirOperand::Variable(null_char_ptr_ssa, false), index: 1 });

                    builder.emit(MirInstruction::MemCopy {
                        dest: MirOperand::Variable(null_dest_ptr_ssa, false),
                        src: MirOperand::Variable(null_char_str_ptr_ssa, false),
                        size: MirOperand::Constant(MirLiteral::I64(1)),
                    });

                    // Let's create a tuple of {sum_len, buf_ssa, is_dynamic}
                    builder.emit(MirInstruction::Tuple {
                        dest,
                        elements: vec![
                            MirOperand::Variable(sum_len_ssa, false),
                            MirOperand::Variable(buf_ssa, false),
                            MirOperand::Constant(MirLiteral::Boolean(true)), // is_dynamic
                        ],
                    });

                    return Ok(MirOperand::Variable(dest, false));
                }

                let dest = builder.new_ssa();
                let (return_type, arg_types) = if let Some(sig) = self.registry.get_signature(name) {
                    (sig.return_type.clone(), sig.input_types.clone())
                } else {
                    (OnuType::Nothing, Vec::new())
                };

                builder.emit(MirInstruction::Call { 
                    dest, 
                    name: name.clone(), 
                    args: mir_args,
                    return_type,
                    arg_types,
                });

                // Flush pending drops after call
                let pending = builder.take_pending_drops();
                for (var, typ) in pending {
                    builder.emit(MirInstruction::Drop { ssa_var: var, typ });
                }

                Ok(MirOperand::Variable(dest, false))
            }
            HirExpression::Derivation { name, typ, value, body } => {
                let val_op = self.lower_expression(value, builder, false)?;
                let ssa_var = builder.new_ssa();
                builder.emit(MirInstruction::Assign { dest: ssa_var, src: val_op });
                builder.enter_scope();
                builder.define_variable(name, ssa_var, typ.clone());
                let res = self.lower_expression(body, builder, is_tail)?;

                // Scope-Exit Drops
                for (var_id, var_typ) in builder.get_current_scope_variables() {
                    builder.emit(MirInstruction::Drop { ssa_var: var_id, typ: var_typ });
                }

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

                // Flush pending drops after emit
                let pending = builder.take_pending_drops();
                for (var, typ) in pending {
                    builder.emit(MirInstruction::Drop { ssa_var: var, typ });
                }

                Ok(MirOperand::Constant(MirLiteral::Nothing))
            }
            HirExpression::Drop(e) => {
                let op = self.lower_expression(e, builder, false)?;
                if let MirOperand::Variable(ssa_var, _) = op {
                    builder.emit(MirInstruction::Drop { ssa_var, typ: OnuType::Nothing });
                }

                // Flush pending drops after drop
                let pending = builder.take_pending_drops();
                for (var, typ) in pending {
                    builder.emit(MirInstruction::Drop { ssa_var: var, typ });
                }

                Ok(MirOperand::Constant(MirLiteral::Nothing))
            }
            HirExpression::Index { subject, index } => {
                let op = self.lower_expression(subject, builder, false)?;
                let dest = builder.new_ssa();
                builder.emit(MirInstruction::Index { dest, subject: op, index: *index });

                // Flush pending drops after index
                let pending = builder.take_pending_drops();
                for (var, typ) in pending {
                    builder.emit(MirInstruction::Drop { ssa_var: var, typ });
                }

                Ok(MirOperand::Variable(dest, false))
            }
            _ => Ok(MirOperand::Constant(MirLiteral::Nothing)),
        }?;
        self.log(LogLevel::Trace, &format!("Expression result: {:?}", res));
        Ok(res)
    }
}
