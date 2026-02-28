use onu_refactor::domain::entities::hir::{HirExpression, HirBinOp};
use onu_refactor::domain::entities::ast::{Expression, BinOp};
use onu_refactor::application::use_cases::lowering_service::LoweringService;
use onu_refactor::application::use_cases::registry_service::RegistryService;
use onu_refactor::domain::entities::mir::{MirInstruction, MirBinOp};
use onu_refactor::application::use_cases::mir_lowering_service::MirLoweringService;
use onu_refactor::application::ports::environment::EnvironmentPort;
use onu_refactor::application::options::LogLevel;
use onu_refactor::domain::entities::hir::{HirDiscourse, HirBehaviorHeader};

struct MockEnv;
impl EnvironmentPort for MockEnv {
    fn read_file(&self, _: &str) -> Result<String, onu_refactor::domain::entities::error::OnuError> { Ok(String::new()) }
    fn write_file(&self, _: &str, _: &str) -> Result<(), onu_refactor::domain::entities::error::OnuError> { Ok(()) }
    fn write_binary(&self, _: &str, _: &[u8]) -> Result<(), onu_refactor::domain::entities::error::OnuError> { Ok(()) }
    fn log(&self, _: LogLevel, _: &str) {}
    fn run_command(&self, _: &str, _: &[&str]) -> Result<String, onu_refactor::domain::entities::error::OnuError> { Ok(String::new()) }
}

#[test]
fn test_hir_binop_compilation() {
    let op = HirBinOp::Add;
    let expr = HirExpression::BinaryOp {
        op,
        left: Box::new(HirExpression::Literal(onu_refactor::domain::entities::hir::HirLiteral::I64(1))),
        right: Box::new(HirExpression::Literal(onu_refactor::domain::entities::hir::HirLiteral::I64(2))),
    };
    
    if let HirExpression::BinaryOp { op, .. } = expr {
        assert_eq!(op, HirBinOp::Add);
    } else {
        panic!("Expected BinaryOp");
    }
}

#[test]
fn test_lowering_binop_mapping() {
    let registry = RegistryService::new();
    let ast_expr = Expression::BinaryOp {
        op: BinOp::Add,
        left: Box::new(Expression::I64(1)),
        right: Box::new(Expression::I64(2)),
    };
    
    let hir_expr = LoweringService::lower_expression(&ast_expr, &registry);
    
    match hir_expr {
        HirExpression::BinaryOp { op, .. } => {
            assert_eq!(op, HirBinOp::Add);
        }
        _ => panic!("Expected HirExpression::BinaryOp, found {:?}", hir_expr),
    }
}

#[test]
fn test_lowering_string_to_binop() {
    let registry = RegistryService::new();
    let ast_expr = Expression::BehaviorCall {
        name: "added-to".to_string(),
        args: vec![Expression::I64(1), Expression::I64(2)],
    };
    
    let hir_expr = LoweringService::lower_expression(&ast_expr, &registry);
    
    match hir_expr {
        HirExpression::BinaryOp { op, .. } => {
            assert_eq!(op, HirBinOp::Add);
        }
        _ => panic!("Expected HirExpression::BinaryOp, found {:?}", hir_expr),
    }
}

#[test]
fn test_mir_lowering_binop_mapping() {
    let env = MockEnv;
    let registry = RegistryService::new();
    let mir_lowering = MirLoweringService::new(&env, &registry);
    
    let test_cases = vec![
        (HirBinOp::Add, MirBinOp::Add),
        (HirBinOp::Sub, MirBinOp::Sub),
        (HirBinOp::Mul, MirBinOp::Mul),
        (HirBinOp::Div, MirBinOp::Div),
        (HirBinOp::Equal, MirBinOp::Eq),
    ];

    for (hir_op, mir_op_expected) in test_cases {
        let hir_header = HirBehaviorHeader {
            name: "test".to_string(),
            is_effect: false,
            args: vec![],
            return_type: onu_refactor::domain::entities::types::OnuType::I64,
        };
        let hir_body = HirExpression::BinaryOp {
            op: hir_op,
            left: Box::new(HirExpression::Literal(onu_refactor::domain::entities::hir::HirLiteral::I64(1))),
            right: Box::new(HirExpression::Literal(onu_refactor::domain::entities::hir::HirLiteral::I64(2))),
        };
        let hir_discourses = vec![HirDiscourse::Behavior { header: hir_header, body: hir_body }];
        
        let mir_program = mir_lowering.lower_program(&hir_discourses).expect("Lowering failed");
        
        let function = &mir_program.functions[0];
        let block = &function.blocks[0];
        
        let mut found = false;
        for inst in &block.instructions {
            if let MirInstruction::BinaryOperation { op, .. } = inst {
                assert_eq!(*op, mir_op_expected);
                found = true;
                break;
            }
        }
        assert!(found, "Expected MirInstruction::BinaryOperation for {:?} not found", mir_op_expected);
    }
}
