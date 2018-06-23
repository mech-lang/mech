#[macro_use]
extern crate mech_syntax;

use mech_syntax::lexer::{Lexer, Token};
use mech_syntax::lexer::Token::{HashTag, Alpha, LeftBracket, RightBracket, Digit, Space, Comma, Plus, Equal};
use mech_syntax::parser::Parser;