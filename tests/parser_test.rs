use onu_refactor::adapters::parser::OnuParser;
use onu_refactor::application::ports::compiler_ports::{LexerPort, Token};
use onu_refactor::application::options::LogLevel;
use onu_refactor::application::use_cases::registry_service::RegistryService;

#[test]
fn test_parser_scan_headers() {
    let tokens = vec![
        Token::TheBehaviorCalled,
        Token::Identifier("test-behavior".to_string()),
        Token::Takes,
        Token::Operator(":".to_string()),
        Token::Identifier("a".to_string()),
        Token::Identifier("integer".to_string()),
        Token::Called,
        Token::Identifier("x".to_string()),
        Token::Delivers,
        Token::Operator(":".to_string()),
        Token::Nothing,
        Token::As,
        Token::Operator(":".to_string()),
        Token::Nothing,
    ];
    
    let mut registry = RegistryService::new();
    let parser = OnuParser::new(LogLevel::Debug);
    
    parser.scan_headers(&tokens, &mut registry).expect("Scan headers failed");
    
    let sig = registry.get_signature("test-behavior");
    assert!(sig.is_some(), "Signature not registered after scan_headers");
    assert_eq!(sig.unwrap().input_types.len(), 1);
}

#[test]
fn test_arity_bounded_utilizes() {
    let tokens = vec![
        Token::Literal(onu_refactor::application::ports::compiler_ports::Literal::Integer(1)),
        Token::Utilizes,
        Token::Identifier("test-behavior".to_string()),
        Token::Literal(onu_refactor::application::ports::compiler_ports::Literal::Integer(2)),
        Token::Literal(onu_refactor::application::ports::compiler_ports::Literal::Integer(3)),
    ];
    
    let mut registry = RegistryService::new();
    registry.symbols_mut().add_name("test-behavior", 2);
    
    let parser = OnuParser::new(LogLevel::Debug);
    let discourses = parser.parse_with_registry(tokens, &mut registry).expect("Parsing failed");
    
    // First discourse should be a behavior if it was wrapped, but here it's just a raw expression block
    // Actually ParserPort::parse returns Result<Vec<Discourse>, OnuError>
}

#[test]
fn test_parser_fails_on_missing_argument_type() {
    let source = "the behavior called test with intent: nothing takes: my_arg called my_arg delivers: nothing as: nothing";
    let lexer = onu_refactor::adapters::lexer::OnuLexer::new(onu_refactor::application::options::LogLevel::Error);
    let tokens = lexer.lex(source).unwrap();
    let parser = onu_refactor::adapters::parser::OnuParser::new(onu_refactor::application::options::LogLevel::Error);
    let mut registry = onu_refactor::application::use_cases::registry_service::RegistryService::new();

    let result = parser.parse_with_registry(tokens, &mut registry);
    assert!(result.is_err(), "Parser should fail when argument type is implicitly fallen back to i64 (missing 'a' or 'an' strictly typed keyword)");
}

#[test]
fn test_parser_fails_on_missing_return_type() {
    let source = "the behavior called test with intent: nothing takes: nothing delivers: some_implicit_thing as: nothing";
    let lexer = onu_refactor::adapters::lexer::OnuLexer::new(onu_refactor::application::options::LogLevel::Error);
    let tokens = lexer.lex(source).unwrap();
    let parser = onu_refactor::adapters::parser::OnuParser::new(onu_refactor::application::options::LogLevel::Error);
    let mut registry = onu_refactor::application::use_cases::registry_service::RegistryService::new();

    let result = parser.parse_with_registry(tokens, &mut registry);
    assert!(result.is_err(), "Parser should fail when return type is missing proper definition (implicit fallback to Nothing/i64)");
}

// ============================================================================
// Matrix literal parsing tests
// ============================================================================

