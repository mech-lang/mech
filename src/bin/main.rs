#![feature(test)]

extern crate test;
extern crate mech;
extern crate core;
extern crate rand;

use test::Bencher;
use mech::database::{Database, Transaction, Change};
use mech::table::{Value, Table};
use mech::indexes::Hasher;
use mech::operations::{Function, Plan, Comparator};
use mech::runtime::{Runtime, Block, Constraint, Register};
use rand::{Rng};


fn make_balls(n: u64) -> Vec<Change> {
  let mut v = Vec::new();
  for i in 0 .. n + 1 {

    let ball = Hasher::hash_str("ball");
  
    v.push(Change::Add{table: ball, row: i, column: 1, value: Value::from_u64(1)});
    v.push(Change::Add{table: ball, row: i, column: 2, value: Value::from_u64(2)});
    v.push(Change::Add{table: ball, row: i, column: 3, value: Value::from_u64(3)});
    v.push(Change::Add{table: ball, row: i, column: 4, value: Value::from_u64(4)});
  
  }
  v
}

fn position_update() -> Block {
  let mut block = Block::new();
  let ball = Hasher::hash_str("ball");
  let system_timer_change = Hasher::hash_str("system/timer");
  block.add_constraint(Constraint::Scan {table: system_timer_change, column: 4, input: 1});
  block.add_constraint(Constraint::Scan {table: ball, column: 1, input: 2});
  block.add_constraint(Constraint::Scan {table: ball, column: 2, input: 3});
  block.add_constraint(Constraint::Scan {table: ball, column: 3, input: 4});
  block.add_constraint(Constraint::Scan {table: ball, column: 4, input: 5});  
  block.add_constraint(Constraint::Identity {source: 2, sink: 1});
  block.add_constraint(Constraint::Identity {source: 4, sink: 2});
  block.add_constraint(Constraint::Identity {source: 3, sink: 3});
  block.add_constraint(Constraint::Identity {source: 5, sink: 4});
  block.add_constraint(Constraint::Function {operation: Function::Add, parameters: vec![1, 2], output: 5}); 
  block.add_constraint(Constraint::Function {operation: Function::Add, parameters: vec![3, 4], output: 6});
  block.add_constraint(Constraint::Constant {value: 10, input: 7});
  block.add_constraint(Constraint::Function {operation: Function::Add, parameters: vec![4, 7], output: 8});
  block.add_constraint(Constraint::Insert {output: 5, table: ball, column: 1});
  block.add_constraint(Constraint::Insert {output: 6, table: ball, column: 2});
  block.add_constraint(Constraint::Insert {output: 7, table: ball, column: 4});
  let plan = vec![
    Constraint::Identity {source: 2, sink: 1},
    Constraint::Identity {source: 4, sink: 2},
    Constraint::Identity {source: 3, sink: 3},
    Constraint::Identity {source: 5, sink: 4},
    Constraint::Constant {value: 10, input: 7},
    Constraint::Function {operation: Function::Add, parameters: vec![1, 2], output: 5},
    Constraint::Function {operation: Function::Add, parameters: vec![3, 4], output: 6},
    Constraint::Function {operation: Function::Add, parameters: vec![4, 7], output: 8},
    Constraint::Insert {output: 5, table: ball, column: 1},
    Constraint::Insert {output: 6, table: ball, column: 2},
    Constraint::Insert {output: 8, table: ball, column: 4},
  ];
  block.plan = plan;
  block
}

fn export_ball() -> Block {
  let mut block = Block::new();
  let ball = Hasher::hash_str("ball");
  let websocket = Hasher::hash_str("client/websocket");
  block.add_constraint(Constraint::Scan {table: ball, column: 1, input: 1});
  block.add_constraint(Constraint::Scan {table: ball, column: 2, input: 2});
  block.add_constraint(Constraint::Identity {source: 1, sink: 1});
  block.add_constraint(Constraint::Identity {source: 2, sink: 2});
  block.add_constraint(Constraint::Insert {output: 1, table: websocket, column: 1});
  block.add_constraint(Constraint::Insert {output: 2, table: websocket, column: 2});
  let plan = vec![
    Constraint::Identity {source: 1, sink: 1},
    Constraint::Identity {source: 2, sink: 2},
    Constraint::Insert {output: 1, table: websocket, column: 1 },
    Constraint::Insert {output: 2, table: websocket, column: 2 },
  ];
  block.plan = plan;
  block
}

