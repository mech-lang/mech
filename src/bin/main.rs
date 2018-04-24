extern crate mech;
extern crate core;

use mech::database::{Database, Transaction, Change, AddChange, NewTableChange};
use mech::table::{Value, Table};
use mech::indexes::Hasher;
use mech::runtime::{Runtime, Block, Constraint, Register};

fn main() {

  let mut db = Database::new(1000, 1000, 1000);
  let students: u64 = Hasher::hash_str("students");  
  let student1: u64 = Hasher::hash_str("Mark");
  let student2: u64 = Hasher::hash_str("Sabra");
  let test1: u64 = Hasher::hash_str("test1");
  let test2: u64 = Hasher::hash_str("test2");
  let result: u64 = Hasher::hash_str("result");

  let c1 = AddChange::new(students, student1, test1, Value::from_u64(83));
  let c2 = AddChange::new(students, student1, test2, Value::from_u64(76));
  let c3 = AddChange::new(students, student2, test1, Value::from_u64(99));
  let c4 = AddChange::new(students, student2, test2, Value::from_u64(88));
  let t1= NewTableChange::new(String::from("students"), vec![], vec![], 10, 10);
  let txn = Transaction::from_changeset(vec![
    Change::Add(c1), 
    Change::NewTable(t1), 
    Change::Add(c2),
    Change::Add(c3),
    Change::Add(c4)
  ]);
  
  let mut block = Block::new();
  block.add_constraint(Constraint::Scan {table: students, attribute: test1, register: 2});
  block.add_constraint(Constraint::Scan {table: students, attribute: test2, register: 1});
  block.add_constraint(Constraint::Insert {table: students, attribute: result, register: 1});
  block.add_constraint(Constraint::Function {op: 1, parameters: vec![1, 2], output: vec![3]});
  db.register_transaction(txn);
  db.runtime.register_block(block.clone(), &db.store);
  
  println!("{:?}", db);
  println!("{:?}", db.runtime);

}