extern crate mech_core;

use mech_core::{Table, Value};
use mech_core::Hasher;

fn make_table() -> Table {

  let tag: u64 = Hasher::hash_str("students");  

  let mut table = Table::new(tag, 2, 16);

  table.grow_to_fit(2, 4);

  table.set_cell(1, 1, Value::from_str("Mark"));
  table.set_cell(1, 2, Value::from_str("Laughlin"));
  table.set_cell(1, 3, Value::from_u64(83));
  table.set_cell(1, 4, Value::from_u64(76));

  table.set_cell(2, 1, Value::from_str("Sabra"));
  table.set_cell(2, 2, Value::from_str("Kindar"));
  table.set_cell(2, 3, Value::from_u64(99));
  table.set_cell(2, 4, Value::from_u64(95));

  table
}

#[test]
fn get_a_row() {
    let mut table = make_table();
    let row = table.get_row(1);
    let answer = Some(vec![
                    Value::from_str("Mark"),
                    Value::from_str("Laughlin"),
                    Value::from_u64(83),
                    Value::from_u64(76)]);
    assert_eq!(row, answer);
}

#[test]
fn get_multiple_rows() {
    let mut table = make_table();
    let row = table.get_rows(vec![1, 2]);
    let answer = vec![Some(vec![
                      Value::from_str("Mark"),
                      Value::from_str("Laughlin"),
                      Value::from_u64(83),
                      Value::from_u64(76)]),
                    Some(vec![
                      Value::from_str("Sabra"),
                      Value::from_str("Kindar"),
                      Value::from_u64(99),
                      Value::from_u64(95),])
                    ];
    assert_eq!(row, answer);
}

#[test]
fn get_a_column() {
    let mut table = make_table();
    let col = table.get_column_by_ix(3);
    let col2 = vec![Value::from_u64(83), Value::from_u64(99)];
    let answer = Some(&col2);
    assert_eq!(col, answer);
}

#[test]
fn get_multiple_columns() {
    let mut table = make_table();
    let col = table.get_columns_by_ixes(vec![3, 4]);
    let c1 = vec![Value::from_u64(83), Value::from_u64(99)];
    let c2 = vec![Value::from_u64(76), Value::from_u64(95)];
    let answer = vec![Some(&c1), Some(&c2)];
    assert_eq!(col, answer);
}

#[test]
fn index_into_cell() {
    let mut table = make_table();
    let score = table.index(1, 3);
    assert_eq!(score, Some(&Value::from_u64(83)));
} 

#[test]
fn clear_cell() {
    let mut table = make_table();
    table.clear_cell(1, 3);
    let score = table.index(1, 3);
    assert_eq!(score, Some(&Value::Empty));
} 


#[test]
fn handle_set_zero() {
    let mut table = make_table();
    let result = table.set_cell(0, 1, Value::from_str("Mark"));
    assert_eq!(result, Err("Index out of table bounds."));
} 