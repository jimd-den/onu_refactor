/// Ọ̀nụ HIR: Domain Entities
///
/// This module defines the High-level Intermediate Representation.
/// HIR is used for semantic analysis, ownership checking, and liveness analysis.

use crate::domain::entities::types::OnuType;

#[derive(Debug, Clone, PartialEq)]
pub enum HirDiscourse {
    Module { name: String, concern: String },
    Shape { name: String, behaviors: Vec<HirBehaviorHeader> },
    Behavior { header: HirBehaviorHeader, body: HirExpression },
}

#[derive(Debug, Clone, PartialEq)]
pub struct HirBehaviorHeader {
    pub name: String,
    pub is_effect: bool,
    pub args: Vec<HirArgument>,
    pub return_type: OnuType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HirArgument {
    pub name: String,
    pub typ: OnuType,
    pub is_observation: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HirExpression {
    Literal(HirLiteral),
    Variable(String, bool), // (name, is_consuming)
    Call { name: String, args: Vec<HirExpression> },
    Derivation { 
        name: String, 
        typ: OnuType, 
        value: Box<HirExpression>, 
        body: Box<HirExpression> 
    },
    If { 
        condition: Box<HirExpression>, 
        then_branch: Box<HirExpression>, 
        else_branch: Box<HirExpression> 
    },
    ActsAs { 
        subject: Box<HirExpression>, 
        shape: String 
    },
    Tuple(Vec<HirExpression>),
    Index { 
        subject: Box<HirExpression>, 
        index: usize 
    },
    Block(Vec<HirExpression>),
    Emit(Box<HirExpression>),
    Drop(Box<HirExpression>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum HirLiteral {
    I64(i64),
    F64(u64), // Bits
    Boolean(bool),
    Text(String),
    Nothing,
}
