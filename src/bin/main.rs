extern crate mech;
extern crate core;

use mech::database::{Database, Transaction, Change, ChangeType};
use mech::table::{Value, Table};
use mech::indexes::Hasher;

fn main() {

  let mut table = Table::new("students", 16, 16);
  
  let student1: u64 = Hasher::hash_str("Mark");
  let student2: u64 = Hasher::hash_str("Sabra");
  let first: u64 = Hasher::hash_str("first name");
  let last: u64 = Hasher::hash_str("last name");
  let test1: u64 = Hasher::hash_str("test1");
  let test2: u64 = Hasher::hash_str("test2");

  table.set(student1, first, Value::from_str("Mark"));
  table.set(student1, last, Value::from_str("Laughlin"));
  table.set(student1, test1, Value::from_u64(83));
  table.set(student1, test2, Value::from_u64(76));

  table.set(student2, first, Value::from_str("Sabra"));
  table.set(student2, last, Value::from_str("Kindar"));
  table.set(student2, test1, Value::from_u64(99));
  table.set(student2, test2, Value::from_u64(95));
  table.set(student2, first, Value::from_u64(100));

  println!("{:?}", table);

    println!("{:?}", table.get_rows(vec![student1]));
  println!("{:?}", table.get_cols(vec![first, test1, last, 3]));

  table.index(student1, test1);

  table.clear(student1, last);
  table.clear(student2, test2);

  println!("{:?}", table);


  //println!("{:?}", foo);
  //let mut my_value = Value::from_u64(100);
  //foo = &mut my_value;
  

  let mut db = Database::new(1000, 1000, 1000);

}