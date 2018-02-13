extern crate mech;
extern crate core;


use std::collections::{BTreeSet, BTreeMap};
use mech::database::{Database, Transaction, Change, ChangeType};
use mech::eav::{Entity, Attribute, Value};
use mech::indexes::Hasher;

fn main() {
  let mut db = Database::new(1000, 1000);

  let mut raw = vec![("tag", Value::from_str("keyboard/event/keydown")),
                     ("key", Value::from_str("A")),
                     ("code", Value::from_u64(42))];
  let mut key = Entity::from_raw(raw);
  let mut changes = key.make_changeset(ChangeType::Add);
  let mut txn = Transaction::from_changeset(changes);

  db.register_transaction(txn);

println!("{:?}", db.entity_index);

  //raw = vec![("tag", Value::from_str("keyboard/event/keydown")),
  //               ("key", Value::from_str("A")),
  //               ("code", Value::from_u64(42))];
  //key = Entity::from_raw(raw);
  changes = key.make_changeset(ChangeType::Remove);
  txn = Transaction::from_changeset(changes);



  db.register_transaction(txn);


  println!("{:?}", db.entity_index);

}