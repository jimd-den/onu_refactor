/// Ọ̀nụ Lowering Service: Application Use Case
///
/// This service translates Domain Entities (AST) into more
/// detailed Domain Entities (HIR).

use crate::domain::entities::ast::{Discourse, Expression, BehaviorHeader, Argument};
use crate::domain::entities::hir::{HirDiscourse, HirExpression, HirBehaviorHeader, HirArgument, HirLiteral};
use crate::domain::entities::types::OnuType;
use crate::application::use_cases::registry_service::RegistryService;

pub struct LoweringService;

impl LoweringService {
    pub fn lower_discourse(discourse: &Discourse, registry: &RegistryService) -> HirDiscourse {
        match discourse {
            Discourse::Module { name, concern } => HirDiscourse::Module {
                name: name.clone(),
                concern: concern.clone(),
            },
            Discourse::Behavior { header, body } => {
                let mut hir_header = Self::lower_header(header);
                if header.name == "main" || header.name == "run" {
                    // Inject synthetic arguments for standard CLI entry point
                    hir_header.args.insert(0, HirArgument { name: "__argc".to_string(), typ: OnuType::I32, is_observation: false });
                    hir_header.args.insert(1, HirArgument { name: "__argv".to_string(), typ: OnuType::U64, is_observation: false });
                }
                HirDiscourse::Behavior {
                    header: hir_header,
                    body: Self::lower_expression(body, registry),
                }
            },
            Discourse::Shape { name, behaviors } => HirDiscourse::Shape { 
                name: name.clone(), 
                behaviors: behaviors.iter().map(Self::lower_header).collect() 
            },
        }
    }

    fn lower_header(header: &BehaviorHeader) -> HirBehaviorHeader {
        HirBehaviorHeader {
            name: header.name.clone(),
            is_effect: header.is_effect,
            args: header.takes.iter().map(Self::lower_argument).collect(),
            return_type: header.delivers.0.clone(),
        }
    }

    fn lower_argument(arg: &Argument) -> HirArgument {
        HirArgument {
            name: arg.name.clone(),
            typ: arg.type_info.onu_type.clone(),
            is_observation: arg.type_info.is_observation,
        }
    }

    fn lower_expression(expr: &Expression, registry: &RegistryService) -> HirExpression {
        match expr {
            Expression::I128(n) => HirExpression::Literal(HirLiteral::I64(*n as i64)),
            Expression::F64(n) => HirExpression::Literal(HirLiteral::F64(*n)),
            Expression::Boolean(b) => HirExpression::Literal(HirLiteral::Boolean(*b)),
            Expression::Text(s) => HirExpression::Literal(HirLiteral::Text(s.clone())),
            Expression::Nothing => HirExpression::Literal(HirLiteral::Nothing),
            Expression::Identifier(s) => {
                // If the identifier is a known behavior, it's a zero-arity call
                if registry.get_signature(s).is_some() {
                    HirExpression::Call { name: s.clone(), args: vec![] }
                } else {
                    HirExpression::Variable(s.clone(), false)
                }
            },
            Expression::BehaviorCall { name, args } => HirExpression::Call {
                name: name.clone(),
                args: args.iter().map(|e| Self::lower_expression(e, registry)).collect(),
            },
            Expression::Derivation { name, type_info, value, body, .. } => HirExpression::Derivation {
                name: name.clone(),
                typ: type_info.as_ref().map(|ti| ti.onu_type.clone()).unwrap_or(OnuType::I64),
                value: Box::new(Self::lower_expression(value, registry)),
                body: Box::new(Self::lower_expression(body, registry)),
            },
            Expression::If { condition, then_branch, else_branch } => HirExpression::If {
                condition: Box::new(Self::lower_expression(condition, registry)),
                then_branch: Box::new(Self::lower_expression(then_branch, registry)),
                else_branch: Box::new(Self::lower_expression(else_branch, registry)),
            },
            Expression::Block(exprs) => HirExpression::Block(
                exprs.iter().map(|e| Self::lower_expression(e, registry)).collect()
            ),
            Expression::Emit(e) => HirExpression::Emit(Box::new(Self::lower_expression(e, registry))),
            Expression::Tuple(exprs) => HirExpression::Tuple(
                exprs.iter().map(|e| Self::lower_expression(e, registry)).collect()
            ),
            Expression::ActsAs { subject, .. } => Self::lower_expression(subject, registry),
            Expression::Broadcasts(e) => HirExpression::Emit(Box::new(Self::lower_expression(e, registry))),
            Expression::Drop(e) => HirExpression::Drop(Box::new(Self::lower_expression(e, registry))),
            Expression::Matrix { .. } => HirExpression::Literal(HirLiteral::Nothing), 
            Expression::I8(n) => HirExpression::Literal(HirLiteral::I64(*n as i64)),
            Expression::I16(n) => HirExpression::Literal(HirLiteral::I64(*n as i64)),
            Expression::I32(n) => HirExpression::Literal(HirLiteral::I64(*n as i64)),
            Expression::I64(n) => HirExpression::Literal(HirLiteral::I64(*n)),
            Expression::U8(n) => HirExpression::Literal(HirLiteral::I64(*n as i64)),
            Expression::U16(n) => HirExpression::Literal(HirLiteral::I64(*n as i64)),
            Expression::U32(n) => HirExpression::Literal(HirLiteral::I64(*n as i64)),
            Expression::U64(n) => HirExpression::Literal(HirLiteral::I64(*n as i64)),
            Expression::F32(n) => HirExpression::Literal(HirLiteral::F64(f32::from_bits(*n) as f64 as u64)),
            _ => HirExpression::Literal(HirLiteral::Nothing),
        }
    }
}
