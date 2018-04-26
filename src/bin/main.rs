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
  let students: u64 = Hasher::hash_str("students");  
  let student1: u64 = Hasher::hash_str("Mark");
  let student2: u64 = Hasher::hash_str("Sabra");
  let test1: u64 = Hasher::hash_str("test1");
  let test2: u64 = Hasher::hash_str("test2");
  let result: u64 = Hasher::hash_str("result");

  let txn = Transaction::from_changeset(vec![
    Change::Add{ix: 0, table: students, entity: student1, attribute: test1, value: Value::from_u64(83)}, 
    Change::NewTable{tag: String::from("students"), entities: vec![], attributes: vec![], rows: 10, cols: 10}, 
    Change::Add{ix: 0, table: students, entity: student1, attribute: test2, value: Value::from_u64(76)},
    Change::Add{ix: 0, table: students, entity: student2, attribute: test1, value: Value::from_u64(99)},
    Change::Add{ix: 0, table: students, entity: student2, attribute: test2, value: Value::from_u64(88)},
  ]);

  let mut block = Block::new();
  block.add_constraint(Constraint::Scan {table: students, attribute: test1, register: 1});
  block.add_constraint(Constraint::Scan {table: students, attribute: test2, register: 2});
  block.add_constraint(Constraint::Function {operation: Function::Add, parameters: vec![1, 2], output: vec![1]});
  block.add_constraint(Constraint::Insert {table: students, attribute: result, register: 1});
  let plan = vec![
    Constraint::Function {operation: Function::Add, parameters: vec![1, 2], output: vec![1]},
    Constraint::Insert {table: students, attribute: result, register: 1}
  ];
  block.plan = plan;
  let mut block2 = Block::new();
  
  let begin = SystemTime::now();


  println!("{:?}", txn);

  db.register_transaction(txn);  
  let foo = db.runtime.register_block(block.clone(), &db.store);
  let foo2 = db.runtime.register_block(block2.clone(), &db.store);

  let txn2 = Transaction::from_changeset(foo);
  db.register_transaction(txn2);
  
  println!("{:?}", db);
  println!("{:?}", db.runtime);

  let end = SystemTime::now();
  let delta = end.duration_since(begin);

  
  println!("{:?}", delta);
  loop{}
}