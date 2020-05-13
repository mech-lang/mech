extern crate mech_core;

use mech_core::{Table, Value, Index};
use mech_core::Hasher;
use std::rc::Rc;

fn make_table() -> Table {

  let tag: u64 = Hasher::hash_str("students");  

  let mut table = Table::new(tag, 2, 4);

  table.set_cell(&Index::Index(1), &Index::Index(1), Rc::new(Value::from_str("Mark")));
  table.set_cell(&Index::Index(1), &Index::Index(2), Rc::new(Value::from_str("Laughlin")));
  table.set_cell(&Index::Index(1), &Index::Index(3), Rc::new(Value::from_u64(83)));
  table.set_cell(&Index::Index(1), &Index::Index(4), Rc::new(Value::from_u64(76)));

  table.set_cell(&Index::Index(2), &Index::Index(1), Rc::new(Value::from_str("Sabra")));
  table.set_cell(&Index::Index(2), &Index::Index(2), Rc::new(Value::from_str("Kindar")));
  table.set_cell(&Index::Index(2), &Index::Index(3), Rc::new(Value::from_u64(99)));
  table.set_cell(&Index::Index(2), &Index::Index(4), Rc::new(Value::from_u64(95)));

  table
}

#[test]
fn index_into_cell() {
    let mut table = make_table();
    let score = table.index(&Index::Index(1), &Index::Index(3));
    assert_eq!(score, Some(&Value::from_u64(83)));
} 

#[test]
fn clear_cell() {
    let mut table = make_table();
    table.clear_cell(&Index::Index(1), &Index::Index(3));
    let score = table.index(&Index::Index(1), &Index::Index(3));
    assert_eq!(score, Some(&Value::Empty));
}