fn boundary_check() -> Block {
  let mut block = Block::new();
  let ball = Hasher::hash_str("ball");
  block.add_constraint(Constraint::Scan {table: ball, column: 2, input: 1});
  block.add_constraint(Constraint::Scan {table: ball, column: 4, input: 2});
  block.add_constraint(Constraint::Identity {source: 1, sink: 1});  
  block.add_constraint(Constraint::Constant {value: 5000, input: 2});
  block.add_constraint(Constraint::Filter {comparator: Comparator::GreaterThan, lhs: 1, rhs: 2, intermediate: 3});
  block.add_constraint(Constraint::Identity {source: 2, sink: 4});  
  block.add_constraint(Constraint::Constant {value: -1, input: 5});
  block.add_constraint(Constraint::Function {operation: Function::Multiply, parameters: vec![4, 5], output: 6});
  block.add_constraint(Constraint::Constant {value: 1, input: 7});
  block.add_constraint(Constraint::Function {operation: Function::Divide, parameters: vec![6, 7], output: 8});
  block.add_constraint(Constraint::Condition {truth: 3, result: 8, default: 5, output: 9});
  block.add_constraint(Constraint::Insert {output: 9, table: ball, column: 4});
  let plan = vec![
    Constraint::Identity {source: 1, sink: 1},
    Constraint::Identity {source: 2, sink: 4},
    Constraint::Constant {value: 5000, input: 2},
    Constraint::Constant {value: -9, input: 5},
    Constraint::Filter {comparator: Comparator::GreaterThan, lhs: 1, rhs: 2, intermediate: 3},
    Constraint::Function {operation: Function::Multiply, parameters: vec![4, 5], output: 6},
    Constraint::Constant {value: 10, input: 7},
    Constraint::Function {operation: Function::Divide, parameters: vec![6, 7], output: 8},
    Constraint::Condition {truth: 3, result: 8, default: 4, output: 9},
    Constraint::Insert {output: 9, table: ball, column: 4}
  ];
  block.plan = plan;
  block
}

fn step_db(db: &mut Database, n: u64) {
  let system_timer_change = Hasher::hash_str("system/timer");
  let timer_id = 1;      
  let txn = Transaction::from_change(Change::Add{table: system_timer_change, row: 1, column: 4, value: Value::from_u64(n)});     
  db.process_transaction(&txn);
  let mut q = 0;
  for i in 1 .. 4000000 {

  }
}


fn block1() -> Block {
  let mut block = Block::new();
  block.add_constraint(Constraint::Scan {table: 1, column: 1, input: 1});
  block.add_constraint(Constraint::Scan {table: 2, column: 1, input: 2});
  block.add_constraint(Constraint::Identity {source: 1, sink: 1});
  block.add_constraint(Constraint::Identity {source: 2, sink: 2});
  let plan = vec![
    Constraint::Identity {source: 1, sink: 1},
    Constraint::Identity {source: 2, sink: 2},
  ];
  block.plan = plan;
  block
}

fn block2() -> Block {
  let mut block = Block::new();
  block.add_constraint(Constraint::Scan {table: 1, column: 1, input: 1});
  block.add_constraint(Constraint::Identity {source: 1, sink: 1});
  let plan = vec![
    Constraint::Identity {source: 1, sink: 1},
  ];
  block.plan = plan;
  block
}

fn reset_balls() -> Block {
  let mut block = Block::new();
  let ball = Hasher::hash_str("ball");
  let click = Hasher::hash_str("html/event/click");
  block.add_constraint(Constraint::Scan {table: click, column: 1, input: 1});
  block.add_constraint(Constraint::Scan {table: click, column: 2, input: 2});
  block.add_constraint(Constraint::Scan {table: ball, column: 1, input: 3});
  block.add_constraint(Constraint::Scan {table: ball, column: 2, input: 4});
  block.add_constraint(Constraint::Identity {source: 1, sink: 1});
  block.add_constraint(Constraint::Identity {source: 2, sink: 2});
  block.add_constraint(Constraint::Identity {source: 3, sink: 3});
  block.add_constraint(Constraint::Identity {source: 4, sink: 4});
  block.add_constraint(Constraint::Constant {value: 0, input: 5});
  block.add_constraint(Constraint::Function {operation: Function::Multiply, parameters: vec![3, 5], output: 6});
  block.add_constraint(Constraint::Function {operation: Function::Multiply, parameters: vec![4, 5], output: 7});
  block.add_constraint(Constraint::Insert {output: 6, table: ball, column: 1});
  block.add_constraint(Constraint::Insert {output: 7, table: ball, column: 2});
  let plan = vec![
    Constraint::Identity {source: 1, sink: 1},
    Constraint::Identity {source: 2, sink: 2},
    Constraint::Identity {source: 3, sink: 3},
    Constraint::Identity {source: 4, sink: 4},
    Constraint::Constant {value: 0, input: 5},
    Constraint::Function {operation: Function::Multiply, parameters: vec![3, 5], output: 6},
    Constraint::Function {operation: Function::Multiply, parameters: vec![4, 5], output: 7},
    Constraint::Insert {output: 6, table: ball, column: 1 },
    Constraint::Insert {output: 7, table: ball, column: 2 },
  ];
  block.plan = plan;
  block
}

