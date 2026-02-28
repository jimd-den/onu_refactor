use onu_refactor::CompilationPipeline;
use onu_refactor::infrastructure::os::NativeOsEnvironment;
use onu_refactor::adapters::codegen::OnuCodegen;
use onu_refactor::application::options::CompilationOptions;

#[test]
fn test_module_registration() {
    let options = CompilationOptions::default();
    let env = NativeOsEnvironment::new(options.log_level);
    let codegen = OnuCodegen::new();
    
    let pipeline = CompilationPipeline::new(env, codegen, options);
    
    // Check for a core behavior
    assert!(pipeline.registry.get_signature("len").is_some());
    
    // Check for a math behavior
    assert!(pipeline.registry.get_signature("added-to").is_some());
    
    // Check for an IO extension behavior
    assert!(pipeline.registry.get_signature("broadcasts").is_some());
}
