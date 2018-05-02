extern crate mech;
extern crate core;
extern crate time;
extern crate rand;

use std::time::SystemTime;
use mech::database::{Database, Transaction, Change};
use mech::table::{Value, Table};
use mech::indexes::Hasher;
use mech::operations::{Function, Plan};
use mech::runtime::{Runtime, Block, Constraint, Register};
use std::thread::{self};
use std::time::*;
use rand::{Rng, thread_rng};

fn make_ball(row2: usize) -> Vec<Change> {
  let row = row2 as u64;
  let mut rng = thread_rng();
  let x = rng.gen_range(1, 100);
  let y = rng.gen_range(1, 100);
  let dx = rng.gen_range(1, 10);
  let dy = rng.gen_range(1, 10);
  let ball = Hasher::hash_str("ball");
  vec![
    Change::Add{ix: 0, table: ball, row, column: 1, value: Value::from_u64(x)},
    Change::Add{ix: 0, table: ball, row, column: 2, value: Value::from_u64(y)},
    Change::Add{ix: 0, table: ball, row, column: 3, value: Value::from_u64(dx)},
    Change::Add{ix: 0, table: ball, row, column: 4, value: Value::from_u64(dy)},
    Change::Add{ix: 0, table: ball, row, column: 5, value: Value::from_u64(16)},
  ]
}


fn main() {

  let mut db = Database::new(100_000, 1_000_000, 2);
  let system_timer_change = Hasher::hash_str("system/timer/change");
  let ball = Hasher::hash_str("ball");
  let mut balls: Vec<Change> = vec![];
  let n: usize = 100000;
  for i in 1 .. n + 1 {
    let mut ball_changes = make_ball(i);
    balls.append(&mut ball_changes);
  }
  let mut table_changes = vec![
    Change::NewTable{tag: String::from("system/timer/change"), entities: vec![], attributes: vec![], rows: 1, columns: 4}, 
    Change::NewTable{tag: String::from("ball"), entities: vec![], attributes: vec![], rows: n, columns: 5}, 
  ];
  table_changes.append(&mut balls);
  let txn = Transaction::from_changeset(table_changes);
  
  //db.register_transaction(txn);
  db.process_transaction(&txn);

  let mut block = Block::new();
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
    Constraint::Function {operation: Function::Add, parameters: vec![3, 5], output: 2},
    Constraint::Function {operation: Function::Add, parameters: vec![5, 6], output: 3},
    Constraint::Insert {table: ball, column: 1, register: 1},
    Constraint::Insert {table: ball, column: 2, register: 2},
    Constraint::Insert {table: ball, column: 4, register: 3},
  ];
  block.plan = plan;
  
  db.runtime.register_block(block.clone(), &db.store);

  let mut v1 = vec![10; 1_000_000];
  let mut v2 = vec![25; 1_000_000];
  let mut v3 = vec![25; 1_000_000];

  let mut mean = 0.0;
  let n = 100;
  for q in 0 .. n {
    
    let cur_time = time::now();
    let timer_id = 1;
    let txn = Transaction::from_changeset(vec![
      Change::Add{ix: 0, table: system_timer_change, row: timer_id, column: 1, value: Value::from_u64(cur_time.tm_hour as u64)},
      Change::Add{ix: 0, table: system_timer_change, row: timer_id, column: 2, value: Value::from_u64(cur_time.tm_min as u64)},
      Change::Add{ix: 0, table: system_timer_change, row: timer_id, column: 3, value: Value::from_u64(cur_time.tm_sec as u64)},
      Change::Add{ix: 0, table: system_timer_change, row: timer_id, column: 4, value: Value::from_u64(cur_time.tm_nsec as u64)},
    ]);     
    let start_ns = time::precise_time_ns();      
    let changes = db.process_transaction(&txn);
    //let txn2 = Transaction::from_changeset(changes);
    //db.process_transaction(&txn2);
    //println!("{:?}", db);
    //println!("{:?}", db.runtime);
    let end_ns = time::precise_time_ns();
    let delta = end_ns - start_ns;
    let delta_sec = delta as f64 / 1.0e9;
    let scaled = 0.001 / delta_sec;
    if q > 1 {
      mean += scaled as f64;
    }
    //println!("{:?}", scaled * 1000.0);
  }

  println!("{:?}", db);
  //println!("{:?}", db.runtime);
  println!("Mean Round Frequency: {:0.6} KHz", mean as f64 / (n as f64 - 1.0));

}
