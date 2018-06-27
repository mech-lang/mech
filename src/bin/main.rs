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
use mech_syntax::compiler::Compiler;
use mech::Block;
use mech::{Change, Transaction};
use mech::{Value};
use mech::Hasher;
use mech::Core;


fn main() {
  
  let mut lexer = Lexer::new();
  let mut parser = Parser::new();
  let mut compiler = Compiler::new();
  let mut core = Core::new(1112, 10);

 let input = String::from("# Bouncing Balls

## Section One

This is an intro paragraph

## Section Two

  #x = 1
  #y = 2

And another block in section two

  x = #x

## Section Three

And this is another paragraph
  #scene = #add.1");
  let add = Hasher::hash_str("add");
  println!("{:?}", input);

  lexer.add_string(input.clone());
  let tokens = lexer.get_tokens();
  println!("{:?}", tokens);
  
  parser.text = input;
  parser.add_tokens(&mut tokens.clone());
  parser.build_parse_tree();

  println!("--------------------------------------------");
  println!("{:?}", parser.parse_tree);
  compiler.build_syntax_tree(parser.parse_tree);
  println!("{:?}", compiler.syntax_tree);
  
}