#[test]
fn test_parse_matrix_literal_2x2() {
    use onu_refactor::application::ports::compiler_ports::Literal;
    use onu_refactor::domain::entities::ast::Expression;

    let tokens = vec![
        Token::Delimiter('['),
        Token::Literal(Literal::Integer(1)),
        Token::Delimiter(','),
        Token::Literal(Literal::Integer(2)),
        Token::Delimiter(';'),
        Token::Literal(Literal::Integer(3)),
        Token::Delimiter(','),
        Token::Literal(Literal::Integer(4)),
        Token::Delimiter(']'),
    ];

    let (expr, consumed) =
        onu_refactor::adapters::parser::matrix_parser::parse_matrix(&tokens)
            .expect("Matrix parsing failed");

    assert_eq!(consumed, tokens.len());
    if let Expression::Matrix { rows, cols, data } = expr {
        assert_eq!(rows, 2);
        assert_eq!(cols, 2);
        assert_eq!(data.len(), 4);
    } else {
        panic!("Expected Expression::Matrix");
    }
}

#[test]
fn test_parse_matrix_via_lexer_and_parser() {
    use onu_refactor::adapters::lexer::OnuLexer;
    use onu_refactor::application::ports::compiler_ports::LexerPort;
    use onu_refactor::domain::entities::ast::{Discourse, Expression};

    let source = r#"
the-module-called Matrices with-concern: testing

the-behavior-called make-matrix
    with-intent: return a matrix constant
    takes: nothing
    delivers: nothing
    as:
        [1, 2; 3, 4]
"#;

    let lexer = OnuLexer::new(LogLevel::Error);
    let tokens = lexer.lex(source).expect("Lexing failed");

    let parser = OnuParser::new(LogLevel::Error);
    let mut registry = RegistryService::new();
    let discourses = parser
        .parse_with_registry(tokens, &mut registry)
        .expect("Parsing failed");

    // Walk the AST to find the matrix expression in the behavior body
    let found_matrix = discourses.iter().any(|d| {
        if let Discourse::Behavior { body, .. } = d {
            matches!(body, Expression::Matrix { .. })
        } else {
            false
        }
    });
    assert!(found_matrix, "Expected a Matrix expression in behavior body");
}

// ============================================================================
// SVO syntax parsing tests
// ============================================================================

#[test]
fn test_svo_write_parses_to_emit() {
    use onu_refactor::adapters::lexer::OnuLexer;
    use onu_refactor::application::ports::compiler_ports::LexerPort;
    use onu_refactor::domain::entities::ast::{Discourse, Expression};

    let source = r#"
the-module-called SvoTest with-concern: testing

the-effect-behavior-called say-hello
    with-intent: write a greeting to console
    takes: nothing
    delivers: nothing
    as:
        write "hello" to console
"#;

    let lexer = OnuLexer::new(LogLevel::Error);
    let tokens = lexer.lex(source).expect("Lexing failed");

    let parser = OnuParser::new(LogLevel::Error);
    let mut registry = RegistryService::new();
    let discourses = parser
        .parse_with_registry(tokens, &mut registry)
        .expect("Parsing failed");

    let found_emit = discourses.iter().any(|d| {
        if let Discourse::Behavior { body, .. } = d {
            matches!(body, Expression::Emit(_))
        } else {
            false
        }
    });
    assert!(found_emit, "Expected an Emit (write) expression in behavior body");
}

#[test]
fn test_svo_read_parses_to_receives_line() {
    use onu_refactor::adapters::lexer::OnuLexer;
    use onu_refactor::application::ports::compiler_ports::LexerPort;
    use onu_refactor::domain::entities::ast::{Discourse, Expression};

    let source = r#"
the-module-called SvoRead with-concern: testing

the-effect-behavior-called get-input
    with-intent: read a line from console
    takes: nothing
    delivers: nothing
    as:
        read line from console
"#;

    let lexer = OnuLexer::new(LogLevel::Error);
    let tokens = lexer.lex(source).expect("Lexing failed");

    let parser = OnuParser::new(LogLevel::Error);
    let mut registry = RegistryService::new();
    let discourses = parser
        .parse_with_registry(tokens, &mut registry)
        .expect("Parsing failed");

    let found_read = discourses.iter().any(|d| {
        if let Discourse::Behavior { body, .. } = d {
            if let Expression::BehaviorCall { name, .. } = body {
                name == "receives-line"
            } else {
                false
            }
        } else {
            false
        }
    });
    assert!(found_read, "Expected a 'receives-line' BehaviorCall expression");
}

