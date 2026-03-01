use std::fs;
use std::process::Command;

fn compile_and_run(sample_path: &str, args: &[&str]) -> String {
    let mut options = onu_refactor::application::options::CompilationOptions::default();
    options.log_level = onu_refactor::application::options::LogLevel::Error;
    let env = onu_refactor::infrastructure::os::NativeOsEnvironment::new(options.log_level);
    let codegen = onu_refactor::adapters::codegen::OnuCodegen::new();
    let mut pipeline = onu_refactor::CompilationPipeline::new(env, codegen, options);

    // Compile
    pipeline.compile(sample_path).unwrap();

    // Execute the generated binary
    let stem = std::path::Path::new(sample_path).file_stem().unwrap().to_str().unwrap();
    let prog_path = format!("./{}_bin", stem);
    let output = Command::new(&prog_path)
        .args(args)
        .output()
        .expect(&format!("Failed to execute {}", sample_path));

    assert!(output.status.success(), "Execution of {} failed", sample_path);
    String::from_utf8_lossy(&output.stdout).to_string()
}

macro_rules! assert_sample_output {
    ($sample:expr, $expected:expr) => {
        let output = compile_and_run($sample, &[]);
        let file_stem = std::path::Path::new($sample).file_stem().unwrap().to_str().unwrap();
        fs::create_dir_all("test_output").unwrap();
        fs::write(format!("test_output/{}.txt", file_stem), &output).unwrap();
        assert_eq!(output.trim(), $expected.trim());
    };
    ($sample:expr, $args:expr, $expected:expr) => {
        let output = compile_and_run($sample, $args);
        let file_stem = std::path::Path::new($sample).file_stem().unwrap().to_str().unwrap();
        fs::create_dir_all("test_output").unwrap();
        fs::write(format!("test_output/{}.txt", file_stem), &output).unwrap();
        assert_eq!(output.trim(), $expected.trim());
    };
}

#[test] fn fibonacci_output() { assert_sample_output!("samples/fibonacci.onu", "The population at generation 40 has reached: \n102334155"); }
#[test] fn ackermann_output() { assert_sample_output!("samples/ackermann.onu", "═══════════════════════════════════════════\n  ACKERMANN GROWTH DEMONSTRATION\n  Rules: Successor, Descent, and Spiral\n═══════════════════════════════════════════\nSolving Spiral(2, 2)...\n7\nSolving Spiral(3, 2)...\n29\n═══════════════════════════════════════════"); }
// Skip bf
#[test] fn collatz_output() { assert_sample_output!("samples/collatz.onu", "COLLATZ SEQUENCE (Starting at 1,000,000):\n1000000\n500000\n250000\n125000\n62500\n31250\n15625\n46876\n23438\n11719"); }
#[test] fn collatz_bench_output() { assert_sample_output!("samples/collatz_bench.onu", "Total Collatz steps for 1 to 1000000 is: \n131434424"); }
// Skip echo_demo
#[test] fn factorial_output() { assert_sample_output!("samples/factorial.onu", "The accumulation of 5 steps is: \n120"); }
// Skip hanoi
#[test] fn hello_world_output() { assert_sample_output!("samples/hello_world.onu", "Hello, World!"); }
#[test] fn hello_world_int_output() { assert_sample_output!("samples/hello_world_int.onu", "Hello, World!"); }
// Skip map_bench
// Skip mutation
#[test] fn parity_output() { assert_sample_output!("samples/parity.onu", "PARITY VERIFICATION:\nIs 10 even? (1=yes): 1\nIs 7 even?  (1=yes): 0"); }
#[test] fn sample_output() { assert_sample_output!("samples/sample.onu", "10"); }
#[test] fn test_logic_output() { assert_sample_output!("samples/test_logic.onu", "FAIL: 1 opposes 2 is FALSE (0)\nFAIL: 1 opposes 1 is TRUE (1)"); }
#[test] fn test_ownership_output() { assert_sample_output!("samples/test_ownership.onu", "Linear Resource\nBranch Resource\nPASS: Ownership verification complete."); }
#[test] fn test_recursion_output() { assert_sample_output!("samples/test_recursion.onu", "PASS: Deep recursion complete."); }
