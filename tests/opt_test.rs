use onu_refactor::application::ports::compiler_ports::CodegenPort;
use onu_refactor::CompilationPipeline;
use onu_refactor::application::options::CompilationOptions;
use onu_refactor::infrastructure::os::NativeOsEnvironment;
use onu_refactor::adapters::codegen::OnuCodegen;
use onu_refactor::domain::entities::ast::{Discourse, BehaviorHeader, Expression, ReturnType, Argument, TypeInfo};
use onu_refactor::domain::entities::types::OnuType;
use onu_refactor::domain::entities::registry::BehaviorSignature;

#[test]
fn test_multiple_returns_phi_detection() {
    let options = CompilationOptions::default();
    let env = NativeOsEnvironment::new(options.log_level);
    let codegen = OnuCodegen::new();
    let mut pipeline = CompilationPipeline::new(env, codegen, options);
    
    // Register symbols
    pipeline.registry.symbols_mut().add_signature("test", BehaviorSignature {
        return_type: OnuType::I64,
        input_types: vec![],
        arg_is_observation: vec![],
    });

    let header = BehaviorHeader {
        name: "test".to_string(),
        is_effect: false,
        intent: "Test".to_string(),
        takes: vec![], 
        delivers: ReturnType(OnuType::I64),
        diminishing: None,
        skip_termination_check: false,
    };
    
    let body = Expression::Block(vec![
        Expression::Derivation {
            name: "x".to_string(),
            type_info: None,
            value: Box::new(Expression::If {
                condition: Box::new(Expression::Boolean(true)),
                then_branch: Box::new(Expression::I64(1)),
                else_branch: Box::new(Expression::I64(2)),
            }),
            body: Box::new(Expression::Broadcasts(Box::new(
                Expression::BehaviorCall {
                    name: "as-text".to_string(),
                    args: vec![Expression::Identifier("x".to_string())],
                }
            ))),
        },
        Expression::I64(0),
    ]);
    
    let discourse = Discourse::Behavior { header, body };
    let hir = onu_refactor::application::use_cases::lowering_service::LoweringService::lower_discourse(&discourse, &pipeline.registry);
    
    let mir = pipeline.lower_mir(vec![hir]).expect("MIR lowering failed");
    pipeline.codegen.set_registry(pipeline.registry.clone());
    let ir = pipeline.codegen.generate(&mir).expect("Codegen failed");

    assert!(ir.contains("phi i64"), "Generated IR should contain a phi node for branched value used in derivation");
}

#[test]
fn test_ackermann_specialization() {
    let options = CompilationOptions::default();
    let env = NativeOsEnvironment::new(options.log_level);
    let codegen = OnuCodegen::new();
    let mut pipeline = CompilationPipeline::new(env, codegen, options);
    
    // Register symbols
    pipeline.registry.symbols_mut().add_signature("test_op", BehaviorSignature {
        return_type: OnuType::I64,
        input_types: vec![OnuType::I64, OnuType::I64],
        arg_is_observation: vec![false, false],
    });

    let header = BehaviorHeader {
        name: "test_op".to_string(),
        is_effect: false,
        intent: "Test".to_string(),
        takes: vec![
            Argument { name: "a".to_string(), type_info: TypeInfo { onu_type: OnuType::I64, display_name: "integer".to_string(), via_role: None, is_observation: false } },
            Argument { name: "b".to_string(), type_info: TypeInfo { onu_type: OnuType::I64, display_name: "integer".to_string(), via_role: None, is_observation: false } }
        ],
        delivers: ReturnType(OnuType::I64),
        diminishing: None,
        skip_termination_check: false,
    };
    
    // test_op(a, b) as: a decreased-by b
    let body = Expression::BehaviorCall {
        name: "decreased-by".to_string(),
        args: vec![
            Expression::Identifier("a".to_string()),
            Expression::Identifier("b".to_string())
        ],
    };
    
    let discourse = Discourse::Behavior { header, body };
    let hir = onu_refactor::application::use_cases::lowering_service::LoweringService::lower_discourse(&discourse, &pipeline.registry);
    let mir = pipeline.lower_mir(vec![hir]).expect("MIR lowering failed");
    pipeline.codegen.set_registry(pipeline.registry.clone());
    let ir = pipeline.codegen.generate(&mir).expect("Codegen failed");

    assert!(ir.contains("sub i64") || ir.contains("add i64"), "Generated IR should contain primitive sub/add instruction");
    assert!(!ir.contains("call fastcc i64 @decreased-by"), "Generated IR should NOT contain a call to decreased-by behavior");
}

#[test]
fn test_comparison_specialization() {
    let options = CompilationOptions::default();
    let env = NativeOsEnvironment::new(options.log_level);
    let codegen = OnuCodegen::new();
    let mut pipeline = CompilationPipeline::new(env, codegen, options);
    
    // Register symbols
    pipeline.registry.symbols_mut().add_signature("test_cmp", BehaviorSignature {
        return_type: OnuType::Boolean,
        input_types: vec![OnuType::I64, OnuType::I64],
        arg_is_observation: vec![false, false],
    });

    let header = BehaviorHeader {
        name: "test_cmp".to_string(),
        is_effect: false,
        intent: "Test".to_string(),
        takes: vec![
            Argument { name: "a".to_string(), type_info: TypeInfo { onu_type: OnuType::I64, display_name: "integer".to_string(), via_role: None, is_observation: false } },
            Argument { name: "b".to_string(), type_info: TypeInfo { onu_type: OnuType::I64, display_name: "integer".to_string(), via_role: None, is_observation: false } }
        ],
        delivers: ReturnType(OnuType::Boolean),
        diminishing: None,
        skip_termination_check: false,
    };
    
    // test_cmp(a, b) as: a exceeds b
    let body = Expression::BehaviorCall {
        name: "exceeds".to_string(),
        args: vec![
            Expression::Identifier("a".to_string()),
            Expression::Identifier("b".to_string())
        ],
    };
    
    let discourse = Discourse::Behavior { header, body };
    let hir = onu_refactor::application::use_cases::lowering_service::LoweringService::lower_discourse(&discourse, &pipeline.registry);
    let mir = pipeline.lower_mir(vec![hir]).expect("MIR lowering failed");
    pipeline.codegen.set_registry(pipeline.registry.clone());
    let ir = pipeline.codegen.generate(&mir).expect("Codegen failed");

    // 'exceeds' should be 'icmp sgt'
    assert!(ir.contains("icmp sgt"), "Generated IR should contain icmp sgt for exceeds");
}
