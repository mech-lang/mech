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

fn main() {
    let ball = Hasher::hash_str("math");
    let db = make_db();
    let col = db.store.get_column(ball, 3);
    let val = vec![Value::from_u64(3)];
    assert_eq!(col, Some(&val));
}

fn make_db() -> Core {
  
  let math = Hasher::hash_str("math");
  let mut db = Core::new(1,1);

  let txn = Transaction::from_changeset(vec![
    Change::NewTable{tag: math, rows: 1, columns: 3}, 
    Change::Add{table: math, row: 1, column: 1, value: Value::from_u64(1)},
    Change::Add{table: math, row: 1, column: 2, value: Value::from_u64(2)},
  ]); 
 
  // Make a block
  let block = 

  db.runtime.register_block(block.clone(), &mut db.store);
  db.process_transaction(&txn);
  db

}