// ============================================================================
// Fault-tolerant parser tests (Red/Green TDD)
// ============================================================================

/// Hypothesis: adding `parse_tolerant()` with `error_recovery::synchronize()`
/// lets the parser catch *multiple* syntax errors in a single pass without
/// aborting on the first one.
///
/// Red: before the implementation this test would fail because the old
///      `parse_with_registry` returns `Err` on the first bad token, so only
///      one error would be reported.
/// Green: after the implementation we collect all errors and return them in
///        a `Vec<Diagnostic>`.
#[test]
fn test_fault_tolerant_parser_collects_two_errors_without_crashing() {
    use onu_refactor::adapters::parser::OnuParser;
    use onu_refactor::adapters::lexer::OnuLexer;
    use onu_refactor::application::options::LogLevel;
    use onu_refactor::application::ports::compiler_ports::LexerPort;
    use onu_refactor::application::use_cases::registry_service::RegistryService;
    use onu_refactor::domain::entities::error::Severity;

    // This program contains two deliberately bad behavior declarations.
    // The first has a missing argument type indicator (`integer` instead of
    // `a integer`), and the second also lacks a proper return type.
    // The fault-tolerant parser must survive both without panicking.
    let source = r#"
the-module-called ErrorTest with-concern: testing

the-behavior-called bad-one
    with-intent: deliberately broken
    takes: nothing
    delivers: some_implicit_thing
    as: nothing

the-behavior-called bad-two
    with-intent: also broken
    takes: nothing
    delivers: another_implicit_thing
    as: nothing

the-behavior-called good-one
    with-intent: a well-formed behavior for contrast
    takes: nothing
    delivers: nothing
    as: nothing
"#;

    let lexer = OnuLexer::new(LogLevel::Error);
    let tokens = lexer.lex(source).expect("Lexing should not fail");

    let parser = OnuParser::new(LogLevel::Error);
    let mut registry = RegistryService::new();

    let (discourses, diagnostics) = parser.parse_tolerant(tokens, &mut registry);

    // The tolerant parser must not panic and must report at least 2 diagnostics.
    assert!(
        diagnostics.len() >= 2,
        "Expected at least 2 error diagnostics, got {}: {:?}",
        diagnostics.len(),
        diagnostics
    );

    // Every collected diagnostic should be an error-level diagnostic.
    for d in &diagnostics {
        assert_eq!(
            d.severity,
            Severity::Error,
            "Unexpected non-error diagnostic: {:?}",
            d
        );
    }

    // The tolerant parser should still have recovered the module and the
    // good behavior.
    let found_module = discourses.iter().any(|d| {
        matches!(d, onu_refactor::domain::entities::ast::Discourse::Module { name, .. } if name == "ErrorTest")
    });
    assert!(found_module, "Module 'ErrorTest' should have been parsed despite errors");
}

#[test]
fn test_fault_tolerant_parser_returns_empty_diagnostics_for_valid_input() {
    use onu_refactor::adapters::parser::OnuParser;
    use onu_refactor::adapters::lexer::OnuLexer;
    use onu_refactor::application::options::LogLevel;
    use onu_refactor::application::ports::compiler_ports::LexerPort;
    use onu_refactor::application::use_cases::registry_service::RegistryService;

    let source = r#"
the-module-called ValidTest with-concern: testing

the-behavior-called identity
    with-intent: return the input unchanged
    takes:
        an integer called n
    delivers: an integer
    as:
        n
"#;

    let lexer = OnuLexer::new(LogLevel::Error);
    let tokens = lexer.lex(source).expect("Lexing should not fail");
    let parser = OnuParser::new(LogLevel::Error);
    let mut registry = RegistryService::new();
    let (discourses, diagnostics) = parser.parse_tolerant(tokens, &mut registry);

    assert!(
        diagnostics.is_empty(),
        "No diagnostics expected for valid input, got: {:?}",
        diagnostics
    );
    assert_eq!(discourses.len(), 2, "Should have parsed 2 discourses (module + behavior)");
}

