extern crate mech;
extern crate core;

use std::time::SystemTime;
use mech::database::{Database, Transaction, Change};
use mech::table::{Value, Table};
use mech::indexes::Hasher;
use mech::operations::{Function, Plan};
use mech::runtime::{Runtime, Block, Constraint, Register};

fn main() {

  let mut db = Database::new(1000, 1000, 1000);
  let table_id = Hasher::hash_str("students");
  let txn = Transaction::from_changeset(vec![
    Change::Add{ix: 0, table: table_id, row: 1, column: 2, value: Value::from_u64(76)},
    Change::Add{ix: 0, table: table_id, row: 2, column: 2, value: Value::from_u64(88)},
    Change::Add{ix: 0, table: table_id, row: 2, column: 1, value: Value::from_u64(99)},
    Change::Add{ix: 0, table: table_id, row: 1, column: 1, value: Value::from_u64(83)}, 
    Change::NewTable{tag: String::from("students"), entities: vec![], attributes: vec![], rows: 10, columns: 10}, 
  ]);

  let mut block = Block::new();
  block.add_constraint(Constraint::Scan {table: table_id, column: 1, register: 1});
  block.add_constraint(Constraint::Scan {table: table_id, column: 2, register: 2});
  block.add_constraint(Constraint::Function {operation: Function::Add, parameters: vec![1, 2], output: vec![1]});
  block.add_constraint(Constraint::Insert {table: table_id, column: 3, register: 1});
  let plan = vec![
    Constraint::Function {operation: Function::Add, parameters: vec![1, 2], output: vec![1]},
    Constraint::Insert {table: table_id, column: 3, register: 1}
  ];
  block.plan = plan;
  let mut block2 = Block::new();
  



  let begin = SystemTime::now();
  
  let foo = db.runtime.register_block(block.clone(), &db.store);
  let foo2 = db.runtime.register_block(block2.clone(), &db.store);
  db.register_transaction(txn);  
  for i in 0..200 {
    let changes = db.process_transactions();
    if changes.len() > 0 {
      let txn2 = Transaction::from_changeset(changes);
      db.register_transaction(txn2);
    }
  }

  let end = SystemTime::now();

  
  //println!("q");
  
  let delta = end.duration_since(begin);

  println!("{:?}", db);
  println!("{:?}", db.runtime);
  
  println!("{:?}", delta);
  //loop{}
}