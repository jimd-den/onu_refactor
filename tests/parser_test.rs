use onu_refactor::adapters::parser::OnuParser;
use onu_refactor::application::ports::compiler_ports::{ParserPort, LexerPort, Token};
use onu_refactor::application::options::LogLevel;
use onu_refactor::application::use_cases::registry_service::RegistryService;
use onu_refactor::domain::entities::ast::Expression;

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
    let discourses = parser.parse_with_registry(tokens, &registry).expect("Parsing failed");
    
    // First discourse should be a behavior if it was wrapped, but here it's just a raw expression block
    // Actually ParserPort::parse returns Result<Vec<Discourse>, OnuError>
}

#[test]
fn test_parser_fails_on_missing_argument_type() {
    let source = "the behavior called test with intent: nothing takes: my_arg called my_arg delivers: nothing as: nothing";
    let mut lexer = onu_refactor::adapters::lexer::OnuLexer::new(onu_refactor::application::options::LogLevel::Error);
    let tokens = lexer.lex(source).unwrap();
    let mut parser = onu_refactor::adapters::parser::OnuParser::new(onu_refactor::application::options::LogLevel::Error);
    let mut registry = onu_refactor::application::use_cases::registry_service::RegistryService::new();

    let result = parser.parse_with_registry(tokens, &mut registry);
    assert!(result.is_err(), "Parser should fail when argument type is implicitly fallen back to i64 (missing 'a' or 'an' strictly typed keyword)");
}

#[test]
fn test_parser_fails_on_missing_return_type() {
    let source = "the behavior called test with intent: nothing takes: nothing delivers: some_implicit_thing as: nothing";
    let mut lexer = onu_refactor::adapters::lexer::OnuLexer::new(onu_refactor::application::options::LogLevel::Error);
    let tokens = lexer.lex(source).unwrap();
    let mut parser = onu_refactor::adapters::parser::OnuParser::new(onu_refactor::application::options::LogLevel::Error);
    let mut registry = onu_refactor::application::use_cases::registry_service::RegistryService::new();

    let result = parser.parse_with_registry(tokens, &mut registry);
    assert!(result.is_err(), "Parser should fail when return type is missing proper definition (implicit fallback to Nothing/i64)");
}
