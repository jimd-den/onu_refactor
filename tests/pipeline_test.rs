use onu_refactor::application::use_cases::lowering_service::LoweringService;
use onu_refactor::domain::entities::ast::{Discourse, BehaviorHeader, Expression, ReturnType};
use onu_refactor::domain::entities::hir::{HirDiscourse, HirExpression, HirLiteral};
use onu_refactor::domain::entities::types::OnuType;

#[test]
fn test_synthetic_argument_injection() {
    let header = BehaviorHeader {
        name: "main".to_string(),
        is_effect: true,
        intent: "Test".to_string(),
        takes: vec![],
        delivers: ReturnType(OnuType::Nothing),
        diminishing: None,
        skip_termination_check: false,
    };
    let body = Expression::Nothing;
    let discourse = Discourse::Behavior { header, body };
    
    let registry = onu_refactor::application::use_cases::registry_service::RegistryService::new();
    let hir = LoweringService::lower_discourse(&discourse, &registry);
    
    if let HirDiscourse::Behavior { header, .. } = hir {
        assert_eq!(header.args.len(), 2);
        assert_eq!(header.args[0].name, "__argc");
        assert_eq!(header.args[1].name, "__argv");
    } else {
        panic!("Expected Behavior");
    }
}

#[test]
fn test_broadcasts_lowering() {
    let header = BehaviorHeader {
        name: "test".to_string(),
        is_effect: true,
        intent: "Test".to_string(),
        takes: vec![],
        delivers: ReturnType(OnuType::Nothing),
        diminishing: None,
        skip_termination_check: false,
    };
    let body = Expression::Emit(Box::new(Expression::Text("Hello".to_string())));
    let discourse = Discourse::Behavior { header, body };
    
    let registry = onu_refactor::application::use_cases::registry_service::RegistryService::new();
    let hir = LoweringService::lower_discourse(&discourse, &registry);
    
    if let HirDiscourse::Behavior { body, .. } = hir {
        match body {
            HirExpression::Emit(inner) => {
                if let HirExpression::Literal(HirLiteral::Text(s)) = *inner {
                    assert_eq!(s, "Hello");
                } else {
                    panic!("Expected Text literal in Emit");
                }
            }
            _ => panic!("Expected Emit expression in HIR"),
        }
    } else {
        panic!("Expected Behavior");
    }
}

#[test]
fn test_drop_lowering() {
    let header = BehaviorHeader {
        name: "test".to_string(),
        is_effect: true,
        intent: "Test".to_string(),
        takes: vec![],
        delivers: ReturnType(OnuType::Nothing),
        diminishing: None,
        skip_termination_check: false,
    };
    let body = Expression::Drop(Box::new(Expression::Identifier("x".to_string())));
    let discourse = Discourse::Behavior { header, body };
    
    let registry = onu_refactor::application::use_cases::registry_service::RegistryService::new();
    let hir = LoweringService::lower_discourse(&discourse, &registry);
    
    if let HirDiscourse::Behavior { body, .. } = hir {
        match body {
            HirExpression::Drop(inner) => {
                if let HirExpression::Variable(name, _) = *inner {
                    assert_eq!(name, "x");
                } else {
                    panic!("Expected Variable in Drop");
                }
            }
            _ => panic!("Expected Drop expression in HIR"),
        }
    } else {
        panic!("Expected Behavior");
    }
}
