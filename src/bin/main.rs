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

  let input = String::from("#table");

  let mut lexer = Lexer::new();
  let mut parser = Parser::new();
  let mut compiler = Compiler::new();
  lexer.add_string(input.clone());
  let tokens = lexer.get_tokens();
  parser.text = input;
  parser.add_tokens(&mut tokens.clone());
  parser.build_parse_tree();

  println!("--------");
  println!("{:?}", parser.parse_tree);

  compiler.build_syntax_tree(parser.parse_tree);
  let ast = compiler.syntax_tree.clone();
  compiler.compile_blocks(ast);



  println!("--------");
  println!("{:?}", compiler.syntax_tree);
  println!("--------");
  println!("{:?}", compiler.blocks);

  println!("--------");

  let mut table_changes = vec![
    Change::NewTable{tag: 0x78, rows: 1, columns: 1}, 
    Change::NewTable{tag: 0x79, rows: 1, columns: 1}, 
  ];
  let txn = Transaction::from_changeset(table_changes);
  core.process_transaction(&txn);
  core.register_blocks(compiler.blocks);
  core.runtime.run_network(&mut core.store);
  println!("{:?}", core);
  println!("{:?}", core.store.changes);


  //assert_eq!(parser.status, ParseStatus::Ready);


  
}