use onu_refactor::CompilationPipeline;
use onu_refactor::infrastructure::os::NativeOsEnvironment;
use onu_refactor::adapters::codegen::OnuCodegen;
use onu_refactor::application::options::CompilationOptions;

#[test]
fn test_module_registration() {
    let options = CompilationOptions::default();
    let env = NativeOsEnvironment::new(options.log_level);
    let codegen = OnuCodegen::new();
    let lexer = Box::new(onu_refactor::adapters::lexer::OnuLexer::new(options.log_level));
    let parser = Box::new(onu_refactor::adapters::parser::OnuParser::new(options.log_level));
    
    let pipeline = CompilationPipeline::new(env, codegen, lexer, parser, options);
    
    // Check for a core behavior
    assert!(pipeline.registry.get_signature("len").is_some());
    
    // Check for a math behavior
    assert!(pipeline.registry.get_signature("added-to").is_some());
    
    // Check for an IO extension behavior
    assert!(pipeline.registry.get_signature("broadcasts").is_some());
}
