use onu_refactor::adapters::parser::OnuParser;
use onu_refactor::adapters::lexer::OnuLexer;
use onu_refactor::application::ports::compiler_ports::{LexerPort, Token, Literal};
use onu_refactor::application::options::LogLevel;
use onu_refactor::application::use_cases::registry_service::RegistryService;
use onu_refactor::domain::entities::ast::Discourse;

/// Scenario 1: Quoted Intent
/// Non-coders should see clearly what the intent is.
/// This should allow the 'as:' keyword to be on the same line without being swallowed.
#[test]
fn test_intent_quoted_isolation() {
    let source = r#"
the behavior called test-quoted
    with intent: "This is clear documentation" as: nothing
"#;
    let lexer = OnuLexer::new(LogLevel::Error);
    let tokens = lexer.lex(source).unwrap();
    let mut registry = RegistryService::new();
    let parser = OnuParser::new(LogLevel::Error);
    
    let discourses = parser.parse_with_registry(tokens, &mut registry).expect("Should isolate quoted intent");
    
    if let Discourse::Behavior { header, .. } = &discourses[0] {
        assert_eq!(header.intent, "This is clear documentation");
    } else {
        panic!("Expected behavior discourse");
    }
}

/// Scenario 2: Unquoted Intent (Stop at Keyword)
/// If no quotations are provided, the parser must NOT swallow the whole line.
/// It must stop when it sees a keyword like 'takes:', 'delivers:', or 'as:'.
#[test]
fn test_intent_unquoted_stops_at_keyword() {
    let source = r#"
the behavior called test-unquoted
    with intent: this prose ends here as: nothing
"#;
    let lexer = OnuLexer::new(LogLevel::Error);
    let tokens = lexer.lex(source).unwrap();
    let mut registry = RegistryService::new();
    let parser = OnuParser::new(LogLevel::Error);
    
    // CURRENT FAILURE: Swallows 'as: nothing' into the intent.
    let discourses = parser.parse_with_registry(tokens, &mut registry).expect("Should stop prose at 'as:'");
    
    if let Discourse::Behavior { header, .. } = &discourses[0] {
        assert_eq!(header.intent, "this prose ends here");
    }
}

/// Scenario 3: Article 'a' as Variable (Lookahead Guard)
/// Non-coders write 'a peanut' but coders might want a variable called 'a'.
/// The parser should only treat 'a' as a type-indicator if followed by a Type Name.
#[test]
fn test_article_lookahead_disambiguation() {
    let source = r#"
the behavior called article-logic
    takes: nothing
    delivers: an integer
    as:
        derivation: a derives-from 1
        a added-to 1
"#;
    let lexer = OnuLexer::new(LogLevel::Error);
    let tokens = lexer.lex(source).unwrap();
    let mut registry = RegistryService::new();
    let parser = OnuParser::new(LogLevel::Error);
    
    // CURRENT FAILURE: Thinks 'a' starts a type definition.
    let _discourses = parser.parse_with_registry(tokens, &mut registry).expect("Should allow 'a' as identifier via lookahead");
}

/// Scenario 4: Operators as Behaviors
/// We want 'added-to' and 'opposes' to be usable in the 'utilizes' syntax.
#[test]
fn test_operator_identity_mapping() {
    let source = r#"
the behavior called op-test
    takes: an integer called x
    delivers: an integer
    as:
        derivation: res derives-from x utilizes added-to 10
        res
"#;
    let lexer = OnuLexer::new(LogLevel::Error);
    let tokens = lexer.lex(source).unwrap();
    let mut registry = RegistryService::new();
    let parser = OnuParser::new(LogLevel::Error);
    
    // CURRENT FAILURE: 'added-to' is seen as Token::AddedTo, not a behavior name string.
    let _discourses = parser.parse_with_registry(tokens, &mut registry).expect("Should map Token::AddedTo to 'added-to' string");
}
