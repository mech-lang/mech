extern crate mech;
extern crate core;


use std::collections::{BTreeSet, BTreeMap};
use mech::database::{Database, Transaction, Change, ChangeType};
use mech::eav::{Entity, Attribute, Value, Table};
use mech::indexes::Hasher;

fn main() {

  let mut table = Table::new("keyboard/event/keydown", 16, 16);
  
  table.add_value(&0, &0, Value::from_u64(10));
  table.add_value(&0, &1, Value::from_u64(11));
  table.add_value(&1, &0, Value::from_u64(31));
  table.add_value(&1, &2, Value::from_u64(99));
  table.add_value(&2, &1, Value::from_u64(75));
  table.add_value(&1, &1, Value::from_u64(83));
  table.add_value(&1, &1, Value::from_u64(29));

  println!("{:?}", table);

  let mut db = Database::new(1000, 1000);

  




  /*let mut raw = vec![("tag", Value::from_str("keyboard/event/keydown")),
                     ("key", Value::from_str("A")),
                     ("code", Value::from_u64(42))];
  let mut key = Entity::from_raw(raw);
  let mut changes = key.make_changeset(ChangeType::Add);
  let mut txn = Transaction::from_changeset(changes);

  db.register_transaction(txn);

  println!("Entities:\n{:?}", db.entity_index);

  //raw = vec![("tag", Value::from_str("keyboard/event/keydown")),
  //               ("key", Value::from_str("A")),
  //               ("code", Value::from_u64(42))];
  //key = Entity::from_raw(raw);
  changes = key.make_changeset(ChangeType::Remove);
  txn = Transaction::from_changeset(changes);



  db.register_transaction(txn);*/


  //println!("Entities:\n{:?}", db.entity_index);

}