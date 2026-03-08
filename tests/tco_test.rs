use onu_refactor::CompilationPipeline;
use onu_refactor::application::options::CompilationOptions;
use onu_refactor::infrastructure::os::NativeOsEnvironment;
use onu_refactor::application::ports::compiler_ports::CodegenPort;
use onu_refactor::domain::entities::mir::{MirProgram, MirInstruction};
use onu_refactor::domain::entities::ast::{Discourse, BehaviorHeader, Expression, ReturnType};
use onu_refactor::domain::entities::types::OnuType;
use onu_refactor::domain::entities::registry::BehaviorSignature;
use onu_refactor::domain::entities::error::OnuError;
use onu_refactor::application::use_cases::registry_service::RegistryService;

struct MockCodegen;
impl CodegenPort for MockCodegen {
    fn generate(&self, _: &MirProgram) -> Result<String, OnuError> { Ok(String::new()) }
    fn set_registry(&mut self, _: RegistryService) {}
}

#[test]
fn test_mir_call_has_tco_metadata() {
    let options = CompilationOptions::default();
    let env = NativeOsEnvironment::new(options.log_level);
    let codegen = MockCodegen;
    let lexer = Box::new(onu_refactor::adapters::lexer::OnuLexer::new(options.log_level));
    let parser = Box::new(onu_refactor::adapters::parser::OnuParser::new(options.log_level));
    let mut pipeline = CompilationPipeline::new(env, codegen, lexer, parser, options);
    
    // Register the recursive function
    pipeline.registry.symbols_mut().add_signature(
        "rec",
        BehaviorSignature {
            return_type: OnuType::Nothing,
            input_types: vec![],
            arg_is_observation: vec![],
        }
    );

    let header = BehaviorHeader {
        name: "rec".to_string(),
        is_effect: false,
        intent: "Test".to_string(),
        takes: vec![],
        delivers: ReturnType(OnuType::Nothing),
        diminishing: vec![],
        memo_cache_size: None,
        skip_termination_check: false,
    };
    
    // rec as: { rec }
    let body = Expression::Block(vec![
        Expression::BehaviorCall { 
            name: "rec".to_string(), 
            args: vec![] 
        }
    ]);
    
    let discourse = Discourse::Behavior { header, body };
    let hir = onu_refactor::application::use_cases::lowering_service::LoweringService::lower_discourse(&discourse, &pipeline.registry);
    
    let mir = pipeline.lower_mir(vec![hir]).expect("MIR lowering failed");
    
    let func = &mir.functions[0];
    // Find the call in the last block
    let mut call_found = false;
    for block in &func.blocks {
        for inst in &block.instructions {
            if let MirInstruction::Call { name, is_tail_call, .. } = inst {
                if name == "rec" {
                    call_found = true;
                    assert!(is_tail_call, "Expected is_tail_call to be true for recursive call in tail position");
                }
            }
        }
    }
    
    assert!(call_found, "Expected a Call instruction to 'rec'");
}

#[test]
fn test_if_tail_call_propagation() {
    let options = CompilationOptions::default();
    let env = NativeOsEnvironment::new(options.log_level);
    let codegen = MockCodegen;
    let lexer = Box::new(onu_refactor::adapters::lexer::OnuLexer::new(options.log_level));
    let parser = Box::new(onu_refactor::adapters::parser::OnuParser::new(options.log_level));
    let mut pipeline = CompilationPipeline::new(env, codegen, lexer, parser, options);
    
    pipeline.registry.symbols_mut().add_signature(
        "rec",
        BehaviorSignature {
            return_type: OnuType::Nothing,
            input_types: vec![],
            arg_is_observation: vec![],
        }
    );

    let header = BehaviorHeader {
        name: "rec".to_string(),
        is_effect: false,
        intent: "Test".to_string(),
        takes: vec![],
        delivers: ReturnType(OnuType::Nothing),
        diminishing: vec![],
        memo_cache_size: None,
        skip_termination_check: false,
    };
    
    // rec as: if true then { rec } else { nothing }
    let body = Expression::If {
        condition: Box::new(Expression::Boolean(true)),
        then_branch: Box::new(Expression::Block(vec![
            Expression::BehaviorCall { name: "rec".to_string(), args: vec![] }
        ])),
        else_branch: Box::new(Expression::Nothing),
    };
    
    let discourse = Discourse::Behavior { header, body };
    let hir = onu_refactor::application::use_cases::lowering_service::LoweringService::lower_discourse(&discourse, &pipeline.registry);
    
    let mir = pipeline.lower_mir(vec![hir]).expect("MIR lowering failed");
    
    let func = &mir.functions[0];
    let mut call_found = false;
    for block in &func.blocks {
        for inst in &block.instructions {
            if let MirInstruction::Call { name, is_tail_call, .. } = inst {
                if name == "rec" {
                    call_found = true;
                    assert!(is_tail_call, "Expected is_tail_call to be true for recursive call in IF branch tail position");
                }
            }
        }
    }
    
    assert!(call_found, "Expected a Call instruction to 'rec' in IF branch");
}

#[test]
fn test_nested_if_tail_call_propagation() {
    let options = CompilationOptions::default();
    let env = NativeOsEnvironment::new(options.log_level);
    let codegen = MockCodegen;
    let lexer = Box::new(onu_refactor::adapters::lexer::OnuLexer::new(options.log_level));
    let parser = Box::new(onu_refactor::adapters::parser::OnuParser::new(options.log_level));
    let mut pipeline = CompilationPipeline::new(env, codegen, lexer, parser, options);
    
    pipeline.registry.symbols_mut().add_signature(
        "rec",
        BehaviorSignature {
            return_type: OnuType::Nothing,
            input_types: vec![],
            arg_is_observation: vec![],
        }
    );

    let header = BehaviorHeader {
        name: "rec".to_string(),
        is_effect: false,
        intent: "Test".to_string(),
        takes: vec![],
        delivers: ReturnType(OnuType::Nothing),
        diminishing: vec![],
        memo_cache_size: None,
        skip_termination_check: false,
    };
    
    // rec as: if true then { if false then { nothing } else { rec } } else { nothing }
    let body = Expression::If {
        condition: Box::new(Expression::Boolean(true)),
        then_branch: Box::new(Expression::If {
            condition: Box::new(Expression::Boolean(false)),
            then_branch: Box::new(Expression::Nothing),
            else_branch: Box::new(Expression::BehaviorCall { name: "rec".to_string(), args: vec![] }),
        }),
        else_branch: Box::new(Expression::Nothing),
    };
    
    let discourse = Discourse::Behavior { header, body };
    let hir = onu_refactor::application::use_cases::lowering_service::LoweringService::lower_discourse(&discourse, &pipeline.registry);
    
    let mir = pipeline.lower_mir(vec![hir]).expect("MIR lowering failed");
    
    let func = &mir.functions[0];
    let mut call_found = false;
    for block in &func.blocks {
        for inst in &block.instructions {
            if let MirInstruction::Call { name, is_tail_call, .. } = inst {
                if name == "rec" {
                    call_found = true;
                    assert!(is_tail_call, "Expected is_tail_call to be true for recursive call in nested IF branch tail position");
                }
            }
        }
    }
    
    assert!(call_found, "Expected a Call instruction to 'rec' in nested IF branch");
}
