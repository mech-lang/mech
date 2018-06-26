extern crate core;
extern crate time;
extern crate rand;
extern crate mech_syntax;
extern crate mech;

use std::time::SystemTime;
use std::thread::{self};
use std::time::*;
use rand::{Rng, thread_rng};
use mech_syntax::lexer::Lexer;
use mech_syntax::parser::{Parser, ParseStatus, Node};
//use mech_syntax::compiler::Compiler;
use mech::Block;
use mech::{Change, Transaction};
use mech::{Value};
use mech::Hasher;
use mech::Core;


fn main() {
  
  let mut lexer = Lexer::new();
  let mut parser = Parser::new();
  //let mut compiler = Compiler::new();
  let mut core = Core::new(100, 10);

  let input = String::from("# Title
## Subtitle
  1 + 1

");
  let add = Hasher::hash_str("add");
  println!("{:?}", input);

  lexer.add_string(input.clone());
  let tokens = lexer.get_tokens();
  println!("{:?}", tokens);
  
  parser.add_tokens(&mut tokens.clone());
  parser.build_parse_tree();
  
  println!("{:?}", parser);
  //println!("--------------------------------------------");
}