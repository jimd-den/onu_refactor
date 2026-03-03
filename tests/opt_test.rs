use onu_refactor::application::ports::compiler_ports::CodegenPort;
use onu_refactor::CompilationPipeline;
use onu_refactor::application::options::CompilationOptions;
use onu_refactor::infrastructure::os::NativeOsEnvironment;
use onu_refactor::adapters::codegen::OnuCodegen;
use onu_refactor::domain::entities::ast::{Discourse, BehaviorHeader, Expression, ReturnType};
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
    
    // test as: { derivation: x derives-from (if true then { 1 } else { 2 }) broadcasts (x utilizes as-text) 0 }
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
    for func in &mir.functions {
        println!("Function: {}", func.name);
        for block in &func.blocks {
            println!("  Block {}: {:?}", block.id, block.terminator);
        }
    }
    pipeline.codegen.set_registry(pipeline.registry.clone());
    let ir = pipeline.codegen.generate(&mir).expect("Codegen failed");

    // RED PHASE: Currently, this should contain a phi node because of the merge block for the 'If' value
    assert!(ir.contains("phi i64"), "Generated IR should contain a phi node for branched value used in derivation");
}

#[test]
fn test_multiple_returns_direct_ret() {
    let options = CompilationOptions::default();
    let env = NativeOsEnvironment::new(options.log_level);
    let codegen = OnuCodegen::new();
    let mut pipeline = CompilationPipeline::new(env, codegen, options);
    
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
    
    // test as: if true then { 1 } else { 2 }
    // In tail position, this should result in multiple return instructions and NO phi node.
    let body = Expression::If {
        condition: Box::new(Expression::Boolean(true)),
        then_branch: Box::new(Expression::I64(1)),
        else_branch: Box::new(Expression::I64(2)),
    };
    
    let discourse = Discourse::Behavior { header, body };
    let hir = onu_refactor::application::use_cases::lowering_service::LoweringService::lower_discourse(&discourse, &pipeline.registry);
    
    let mir = pipeline.lower_mir(vec![hir]).expect("MIR lowering failed");
    for func in &mir.functions {
        println!("Function: {}", func.name);
        for block in &func.blocks {
            println!("  Block {}: {:?}", block.id, block.terminator);
        }
    }
    pipeline.codegen.set_registry(pipeline.registry.clone());
    let ir = pipeline.codegen.generate(&mir).expect("Codegen failed");

    // Once optimized, this should be false
    assert!(!ir.contains("phi i64"), "Generated IR should NOT contain a phi node for terminal branched return");
    // It should contain at least two ret instructions
    let ret_count = ir.matches("ret i64").count();
    assert!(ret_count >= 2, "Expected at least 2 ret instructions, found {}", ret_count);
}
