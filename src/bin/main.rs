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
  let mut core = Core::new(100,10);

  let input = String::from("#add.3 = #add.1 + #add.2");
  let add = Hasher::hash_str("add");
  println!("{:?}", input);

  lexer.add_string(input);
  let tokens = lexer.get_tokens();
  println!("{:?}", tokens);
  
  parser.add_tokens(&mut tokens.clone());
  parser.build_ast();
  
  println!("{:?}", parser);
  println!("--------------------------------------------");

  walk_tree(&parser.ast, 0);
  let constraints = compiler.compile(parser.ast);
  let mut block = Block::new();
  block.add_constraints(constraints);
  block.plan();
  core.runtime.register_blocks(vec![block], &mut core.store);
  println!("{:?}", core);
  println!("{:?}", core.runtime);
  let txn = Transaction::from_changeset(vec![
    Change::NewTable{tag: add, rows: 4, columns: 3},
    Change::Add{table: add, row: 1, column: 1, value: Value::from_u64(1)},
    Change::Add{table: add, row: 1, column: 2, value: Value::from_u64(2)},
    Change::Add{table: add, row: 2, column: 1, value: Value::from_u64(3)},
    Change::Add{table: add, row: 2, column: 2, value: Value::from_u64(4)},
    Change::Add{table: add, row: 3, column: 1, value: Value::from_u64(5)},
    Change::Add{table: add, row: 3, column: 2, value: Value::from_u64(6)}
  ]);
  core.process_transaction(&txn);
  println!("{:?}", core);
  println!("{:?}", core.runtime);
}
 
pub fn walk_tree(node: &Node, depth: usize) {
  if depth == 0 {
    print!("");
  } else {
    print!("  ├");
  }
  space(depth);
  println!(" {:?}", node);
  match node {
    Node::Table{id, token, children} => {
      for child in children {
        walk_tree(child, depth + 1)
      }
    },
    Node::Select{children} => {
      for child in children {
        walk_tree(child, depth + 1)
      }
    },
    Node::ColumnDefine{parts} => {
      for child in parts {
        walk_tree(child, depth + 1)
      }
    },
    Node::MathExpression{parameters} => {
      for child in parameters {
        walk_tree(child, depth + 1)
      }
    },
    Node::Root{children} => {
      for child in children {
        walk_tree(child, depth + 1)
      }
    },
    Node::Constraint{children} => {
      for child in children {
        walk_tree(child, depth + 1)
      }
    },
    Node::Block{children} => {
      for child in children {
        walk_tree(child, depth + 1)
      }
    },
    Node::Insert{children} => {
      for child in children {
        walk_tree(child, depth + 1)
      }
    },
    _ => (),
  }
}

fn space(n: usize) {
  for _ in 0..n {
    print!("─");
  }
}