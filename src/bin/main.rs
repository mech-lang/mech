extern crate mech;
extern crate core;

use mech::database::{Database, Transaction, Change, ChangeType};
use mech::eav::{Entity, Attribute, Value, Table};
use mech::indexes::Hasher;

fn main() {

  let mut table = Table::new("keyboard/event/keydown", 16, 16);
  
  let me: u64 = Hasher::hash_str("This is me");
  let wife: u64 = Hasher::hash_str("This is my wife");
  let age: u64 = Hasher::hash_str("age");
  let first: u64 = Hasher::hash_str("first name");
  let last: u64 = Hasher::hash_str("last name");
  let gender: u64 = Hasher::hash_str("gender");

  table.add_value(me, first, Value::from_str("Corey"));
  table.add_value(wife, first, Value::from_str("Rachel"));
  table.add_value(me, last, Value::from_str("Montella"));
  table.add_value(me, age, Value::from_u64(31));
  table.add_value(wife, age, Value::from_u64(29));
  table.add_value(wife, last, Value::from_str("Montella"));
  table.add_value(wife, gender, Value::from_str("female"));
  table.add_value(me, gender, Value::from_str("male"));

  println!("{:?}", table);

  println!("{:?}", table.get_rows(wife));

  let mut db = Database::new(1000, 1000);

}