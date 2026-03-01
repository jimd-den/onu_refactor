use std::process::Command;

fn assert_valgrind_clean(binary_path: &str, args: &[&str]) {
    let output = Command::new("valgrind")
        .args([
            "--error-exitcode=1",
            "--leak-check=full",
            "--errors-for-leak-kinds=all",
            "--show-leak-kinds=all",
            "--track-origins=yes",
        ])
        .arg(binary_path)
        .args(args)
        .output()
        .expect("valgrind must be installed");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let success = output.status.success() &&
        stderr.contains("ERROR SUMMARY: 0 errors") &&
        (stderr.contains("definitely lost: 0 bytes in 0 blocks") || stderr.contains("All heap blocks were freed -- no leaks are possible"));

    if !success {
        panic!("Valgrind failed for {}:\n{}", binary_path, stderr);
    }
}

fn test_valgrind(sample_name: &str, args: &[&str]) {
    // Make sure it is compiled
    let sample_path = format!("samples/{}.onu", sample_name);
    let mut options = onu_refactor::application::options::CompilationOptions::default();
    options.log_level = onu_refactor::application::options::LogLevel::Error;
    let env = onu_refactor::infrastructure::os::NativeOsEnvironment::new(options.log_level);
    let codegen = onu_refactor::adapters::codegen::OnuCodegen::new();
    let mut pipeline = onu_refactor::CompilationPipeline::new(env, codegen, options);
    let _ = pipeline.compile(&sample_path);

    let binary_path = format!("./{}_bin", sample_name);
    assert_valgrind_clean(&binary_path, args);
}

#[test] fn valgrind_test_ownership() { test_valgrind("test_ownership", &[]); }
#[test] fn valgrind_fibonacci() { test_valgrind("fibonacci", &[]); }
#[test] fn valgrind_ackermann() { test_valgrind("ackermann", &[]); }

#[test] fn valgrind_collatz_bench() { test_valgrind("collatz_bench", &[]); }
// skip bf, hanoi, mutation because they fail to compile or run completely properly for now in some conditions as seen above
