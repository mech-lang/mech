extern crate mech_syntax;

use mech_syntax::lexer::{Lexer, Token};
use mech_syntax::lexer::Token::{Table, LeftBracket, RightBracket, Digit, Space, Comma, Plus, Equal};

#[test]
fn lex_table() {
    let mut lexer = Lexer::new();
    lexer.add_string(String::from("#abc"));
    let tokens = lexer.get_tokens();
    assert_eq!(tokens, &vec![Table{name: vec![97, 98, 99]}]);
}

#[test]
fn lex_left_bracket() {
    let mut lexer = Lexer::new();
    lexer.add_string(String::from("["));
    let tokens = lexer.get_tokens();
    assert_eq!(tokens, &vec![LeftBracket]);
}

#[test]
fn lex_right_bracket() {
    let mut lexer = Lexer::new();
    lexer.add_string(String::from("]"));
    let tokens = lexer.get_tokens();
    assert_eq!(tokens, &vec![RightBracket]);
}

#[test]
fn lex_table_with_brackets() {
    let mut lexer = Lexer::new();
    lexer.add_string(String::from("#abc[]"));
    let tokens = lexer.get_tokens();
    assert_eq!(tokens, &vec![Table{name: vec![97, 98, 99]}, LeftBracket, RightBracket]);
}

#[test]
fn lex_table_full() {
    let mut lexer = Lexer::new();
    lexer.add_string(String::from("#abc[1, 2, 3]"));
    let tokens = lexer.get_tokens();
    assert_eq!(tokens, &vec![Table { name: vec![97, 98, 99] }, LeftBracket, Digit { value: 49 }, Comma, Space, Digit { value: 50 }, Comma, Space, Digit { value: 51 }, RightBracket]);
}

#[test]
fn lex_table_add() {
    let mut lexer = Lexer::new();
    lexer.add_string(String::from("#abc += [1 2]"));
    let tokens = lexer.get_tokens();
    assert_eq!(tokens, &vec![Table { name: vec![97, 98, 99] }, Space, Plus, Equal, Space, LeftBracket, Digit { value: 49 }, Space, Digit { value: 50 }, RightBracket]);
}

#[test]
fn lex_table_relation() {
    let mut lexer = Lexer::new();
    lexer.add_string(String::from("#abc[3] = #abc[1] + #abc[2]"));
    let tokens = lexer.get_tokens();
    assert_eq!(tokens, &vec![Table { name: vec![97, 98, 99] }, LeftBracket, Digit { value: 51 }, RightBracket, Space, Equal, Space, Table { name: vec![97, 98, 99] }, LeftBracket, Digit { value: 49 }, RightBracket, Space, Plus, Space, Table { name: vec![97, 98, 99] }, LeftBracket, Digit { value: 50 }, RightBracket]);
}