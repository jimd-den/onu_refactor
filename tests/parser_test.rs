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
