data = open('tests/benchmarktest.rs').read()

data = data.replace('let ir = std::fs::read_to_string("samples/test_ownership.ll").or_else(|_| std::fs::read_to_string("test_ownership.ll")).unwrap();', 'let ir = std::fs::read_to_string("test_ownership.ll").unwrap();')
data = data.replace('let ir = std::fs::read_to_string("samples/hello_world.ll").or_else(|_| std::fs::read_to_string("hello_world.ll")).unwrap();', 'let ir = std::fs::read_to_string("hello_world.ll").unwrap();')
data = data.replace('let ir = std::fs::read_to_string("samples/fibonacci.ll").or_else(|_| std::fs::read_to_string("fibonacci.ll")).unwrap();', 'let ir = std::fs::read_to_string("fibonacci.ll").unwrap();')

open('tests/benchmarktest.rs', 'w').write(data)
