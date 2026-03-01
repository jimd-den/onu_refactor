use onu_refactor::application::use_cases::lowering_service::LoweringService;
use onu_refactor::domain::entities::ast::{Discourse, BehaviorHeader, Expression, ReturnType};
use onu_refactor::domain::entities::hir::{HirDiscourse, HirExpression, HirLiteral};
use onu_refactor::domain::entities::types::OnuType;
use onu_refactor::CompilationPipeline;
use onu_refactor::application::options::{CompilationOptions, LogLevel};
use onu_refactor::infrastructure::os::NativeOsEnvironment;
use onu_refactor::application::ports::compiler_ports::CodegenPort;
use onu_refactor::application::use_cases::registry_service::RegistryService;
use onu_refactor::domain::entities::mir::MirProgram;
use onu_refactor::domain::entities::error::OnuError;

struct MockCodegen;
impl CodegenPort for MockCodegen {
    fn generate(&self, _: &MirProgram) -> Result<String, OnuError> { Ok(String::new()) }
    fn set_registry(&mut self, _: RegistryService) {}
}

#[test]
fn test_pipeline_stages() {
    let options = CompilationOptions::default();
    let env = NativeOsEnvironment::new(options.log_level);
    let codegen = MockCodegen;
    let mut pipeline = CompilationPipeline::new(env, codegen, options);

    let source = "the module called Test with concern: nothing
the behavior called run with intent: nothing as: nothing";

    // These methods exist
    let tokens = pipeline.lex(source).expect("Lexing failed");
    assert!(!tokens.is_empty());

    pipeline.scan_headers(&tokens).expect("Scanning failed");

    let ast = pipeline.parse(tokens).expect("Parsing failed");
    assert!(!ast.is_empty());

    let hir = pipeline.lower_hir(ast).expect("HIR lowering failed");
    assert!(!hir.is_empty());

    let mir = pipeline.lower_mir(hir).expect("MIR lowering failed");
    assert!(!mir.functions.is_empty());

    let ir = pipeline.emit_ir(mir).expect("IR emission failed");
    assert!(ir.is_empty()); // Mock returns empty
}

#[test]
fn test_service_injection() {
    let _registry = RegistryService::new();
    let _env = NativeOsEnvironment::new(LogLevel::Debug);

    // This should fail to compile once we change the signature
    // let _analysis = onu_refactor::application::use_cases::analysis_service::AnalysisService::new(&env, &registry);
    // let _module = onu_refactor::application::use_cases::module_service::ModuleService::new(&env, LogLevel::Debug);
}

#[test]
fn test_ownership_rule_port_inversion() {
    struct MockRegistry;
    impl onu_refactor::domain::entities::registry::BehaviorRegistryPort for MockRegistry {
        fn get_signature(&self, _: &str) -> Option<&onu_refactor::domain::entities::registry::BehaviorSignature> { None }
    }
    
    let port = MockRegistry;
    let _rule = onu_refactor::domain::rules::ownership::OwnershipRule::new(&port);
}

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

#[test]
fn test_stdlib_op_registry_dispatches() {
    use onu_refactor::application::use_cases::stdlib::StdlibOpRegistry;
    let registry = StdlibOpRegistry::new();
    assert!(registry.get("joined-with").is_some());
    assert!(registry.get("as-text").is_some());
    assert!(registry.get("len").is_some());
}
