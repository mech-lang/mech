#![feature(test)]

extern crate test;
extern crate mech;
extern crate core;
extern crate rand;

use test::Bencher;
use mech::{Core, Transaction, Change};
use mech::{Value, Table};
use mech::Hasher;
use mech::{Function, Plan, Comparator};
use mech::{Runtime, Block, Constraint, Register};
use rand::{Rng};

/*
fn make_balls(n: u64) -> Vec<Change> {
  let mut v = Vec::new();
  for i in 0 .. n + 1 {

    let ball = Hasher::hash_str("ball");
  
    v.push(Change::Add{table: ball, row: i, column: 1, value: Value::from_u64(1)});
    v.push(Change::Add{table: ball, row: i, column: 2, value: Value::from_u64(2)});
    v.push(Change::Add{table: ball, row: i, column: 3, value: Value::from_u64(3)});
    v.push(Change::Add{table: ball, row: i, column: 4, value: Value::from_u64(4)});
    v.push(Change::Add{table: ball, row: i, column: 5, value: Value::from_u64(16)});
    v.push(Change::Add{table: ball, row: i, column: 6, value: Value::from_u64(500)});
  
  }
  v
}

fn position_update() -> Block {
  let mut block = Block::new();
  let ball = Hasher::hash_str("ball");
  let system_timer_change = Hasher::hash_str("system/timer/change");
  block.add_constraint(Constraint::ChangeScan {table: system_timer_change, column: 4, input: 1});
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
    Constraint::ChangeScan {table: system_timer_change, column: 4, input: 1},
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

fn step_db(db: &mut Core) {
  let system_timer_change = Hasher::hash_str("system/timer/change");
  let timer_id = 1;      
  let txn = Transaction::from_changeset(vec![
    Change::Add{table: system_timer_change, row: timer_id, column: 1, value: Value::from_u64(1)},
    Change::Add{table: system_timer_change, row: timer_id, column: 2, value: Value::from_u64(2)},
    Change::Add{table: system_timer_change, row: timer_id, column: 3, value: Value::from_u64(3)},
    Change::Add{table: system_timer_change, row: timer_id, column: 4, value: Value::from_u64(4)},
  ]);     
  db.process_transaction(&txn);
}

fn make_db(n: u64) -> Core {
  let mut db = Core::new(1000, 2);
    let system_timer_change = Hasher::hash_str("system/timer/change");
  let ball = Hasher::hash_str("ball");
  db.runtime.register_blocks(vec![position_update(), boundary_check(), boundary_check2(), boundary_check3(), boundary_check4()], &mut db.store);
  let mut balls = make_balls(n);
  let mut table_changes = vec![
    Change::NewTable{tag: system_timer_change, rows: 1, columns: 4}, 
    Change::NewTable{tag: ball, rows: n as usize, columns: 6}, 
  ];
  table_changes.append(&mut balls);
  let txn = Transaction::from_changeset(table_changes);
  db.process_transaction(&txn);
  db
}

#[bench]
fn balls_10(b:&mut Bencher) {
  let mut db = make_db(10);
  b.iter(|| {
    step_db(&mut db);
  });
}

#[bench]
fn balls_100(b:&mut Bencher) {
  let mut db = make_db(100);
  b.iter(|| {
    step_db(&mut db);
  });
}

#[bench]
fn balls_1_000(b:&mut Bencher) {
  let mut db = make_db(1000);
  b.iter(|| {
    step_db(&mut db);
  });
}

#[bench]
fn balls_10_000(b:&mut Bencher) {
  let mut db = make_db(10000);
  b.iter(|| {
    step_db(&mut db);
  });
}


#[bench]
fn balls_100_000(b:&mut Bencher) {
  let mut db = make_db(100000);
  b.iter(|| {
    step_db(&mut db);
  });
}*/