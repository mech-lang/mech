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
  let system_timer_change = Hasher::hash_str("system/timer/change");
  block.add_constraint(Constraint::Scan {table: system_timer_change, column: 4, input: 1});
  block.add_constraint(Constraint::Scan {table: ball, column: 1, input: 2});
  block.add_constraint(Constraint::Scan {table: ball, column: 2, input: 3});
  block.add_constraint(Constraint::Scan {table: ball, column: 3, input: 4});
  block.add_constraint(Constraint::Scan {table: ball, column: 4, input: 5});  
  block.add_constraint(Constraint::Function {operation: Function::Add, parameters: vec![5, 6], output: 1}); 
  block.add_constraint(Constraint::Function {operation: Function::Add, parameters: vec![7, 8], output: 2});
  block.add_constraint(Constraint::Function {operation: Function::Add, parameters: vec![8, 4], output: 3});
  block.add_constraint(Constraint::Constant {value: 16, input: 4});
  block.add_constraint(Constraint::Identity {source: 2, sink: 5});
  block.add_constraint(Constraint::Identity {source: 4, sink: 6});
  block.add_constraint(Constraint::Identity {source: 3, sink: 7});
  block.add_constraint(Constraint::Identity {source: 5, sink: 8});
  block.add_constraint(Constraint::Insert {output: 1, table: ball, column: 1});
  block.add_constraint(Constraint::Insert {output: 2, table: ball, column: 2});
  block.add_constraint(Constraint::Insert {output: 3, table: ball, column: 4});
  let plan = vec![
    Constraint::Function {operation: Function::Add, parameters: vec![5, 6], output: 1},
    Constraint::Function {operation: Function::Add, parameters: vec![7, 8], output: 2},
    Constraint::Function {operation: Function::Add, parameters: vec![8, 4], output: 3},
    Constraint::Insert {output: 1, table: ball, column: 1},
    Constraint::Insert {output: 2, table: ball, column: 2},
    Constraint::Insert {output: 3, table: ball, column: 4},
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
  block.add_constraint(Constraint::Scan {table: ball, column: 1, input: 1});
  block.add_constraint(Constraint::Scan {table: ball, column: 4, input: 2});
  block.add_constraint(Constraint::Filter {comparator: Comparator::GreaterThan, lhs: 2, rhs: 3, intermediate: 1});
  block.add_constraint(Constraint::Identity {source: 1, sink: 2});
  block.add_constraint(Constraint::Identity {source: 2, sink: 3});
  block.add_constraint(Constraint::Insert {output: 1, table: ball, column: 3});  
  let plan = vec![
    Constraint::Filter {comparator: Comparator::GreaterThan, lhs: 2, rhs: 3, intermediate: 1}
  ];
  block.plan = plan;
  block
}

fn boundary_check2() -> Block {
  let mut block = Block::new();
  let ball = Hasher::hash_str("ball");
  block.add_constraint(Constraint::Scan {table: ball, column: 1, input: 1});
  block.add_constraint(Constraint::Scan {table: ball, column: 4, input: 2});
  block.add_constraint(Constraint::Filter {comparator: Comparator::LessThan, lhs: 2, rhs: 3, intermediate: 1});
  block.add_constraint(Constraint::Identity {source: 1, sink: 2});
  block.add_constraint(Constraint::Identity {source: 2, sink: 3});
  block.add_constraint(Constraint::Insert {output: 1, table: ball, column: 3});  
  let plan = vec![
    Constraint::Filter {comparator: Comparator::GreaterThan, lhs: 2, rhs: 3, intermediate: 1}
  ];
  block.plan = plan;
  block
}

fn boundary_check3() -> Block {
  let mut block = Block::new();
  let ball = Hasher::hash_str("ball");
  block.add_constraint(Constraint::Scan {table: ball, column: 1, input: 1});
  block.add_constraint(Constraint::Scan {table: ball, column: 4, input: 2});
  block.add_constraint(Constraint::Filter {comparator: Comparator::GreaterThan, lhs: 2, rhs: 3, intermediate: 1});
  block.add_constraint(Constraint::Identity {source: 1, sink: 2});
  block.add_constraint(Constraint::Identity {source: 2, sink: 3});
  block.add_constraint(Constraint::Insert {output: 1, table: ball, column: 3});  
  let plan = vec![
    Constraint::Filter {comparator: Comparator::GreaterThan, lhs: 2, rhs: 3, intermediate: 1}
  ];
  block.plan = plan;
  block
}

fn boundary_check4() -> Block {
  let mut block = Block::new();
  let ball = Hasher::hash_str("ball");
  block.add_constraint(Constraint::Scan {table: ball, column: 1, input: 1});
  block.add_constraint(Constraint::Scan {table: ball, column: 4, input: 2});
  block.add_constraint(Constraint::Filter {comparator: Comparator::LessThan, lhs: 2, rhs: 3, intermediate: 1});
  block.add_constraint(Constraint::Identity {source: 1, sink: 2});
  block.add_constraint(Constraint::Identity {source: 2, sink: 3});
  block.add_constraint(Constraint::Insert {output: 1, table: ball, column: 3});  
  let plan = vec![
    Constraint::Filter {comparator: Comparator::GreaterThan, lhs: 2, rhs: 3, intermediate: 1}
  ];
  block.plan = plan;
  block
}

fn step_db(db: &mut Database) {
  let system_timer_change = Hasher::hash_str("system/timer/change");
  let timer_id = 1;      
  let txn = Transaction::from_changeset(vec![
    Change::Add{table: system_timer_change, row: timer_id, column: 1, value: Value::from_u64(1)},
    Change::Add{table: system_timer_change, row: timer_id, column: 2, value: Value::from_u64(2)},
    Change::Add{table: system_timer_change, row: timer_id, column: 3, value: Value::from_u64(3)},
    Change::Add{table: system_timer_change, row: timer_id, column: 4, value: Value::from_u64(4)},
  ]);     
  db.process_transaction(&txn);
  for i in 1 .. 4000000 {

  }
}

fn make_db(n: u64) -> Database {
  let mut db = Database::new(10000000, 2);
    let system_timer_change = Hasher::hash_str("system/timer/change");
  let ball = Hasher::hash_str("ball");
  let ws = Hasher::hash_str("client/websocket");
  db.runtime.register_blocks(vec![export_ball()], &mut db.store);
  let mut balls = make_balls(n);
  let mut table_changes = vec![
    Change::NewTable{tag: system_timer_change, rows: 1, columns: 4}, 
    Change::NewTable{tag: ball, rows: n as usize, columns: 6}, 
    Change::NewTable{tag: ws, rows: n as usize, columns: 2}, 
  ];
  table_changes.append(&mut balls);
  let txn = Transaction::from_changeset(table_changes);
  db.process_transaction(&txn);
  db.register_watcher(ball);
  db
}

fn main() {
  let mut db = make_db(10);
  loop {
    println!("{:?}", db);
    println!("{:?}", db.runtime);
    step_db(&mut db);
  }
}