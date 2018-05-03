extern crate mech;
extern crate core;
extern crate time;
extern crate rand;

use std::time::SystemTime;
use mech::database::{Database, Transaction, Change};
use mech::table::{Value, Table};
use mech::indexes::Hasher;
use mech::operations::{Function, Plan, Comparator};
use mech::runtime::{Runtime, Block, Constraint, Register};
use std::thread::{self};
use std::time::*;
use rand::{Rng, thread_rng};

fn make_balls(n: u64) -> Vec<Change> {
  let mut v = Vec::new();
  for i in 0 .. n + 1 {

    let mut rng = thread_rng();
    let x = rng.gen_range(1, 100);
    let y = rng.gen_range(1, 100);
    let dx = rng.gen_range(1, 10);
    let dy = rng.gen_range(1, 10);
    let ball = Hasher::hash_str("ball");
  
    v.push(Change::Add{table: ball, row: i, column: 1, value: Value::from_u64(x)});
    v.push(Change::Add{table: ball, row: i, column: 2, value: Value::from_u64(y)});
    v.push(Change::Add{table: ball, row: i, column: 3, value: Value::from_u64(dx)});
    v.push(Change::Add{table: ball, row: i, column: 4, value: Value::from_u64(dy)});
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
  let cur_time = time::now();
  let timer_id = 1;      
  let txn = Transaction::from_changeset(vec![
    Change::Add{table: system_timer_change, row: timer_id, column: 1, value: Value::from_u64(cur_time.tm_hour as u64)},
    Change::Add{table: system_timer_change, row: timer_id, column: 2, value: Value::from_u64(cur_time.tm_min as u64)},
    Change::Add{table: system_timer_change, row: timer_id, column: 3, value: Value::from_u64(cur_time.tm_sec as u64)},
    Change::Add{table: system_timer_change, row: timer_id, column: 4, value: Value::from_u64(cur_time.tm_nsec as u64)},
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


fn main() {

  let mut db = make_db(10);
  
  let system_timer_change = Hasher::hash_str("system/timer/change");
  let mut mean = 0.0;

  thread::spawn(move || {
    let mut i = 1;    
    loop {   
      let cur_time = time::now();
      
      let timer_id = 1;

      let txn = Transaction::from_changeset(vec![
        Change::Add{table: system_timer_change, row: timer_id, column: 1, value: Value::from_u64(cur_time.tm_hour as u64)},
        Change::Add{table: system_timer_change, row: timer_id, column: 2, value: Value::from_u64(cur_time.tm_min as u64)},
        Change::Add{table: system_timer_change, row: timer_id, column: 3, value: Value::from_u64(cur_time.tm_sec as u64)},
        Change::Add{table: system_timer_change, row: timer_id, column: 4, value: Value::from_u64(cur_time.tm_nsec as u64)},
      ]);     

      thread::sleep(Duration::from_millis(10));    

      let start_ns = time::precise_time_ns();  
      db.process_transaction(&txn);
      let end_ns = time::precise_time_ns();
      
      let delta = end_ns - start_ns;
      let delta_sec = delta as f64 / 1.0e9;
      let scaled = 0.001 / delta_sec;
      if db.capacity() >= 100.0 {
        mean += scaled as f64;
        i += 1;
      }
      
      println!("{:?}", db.runtime);
      println!("{:?}", db);
      println!("Mean Steady State Round Frequency: {:0.6} KHz", mean as f64 / (i as f64 - 1.0));
      println!("Capacity: {:0.2}%", db.capacity());
    }
    
  });

  //println!("{:?}", db);
  //println!("{:?}", db.runtime);
  loop{}
}
