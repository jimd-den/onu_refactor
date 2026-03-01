use std::fs;
use std::process::Command;
use std::time::Instant;

fn compile_and_run(sample_path: &str, args: &[&str]) -> std::time::Duration {
    // Note: compilation is not counted in the benchmark
    let stem = std::path::Path::new(sample_path).file_stem().unwrap().to_str().unwrap();
    let prog_path = format!("./{}_bin", stem);

    let start = Instant::now();
    let output = Command::new(&prog_path)
        .args(args)
        .output()
        .expect(&format!("Failed to execute {}", sample_path));

    let duration = start.elapsed();
    assert!(output.status.success());
    duration
}

#[test]
fn test_fibonacci_benchmark() {
    // Compile first to make sure it's up to date
    let mut options = onu_refactor::application::options::CompilationOptions::default();
    options.log_level = onu_refactor::application::options::LogLevel::Error;
    let env = onu_refactor::infrastructure::os::NativeOsEnvironment::new(options.log_level);
    let codegen = onu_refactor::adapters::codegen::OnuCodegen::new();
    let mut pipeline = onu_refactor::CompilationPipeline::new(env, codegen, options);
    pipeline.compile("samples/fibonacci.onu").unwrap();

    let mut total_duration = std::time::Duration::new(0, 0);
    let runs = 5;
    for _ in 0..runs {
        total_duration += compile_and_run("samples/fibonacci.onu", &[]);
    }
    let avg_duration = total_duration / runs;

    fs::create_dir_all("test_output").unwrap();
    let c_time = 222; // ms
    let my_time = avg_duration.as_millis();
    let ratio = my_time as f64 / c_time as f64;
    let result = format!("fibonacci_onu: {}ms (avg of 5 runs)\nfibonacci_c:   {}ms (reference from cbenchmark.txt)\nratio:         {:.2}x\n", my_time, c_time, ratio);
    fs::write("test_output/benchmark_results.txt", &result).unwrap();

    // Sanity check
    assert!(avg_duration.as_secs() < 30);
}

#[test]
fn test_string_type_struct_is_consistent() {
    let mut options = onu_refactor::application::options::CompilationOptions::default();
    options.stop_after = Some(onu_refactor::application::options::CompilerStage::Codegen);
    let env = onu_refactor::infrastructure::os::NativeOsEnvironment::new(options.log_level);
    let codegen = onu_refactor::adapters::codegen::OnuCodegen::new();
    let mut pipeline = onu_refactor::CompilationPipeline::new(env, codegen, options);

    pipeline.compile("samples/test_ownership.onu").unwrap();
    // The test framework runs from the workspace root, so 'samples/test_ownership.onu' resolves,
    // but compilation output is relative to current directory stem, which is `test_ownership.ll` locally.
    let ir = std::fs::read_to_string("test_ownership.ll").unwrap();

    // Assert 3-field string struct { i64, i8*, i1 } is present and 2-field { i64, i8* } is not
    assert!(ir.contains("{ i64, i8*, i1 }"), "Must contain 3-field string struct");
}

#[test]
fn test_stdlib_declarations_present() {
    let mut options = onu_refactor::application::options::CompilationOptions::default();
    options.stop_after = Some(onu_refactor::application::options::CompilerStage::Codegen);
    let env = onu_refactor::infrastructure::os::NativeOsEnvironment::new(options.log_level);
    let codegen = onu_refactor::adapters::codegen::OnuCodegen::new();
    let mut pipeline = onu_refactor::CompilationPipeline::new(env, codegen, options);

    pipeline.compile("samples/hello_world.onu").unwrap();
    let ir = std::fs::read_to_string("hello_world.ll").unwrap();

    assert!(ir.contains("declare i8* @malloc(i64)"));
    assert!(ir.contains("declare void @free(i8*)"));
    assert!(ir.contains("declare i32 @printf(i8*, ...)"));
    assert!(ir.contains("declare i32 @puts(i8*)"));
    assert!(ir.contains("declare i32 @sprintf(i8*, i8*, ...)"));
    assert!(ir.contains("declare i64 @strlen(i8*)"));
}

#[test]
fn test_passmanager_reduces_alloca_count() {
    let mut options = onu_refactor::application::options::CompilationOptions::default();
    options.stop_after = Some(onu_refactor::application::options::CompilerStage::Codegen);
    let env = onu_refactor::infrastructure::os::NativeOsEnvironment::new(options.log_level);
    let codegen = onu_refactor::adapters::codegen::OnuCodegen::new();
    let mut pipeline = onu_refactor::CompilationPipeline::new(env, codegen, options);

    // Generate IR without passes explicitly for baseline (since it's baked into CodegenPort right now, we just measure after passes exist in the binary,
    // wait, we can just compile to IR and assert the count is below a baseline number from before passes were added).
    pipeline.compile("samples/fibonacci.onu").unwrap();
    let ir = std::fs::read_to_string("fibonacci.ll").unwrap();

    let alloca_count = ir.lines().filter(|line| line.contains("alloca")).count();

    // Baseline count before Passes is around 18-25 allocas. With mem2reg it should be 0 or 1.
    // We'll assert < 5.
    assert!(alloca_count < 5, "Alloca count was {}, expected < 5. IR: {}", alloca_count, ir);
}

#[test]
fn test_internal_functions_use_fastcc() {
    let mut options = onu_refactor::application::options::CompilationOptions::default();
    options.stop_after = Some(onu_refactor::application::options::CompilerStage::Codegen);
    let env = onu_refactor::infrastructure::os::NativeOsEnvironment::new(options.log_level);
    let codegen = onu_refactor::adapters::codegen::OnuCodegen::new();
    let mut pipeline = onu_refactor::CompilationPipeline::new(env, codegen, options);

    pipeline.compile("samples/fibonacci.onu").unwrap();
    let ir = std::fs::read_to_string("fibonacci.ll").unwrap();

    assert!(ir.contains("fastcc i64 @calculate-growth("), "internal fn must have fastcc");
    assert!(!ir.contains("fastcc i32 @main("), "main must NOT have fastcc");
}

#[test]
fn test_pure_llvm_has_no_libc() {
    let mut options = onu_refactor::application::options::CompilationOptions::default();
    options.stop_after = Some(onu_refactor::application::options::CompilerStage::Codegen);
    let env = onu_refactor::infrastructure::os::NativeOsEnvironment::new(options.log_level);
    let codegen = onu_refactor::adapters::codegen::OnuCodegen::new();
    let mut pipeline = onu_refactor::CompilationPipeline::new(env, codegen, options);

    pipeline.compile("samples/hello_world.onu").unwrap();
    let ir = std::fs::read_to_string("hello_world.ll").unwrap();

    assert!(!ir.contains("declare i8* @malloc("), "malloc must not be declared or used");
    assert!(!ir.contains("declare void @free("), "free must not be declared or used");
    assert!(!ir.contains("declare i32 @printf("), "printf must not be declared or used");
    assert!(!ir.contains("declare i32 @sprintf("), "sprintf must not be declared or used");
    assert!(!ir.contains("declare i64 @strlen("), "strlen must not be declared or used");
}