fn position_update_1d() -> Block {
  let mut block = Block::new();
  let ball = Hasher::hash_str("ball");
  let system_timer_change = Hasher::hash_str("system/timer");
  block.add_constraint(Constraint::Scan {table: ball, column: 1, input: 1});
  block.add_constraint(Constraint::Scan {table: ball, column: 1, input: 2});
  block.add_constraint(Constraint::ChangeScan {table: system_timer_change, column: 4, input: 3});
  block.add_constraint(Constraint::Identity {source: 1, sink: 1});
  block.add_constraint(Constraint::Identity {source: 2, sink: 2});
  block.add_constraint(Constraint::Function {operation: Function::Add, parameters: vec![1, 2], output: 3}); 
  block.add_constraint(Constraint::Insert {output: 3, table: ball, column: 1});
  let plan = vec![
    Constraint::Identity {source: 1, sink: 1},
    Constraint::Identity {source: 2, sink: 2},
    Constraint::ChangeScan {table: system_timer_change, column: 4, input: 3},
    Constraint::Function {operation: Function::Add, parameters: vec![1, 2], output: 3},
    Constraint::Insert {output: 3, table: ball, column: 1},
  ];
  block.plan = plan;
  block
}

fn position_update_1d2() -> Block {
  let mut block = Block::new();
  let ball = Hasher::hash_str("ball");
  let system_timer_change = Hasher::hash_str("system/timer");
  block.add_constraint(Constraint::Scan {table: ball, column: 1, input: 1});
  block.add_constraint(Constraint::Scan {table: ball, column: 2, input: 2});
  block.add_constraint(Constraint::Identity {source: 1, sink: 1});
  block.add_constraint(Constraint::Identity {source: 2, sink: 2});
  block.add_constraint(Constraint::Function {operation: Function::Add, parameters: vec![1, 2], output: 3}); 
  block.add_constraint(Constraint::Insert {output: 3, table: ball, column: 3});
  let plan = vec![
    Constraint::Identity {source: 1, sink: 1},
    Constraint::Identity {source: 2, sink: 2},
    Constraint::Function {operation: Function::Add, parameters: vec![1, 2], output: 3},
    Constraint::Insert {output: 3, table: ball, column: 3},
  ];
  block.plan = plan;
  block
}

fn make_db(n: u64) -> Database {
  let mut db = Database::new(10000000, 2);
  let system_timer_change = Hasher::hash_str("system/timer");
  let ball = Hasher::hash_str("ball");
  let ws = Hasher::hash_str("client/websocket");
  let click = Hasher::hash_str("html/event/click");
  db.runtime.register_blocks(vec![
    position_update_1d(), 
    position_update_1d2(), 
    //reset_balls(),
    //boundary_check()
  ], &mut db.store);
  let mut balls = make_balls(n);
  let mut table_changes = vec![
    //Change::NewTable{tag: 1, rows: 1, columns: 1}, 
    //Change::NewTable{tag: 2, rows: 1, columns: 1}, 
    //Change::NewTable{tag: 3, rows: 1, columns: 1}, 
    Change::NewTable{tag: click, rows: 1, columns: 2},
    Change::NewTable{tag: system_timer_change, rows: 1, columns: 4}, 
    Change::NewTable{tag: ball, rows: n as usize, columns: 6}, 
    //Change::NewTable{tag: ws, rows: n as usize, columns: 2}, 
  ];
  table_changes.append(&mut balls);
  let txn = Transaction::from_changeset(table_changes);
  db.process_transaction(&txn);
  db.register_watcher(ball);
  db
}

fn main() {
  let mut db = make_db(10);
  let mut i: u64 = 0;
  loop {
    println!("{:?}", db);
    //println!("{:?}", db.runtime);
    //println!("{:?}", i);
    step_db(&mut db, i);
    
    i += 1;
  }
}