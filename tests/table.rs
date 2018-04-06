extern crate mech;

use mech::eav::{Table, Value};
use mech::indexes::Hasher;

fn make_table() -> Table {
  let mut table = Table::new("students", 16, 16);
  
  let student1: u64 = Hasher::hash_str("Mark");
  let student2: u64 = Hasher::hash_str("Sabra");
  let first: u64 = Hasher::hash_str("first name");
  let last: u64 = Hasher::hash_str("last name");
  let test1: u64 = Hasher::hash_str("test1");
  let test2: u64 = Hasher::hash_str("test2");

  table.add_value(student1, first, Value::from_str("Mark"));
  table.add_value(student1, last, Value::from_str("Laughlin"));
  table.add_value(student1, test1, Value::from_u64(83));
  table.add_value(student1, test2, Value::from_u64(76));

  table.add_value(student2, first, Value::from_str("Sabra"));
  table.add_value(student2, last, Value::from_str("Kindar"));
  table.add_value(student2, test1, Value::from_u64(99));
  table.add_value(student2, test2, Value::from_u64(95));

  table
}


#[test]
fn get_a_row() {
    let mut table = make_table();
    let student1: u64 = Hasher::hash_str("Mark");
    let row = table.get_rows(vec![student1]);
    let answer = vec![Some(vec![
                    Value::from_str("Mark"),
                    Value::from_str("Laughlin"),
                    Value::from_u64(83),
                    Value::from_u64(76)])];
    assert_eq!(row, answer);
}

#[test]
fn get_multiple_rows() {
    let mut table = make_table();
    let student1: u64 = Hasher::hash_str("Mark");
    let student2: u64 = Hasher::hash_str("Sabra");
    let row = table.get_rows(vec![student1, student2]);
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
    let test1 = Hasher::hash_str("test1");
    let col = table.get_cols(vec![test1]);
    let answer = vec![Some(vec![
                      Value::from_u64(83),
                      Value::from_u64(99)]),
                    ];
    assert_eq!(col, answer);
}

#[test]
fn get_multiple_columns() {
    let mut table = make_table();
    let test1 = Hasher::hash_str("test1");
    let test2 = Hasher::hash_str("test2");
    let col = table.get_cols(vec![test1, test2]);
    let answer = vec![Some(vec![
                        Value::from_u64(83),
                        Value::from_u64(99)]),
                      Some(vec![
                        Value::from_u64(76),
                        Value::from_u64(95)])
                    ];
    assert_eq!(col, answer);
}