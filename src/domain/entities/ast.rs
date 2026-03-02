/// Ọ̀nụ AST: Domain Entities
///
/// This module defines the structural units of the Ọ̀nụ language.
/// These are pure data structures representing the "Proposition" and "Discourse."

use crate::domain::entities::types::OnuType;

#[derive(Debug, Clone, PartialEq)]
pub enum Discourse {
    Module { name: String, concern: String },
    Shape { 
        name: String, 
        fields: Vec<Argument>,
        behaviors: Vec<BehaviorHeader> 
    },
    Behavior { header: BehaviorHeader, body: Expression },
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    I8(i8), I16(i16), I32(i32), I64(i64), I128(i128),
    U8(u8), U16(u16), U32(u32), U64(u64), U128(u128),
    F32(u32), F64(u64), // Bits for floats
    Boolean(bool),
    BinaryOp {
        op: BinOp,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Text(String),
    Identifier(String),
    Nothing,
    Tuple(Vec<Expression>),
    Array(Vec<Expression>),
    Matrix {
        rows: usize,
        cols: usize,
        data: Vec<Expression>,
    },
    Derivation {
        name: String,
        type_info: Option<TypeInfo>,
        value: Box<Expression>,
        body: Box<Expression>,
    },
    ActsAs {
        subject: Box<Expression>,
        shape: String,
    },
    BehaviorCall { name: String, args: Vec<Expression> },
    If {
        condition: Box<Expression>,
        then_branch: Box<Expression>,
        else_branch: Box<Expression>,
    },
    Block(Vec<Expression>),
    Emit(Box<Expression>),
    Broadcasts(Box<Expression>),
    Drop(Box<Expression>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeInfo {
    pub onu_type: OnuType,
    pub display_name: String,
    pub via_role: Option<String>,
    pub is_observation: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Argument {
    pub name: String,
    pub type_info: TypeInfo,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnType(pub OnuType);

#[derive(Debug, Clone, PartialEq)]
pub struct BehaviorHeader {
    pub name: String,
    pub is_effect: bool,
    pub intent: String,
    pub takes: Vec<Argument>,
    pub delivers: ReturnType,
    pub diminishing: Option<String>,
    pub skip_termination_check: bool,
}
