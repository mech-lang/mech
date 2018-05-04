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
    v.push(Change::Add{table: ball, row: i, column: 5, value: Value::from_u64(16)});
    v.push(Change::Add{table: ball, row: i, column: 6, value: Value::from_u64(500)});
  
  }
  v
}

fn position_update() -> Block {
  let mut block = Block::new();
  let ball = Hasher::hash_str("ball");
  let system_timer_change = Hasher::hash_str("system/timer/change");
  block.add_constraint(Constraint::Scan {table: system_timer_change, column: 4, register: 1});
  block.add_constraint(Constraint::Scan {table: ball, column: 1, register: 2});
  block.add_constraint(Constraint::Scan {table: ball, column: 2, register: 3});
  block.add_constraint(Constraint::Scan {table: ball, column: 3, register: 4});
  block.add_constraint(Constraint::Scan {table: ball, column: 4, register: 5});
  block.add_constraint(Constraint::Scan {table: ball, column: 5, register: 6});
  block.add_constraint(Constraint::Function {operation: Function::Add, parameters: vec![2, 4], output: 1});
  block.add_constraint(Constraint::Function {operation: Function::Add, parameters: vec![3, 5], output: 2});
  block.add_constraint(Constraint::Function {operation: Function::Add, parameters: vec![5, 6], output: 3});
  block.add_constraint(Constraint::Insert {table: ball, column: 1, register: 1});
  block.add_constraint(Constraint::Insert {table: ball, column: 2, register: 2});
  block.add_constraint(Constraint::Insert {table: ball, column: 4, register: 3});
  let plan = vec![
    Constraint::Function {operation: Function::Add, parameters: vec![2, 4], output: 1},
    Constraint::Function {operation: Function::Subtract, parameters: vec![3, 5], output: 2},
    Constraint::Function {operation: Function::Add, parameters: vec![5, 6], output: 3},
    Constraint::Insert {table: ball, column: 1, register: 1},
    Constraint::Insert {table: ball, column: 2, register: 2},
    Constraint::Insert {table: ball, column: 4, register: 3},
  ];
  block.plan = plan;
  block
}

fn boundary_check() -> Block {
  let mut block = Block::new();
  let ball = Hasher::hash_str("ball");
  block.add_constraint(Constraint::Scan {table: ball, column: 1, register: 1});
  block.add_constraint(Constraint::Scan {table: ball, column: 6, register: 2});
  block.add_constraint(Constraint::Filter {comparator: Comparator::GreaterThan, lhs: 1, rhs: 2, register: 1});
  block.add_constraint(Constraint::Constant {value: 500, register: 3});
  block.add_constraint(Constraint::Insert {table: ball, column: 1, register: 1});  
  let plan = vec![
    Constraint::Filter {comparator: Comparator::GreaterThan, lhs: 1, rhs: 2, register: 1}
  ];
  block.plan = plan;
  block
}

fn boundary_check2() -> Block {
  let mut block = Block::new();
  let ball = Hasher::hash_str("ball");
  block.add_constraint(Constraint::Scan {table: ball, column: 1, register: 1});
  block.add_constraint(Constraint::Scan {table: ball, column: 6, register: 2});
  block.add_constraint(Constraint::Filter {comparator: Comparator::LessThan, lhs: 1, rhs: 2, register: 1});
  block.add_constraint(Constraint::Constant {value: 0, register: 3});
  block.add_constraint(Constraint::Insert {table: ball, column: 1, register: 1});  
  let plan = vec![
    Constraint::Filter {comparator: Comparator::GreaterThan, lhs: 1, rhs: 2, register: 1}
  ];
  block.plan = plan;
  block
}

fn boundary_check3() -> Block {
  let mut block = Block::new();
  let ball = Hasher::hash_str("ball");
  block.add_constraint(Constraint::Scan {table: ball, column: 2, register: 1});
  block.add_constraint(Constraint::Scan {table: ball, column: 6, register: 2});
  block.add_constraint(Constraint::Filter {comparator: Comparator::GreaterThan, lhs: 1, rhs: 2, register: 1});
  block.add_constraint(Constraint::Constant {value: 500, register: 3});
  block.add_constraint(Constraint::Insert {table: ball, column: 1, register: 1});  
  let plan = vec![
    Constraint::Filter {comparator: Comparator::GreaterThan, lhs: 1, rhs: 2, register: 1}
  ];
  block.plan = plan;
  block
}

fn boundary_check4() -> Block {
  let mut block = Block::new();
  let ball = Hasher::hash_str("ball");
  block.add_constraint(Constraint::Scan {table: ball, column: 2, register: 1});
  block.add_constraint(Constraint::Scan {table: ball, column: 6, register: 2});
  block.add_constraint(Constraint::Filter {comparator: Comparator::LessThan, lhs: 1, rhs: 2, register: 1});
  block.add_constraint(Constraint::Constant {value: 0, register: 3});
  block.add_constraint(Constraint::Insert {table: ball, column: 1, register: 1});  
  let plan = vec![
    Constraint::Filter {comparator: Comparator::GreaterThan, lhs: 1, rhs: 2, register: 1}
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
}

fn make_db(n: u64) -> Database {
  let mut db = Database::new(1000, 2);
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
}