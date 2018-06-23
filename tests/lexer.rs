extern crate mech_syntax;

use mech_syntax::lexer::{Lexer, Token};
use mech_syntax::lexer::Token::{HashTag, Alpha, LeftBracket, RightBracket, Digit, Space, Comma, Plus, Equal, EndOfStream};

#[test]
fn lex_table() {
    let mut lexer = Lexer::new();
    lexer.add_string(String::from("#abc"));
    let tokens = lexer.get_tokens();
    assert_eq!(tokens, &vec![HashTag, Alpha, Alpha, Alpha, EndOfStream]);
}

#[test]
fn lex_left_bracket() {
    let mut lexer = Lexer::new();
    lexer.add_string(String::from("["));
    let tokens = lexer.get_tokens();
    assert_eq!(tokens, &vec![LeftBracket, EndOfStream]);
}

#[test]
fn lex_right_bracket() {
    let mut lexer = Lexer::new();
    lexer.add_string(String::from("]"));
    let tokens = lexer.get_tokens();
    assert_eq!(tokens, &vec![RightBracket, EndOfStream]);
}

#[test]
fn lex_table_with_brackets() {
    let mut lexer = Lexer::new();
    lexer.add_string(String::from("#abc[]"));
    let tokens = lexer.get_tokens();
    assert_eq!(tokens, &vec![HashTag, Alpha, Alpha, Alpha, LeftBracket, RightBracket, EndOfStream]);
}

#[test]
fn lex_table_full() {
    let mut lexer = Lexer::new();
    lexer.add_string(String::from("#abc[1, 2, 3]"));
    let tokens = lexer.get_tokens();
    assert_eq!(tokens, &vec![HashTag, Alpha, Alpha, Alpha, LeftBracket, Digit, Comma, Space, Digit, Comma, Space, Digit, RightBracket, EndOfStream]);
}

#[test]
fn lex_table_add() {
    let mut lexer = Lexer::new();
    lexer.add_string(String::from("#abc += [1 2]"));
    let tokens = lexer.get_tokens();
    assert_eq!(tokens, &vec![HashTag, Alpha, Alpha, Alpha, Space, Plus, Equal, Space, LeftBracket, Digit, Space, Digit, RightBracket, EndOfStream]);
}

#[test]
fn lex_table_relation() {
    let mut lexer = Lexer::new();
    lexer.add_string(String::from("#abc[3] = #abc[1] + #abc[2]"));
    let tokens = lexer.get_tokens();
    assert_eq!(tokens, &vec![HashTag, Alpha, Alpha, Alpha, LeftBracket, Digit, RightBracket, Space, Equal, Space, HashTag, Alpha, Alpha, Alpha, LeftBracket, Digit, RightBracket, Space, Plus, Space, HashTag, Alpha, Alpha, Alpha, LeftBracket, Digit, RightBracket, EndOfStream]);
}