extern crate mech_syntax;

use mech_syntax::lexer::{Lexer, Token};

#[test]
fn lex_table() {
    let mut lexer = Lexer::new();
    lexer.add_string(String::from("#abc"));
    let tokens = lexer.get_tokens();
    assert_eq!(tokens, vec![Token::Table{name: vec![97, 98, 99]}]);
}

#[test]
fn lex_left_bracket() {
    let mut lexer = Lexer::new();
    lexer.add_string(String::from("["));
    let tokens = lexer.get_tokens();
    assert_eq!(tokens, vec![Token::LeftBracket]);
}

#[test]
fn lex_right_bracket() {
    let mut lexer = Lexer::new();
    lexer.add_string(String::from("]"));
    let tokens = lexer.get_tokens();
    assert_eq!(tokens, vec![Token::RightBracket]);
}

#[test]
fn lex_table_with_brackets() {
    let mut lexer = Lexer::new();
    lexer.add_string(String::from("#abc[]"));
    let tokens = lexer.get_tokens();
    assert_eq!(tokens, vec![Token::Table{name: vec![97, 98, 99]}, Token::LeftBracket, Token::RightBracket]);
}