// ============================================================================
// Semantic analyzer integration tests (unused-variable warnings)
// ============================================================================

#[test]
fn test_semantic_analyzer_emits_warning_for_unused_variable() {
    use onu_refactor::adapters::parser::OnuParser;
    use onu_refactor::adapters::lexer::OnuLexer;
    use onu_refactor::application::options::LogLevel;
    use onu_refactor::application::ports::compiler_ports::LexerPort;
    use onu_refactor::application::use_cases::registry_service::RegistryService;
    use onu_refactor::application::use_cases::lowering_service::LoweringService;
    use onu_refactor::application::use_cases::analyzer::SemanticAnalyzer;
    use onu_refactor::domain::entities::error::Severity;

    let source = r#"
the-module-called SemanticTest with-concern: testing

the-behavior-called unused-var-demo
    with-intent: define a variable but never use it
    takes: nothing
    delivers: an integer
    as:
        derivation: unused derives-from a integer 99
        42
"#;

    let lexer = OnuLexer::new(LogLevel::Error);
    let tokens = lexer.lex(source).expect("Lexing should succeed");
    let parser = OnuParser::new(LogLevel::Error);
    let mut registry = RegistryService::new();
    let discourses = parser.parse_with_registry(tokens, &mut registry).expect("Parsing should succeed");

    // Lower AST → HIR
    let hir_discourses: Vec<_> = discourses.iter()
        .map(|d| LoweringService::lower_discourse(d, &registry))
        .collect();

    let diagnostics = SemanticAnalyzer::analyze(&hir_discourses);

    let warnings: Vec<_> = diagnostics.iter()
        .filter(|d| d.severity == Severity::Warning)
        .collect();

    assert!(
        !warnings.is_empty(),
        "Expected an unused-variable warning, but none were emitted"
    );
    assert!(
        warnings[0].message.contains("unused"),
        "Warning message should mention 'unused': {}",
        warnings[0].message
    );
}

#[test]
fn test_semantic_analyzer_no_warning_when_variable_used() {
    use onu_refactor::adapters::parser::OnuParser;
    use onu_refactor::adapters::lexer::OnuLexer;
    use onu_refactor::application::options::LogLevel;
    use onu_refactor::application::ports::compiler_ports::LexerPort;
    use onu_refactor::application::use_cases::registry_service::RegistryService;
    use onu_refactor::application::use_cases::lowering_service::LoweringService;
    use onu_refactor::application::use_cases::analyzer::SemanticAnalyzer;
    use onu_refactor::domain::entities::error::Severity;

    let source = r#"
the-module-called SemanticTest with-concern: testing

the-behavior-called used-var-demo
    with-intent: use the variable
    takes: nothing
    delivers: an integer
    as:
        derivation: x derives-from a integer 10
        x
"#;

    let lexer = OnuLexer::new(LogLevel::Error);
    let tokens = lexer.lex(source).expect("Lexing should succeed");
    let parser = OnuParser::new(LogLevel::Error);
    let mut registry = RegistryService::new();
    let discourses = parser.parse_with_registry(tokens, &mut registry).expect("Parsing should succeed");

    let hir_discourses: Vec<_> = discourses.iter()
        .map(|d| LoweringService::lower_discourse(d, &registry))
        .collect();

    let diagnostics = SemanticAnalyzer::analyze(&hir_discourses);
    let warnings: Vec<_> = diagnostics.iter()
        .filter(|d| d.severity == Severity::Warning)
        .collect();

    assert!(
        warnings.is_empty(),
        "Expected no unused-variable warnings, but got: {:?}",
        warnings
    );
}
