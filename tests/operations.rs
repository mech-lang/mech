extern crate mech;

use mech::indexes::Hasher;
use mech::database::{Database, Transaction, Change};
use mech::runtime::{Block, Constraint};
use mech::operations::{Function};
use mech::table::{Value};

#[test]
fn math_add() {
    let ball = Hasher::hash_str("ball");
    let db = make_db();
    let col = db.store.get_column(ball, 5);
    let val = vec![Value::from_u64(3)];
    assert_eq!(col, Some(&val));
}

fn make_ball(row2: usize) -> Vec<Change> {
  let row = row2 as u64;
  let ball = Hasher::hash_str("ball");
  vec![
    Change::Add{table: ball, row, column: 1, value: Value::from_u64(1)},
    Change::Add{table: ball, row, column: 2, value: Value::from_u64(2)},
    Change::Add{table: ball, row, column: 3, value: Value::from_u64(3)},
    Change::Add{table: ball, row, column: 4, value: Value::from_u64(4)},
    Change::Add{table: ball, row, column: 5, value: Value::from_u64(16)},
  ]
}

fn make_db() -> Database {

  let ball = Hasher::hash_str("ball");
  let mut db = Database::new(1,1,1);

  let txn = Transaction::from_changeset(vec![
    Change::NewTable{tag: ball, rows: 1, columns: 5}, 
    Change::Add{table: ball, row: 1, column: 1, value: Value::from_u64(1)},
    Change::Add{table: ball, row: 1, column: 2, value: Value::from_u64(2)},
    Change::Add{table: ball, row: 1, column: 3, value: Value::from_u64(3)},
    Change::Add{table: ball, row: 1, column: 4, value: Value::from_u64(4)},
  ]); 
 
  // Make a block
  let mut block = Block::new();
  block.add_constraint(Constraint::Scan {table: ball, column: 1, register: 1});
  block.add_constraint(Constraint::Scan {table: ball, column: 2, register: 2});
  block.add_constraint(Constraint::Function {operation: Function::Add, parameters: vec![1, 2], output: 1});
  block.add_constraint(Constraint::Insert {table: ball, column: 3, register: 1});
  let plan = vec![
    Constraint::Function {operation: Function::Add, parameters: vec![1, 2], output: 1},
    Constraint::Insert {table: ball, column: 5, register: 1},
  ];
  block.plan = plan;

  db.runtime.register_block(block.clone(), &mut db.store);
  let changes = db.process_transaction(&txn);
  let txn2 = Transaction::from_changeset(changes);
  db.process_transaction(&txn2);
  db

}