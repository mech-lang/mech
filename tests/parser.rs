#[macro_use]
extern crate mech_syntax;

use mech_syntax::lexer::{Lexer, Token};
use mech_syntax::lexer::Token::{HashTag, Identifier, LeftBracket, RightBracket, Digit, Space, Comma, Plus, Equal};
use mech_syntax::parser::Parser;

#[test]
fn and_combinator() {
  assert_eq!(and_combinator! {true, true, true}, true);
  assert_eq!(and_combinator! {true, true, false}, false);
  assert_eq!(and_combinator! {true, false, false}, false);
  assert_eq!(and_combinator! {false, false, false}, false);
  assert_eq!(and_combinator! {false, true, true}, false);
  assert_eq!(and_combinator! {false, false, true}, false);
  assert_eq!(and_combinator! {true, false, true}, false);
  assert_eq!(and_combinator! {false, true, false}, false);
  assert_eq!(and_combinator! {true}, true);
  assert_eq!(and_combinator! {false}, false);
  assert_eq!(and_combinator! {true, false}, false);
  assert_eq!(and_combinator! {false, true}, false);
  assert_eq!(and_combinator! {true, true}, true);
}

#[test]
fn or_combinator() {
  assert_eq!(or_combinator! {true, true, true}, true);
  assert_eq!(or_combinator! {true, true, false}, true);
  assert_eq!(or_combinator! {true, false, false}, true);
  assert_eq!(or_combinator! {false, false, false}, false);
  assert_eq!(or_combinator! {false, true, true}, true);
  assert_eq!(or_combinator! {false, false, true}, true);
  assert_eq!(or_combinator! {true, false, true}, true);
  assert_eq!(or_combinator! {false, true, false}, true);
  assert_eq!(or_combinator! {true}, true);
  assert_eq!(or_combinator! {false}, false);
  assert_eq!(or_combinator! {true, false}, true);
  assert_eq!(or_combinator! {false, true}, true);
  assert_eq!(or_combinator! {true, true}, true);
}