extern crate mech;

use mech::Hasher;
use mech::{Core, Transaction, Change};
use mech::{Block, Constraint};
use mech::{Function};
use mech::{Value};

#[test]
fn math_add() {
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
  let mut block = Block::new();
  block.add_constraint(Constraint::Scan {table: math, column: 1, input: 1});
  block.add_constraint(Constraint::Scan {table: math, column: 2, input: 2});
  block.add_constraint(Constraint::Identity {source: 1, sink: 1});
  block.add_constraint(Constraint::Identity {source: 2, sink: 2});
  block.add_constraint(Constraint::Function {operation: Function::Add, parameters: vec![1, 2], output: 3});
  block.add_constraint(Constraint::Insert {output: 1, table: math, column: 3});
  block.plan();
  db.runtime.register_block(block.clone(), &mut db.store);
  db.process_transaction(&txn);
  db

}