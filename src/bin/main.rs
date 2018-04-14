extern crate mech;
extern crate core;
extern crate time;

use mech::database::{Database, Transaction, Change, AddChange, NewTableChange};
use mech::table::{Value, Table};
use mech::indexes::Hasher;
use std::thread::{self};
use std::time::*;
use mech::runtime::{Runtime, Block, Constraint, Register};

fn main() {

  let tag: u64 = Hasher::hash_str("students");  
  let teachers: u64 = Hasher::hash_str("teachers");
  let student1: u64 = Hasher::hash_str("Mark");
  let student2: u64 = Hasher::hash_str("Sabra");
  let first: u64 = Hasher::hash_str("first name");
  let last: u64 = Hasher::hash_str("last name");
  let test1: u64 = Hasher::hash_str("test1");
  let test2: u64 = Hasher::hash_str("test2");

  let mut table = Table::new(tag, 16, 16);

  table.set(student1, first, Value::from_str("Mark"));
  table.set(student1, last, Value::from_str("Laughlin"));
  table.set(student1, test1, Value::from_u64(83));
  table.set(student1, test2, Value::from_u64(76));

  table.set(student2, first, Value::from_str("Sabra"));
  table.set(student2, last, Value::from_str("Kindar"));
  table.set(student2, test1, Value::from_u64(99));
  table.set(student2, test2, Value::from_u64(95));
  table.set(student2, first, Value::from_u64(100));

  //println!("{:?}", table);

  //println!("{:?}", table.get_rows(vec![student1]));
  //println!("{:?}", table.get_cols(vec![first, test1, last, 3]));

  table.index(student1, test1);

  table.clear(student1, last);
  table.clear(student2, test2);

  //println!("{:?}", table);


  //println!("{:?}", foo);
  //let mut my_value = Value::from_u64(100);
  //foo = &mut my_value;

  let mut db = Database::new(1000, 1000, 1000);

  
  let mut block = Block::new();
  block.id = 1;
  block.constraints.push(Constraint::Scan {table: tag, attribute: test1});
  block.constraints.push(Constraint::Scan {table: tag, attribute: test2});
  db.runtime.register_block(block.clone(), &db.store);



  let c1 = AddChange::new(tag, student1, first, Value::from_str("Mark"));
  let c2 = AddChange::new(tag, student1, last, Value::from_str("Laughlin"));
  let c3 = AddChange::new(tag, student1, test1, Value::from_u64(83));
  let c4 = AddChange::new(tag, student1, test1, Value::from_u64(76));
  let c5 = AddChange::new(teachers, student2, first, Value::from_str("Sabra"));
  let t1= NewTableChange::new(String::from("students"), vec![], vec![], 10, 10);
  let t2= NewTableChange::new(String::from("teachers"), vec![], vec![], 10, 10);
  let txn = Transaction::from_changeset(vec![
    Change::Add(c1), 
    Change::NewTable(t1), 
    Change::Add(c3), 
    Change::Add(c2)]);
  println!("{:?}", txn);
  db.register_transaction(txn);
  println!("{:?}", db);
  let txn = Transaction::from_changeset(vec![
    Change::Add(c4),
    Change::Add(c5),
    Change::NewTable(t2)]);
  println!("{:?}", txn);
  db.register_transaction(txn);
  println!("{:?}", db);
  /*for i in 0 .. 1_000_000 {
    let c = AddChange::new(tag, student1, test1, Value::from_u64(i as u64));
    let t = Transaction::from_change(Change::Add(c));
    db.register_transaction(t);
  }*/
  
  println!("{:?}", db);
 
  let mut time = Hasher::hash_str("system/time/change");
  let mut day = Hasher::hash_str("day");
  let mut month = Hasher::hash_str("month");
  let mut weekday = Hasher::hash_str("day of week");
  let mut year = Hasher::hash_str("year");
  let mut hour = Hasher::hash_str("hour");
  let mut minute = Hasher::hash_str("minute");
  let mut second = Hasher::hash_str("second");
  let mut nsecond = Hasher::hash_str("nanosecond");
  let mut tic = Hasher::hash_str("tic");
  let systemTimeTable = NewTableChange::new(String::from("system/time/change"), vec![], vec![], 10, 9);

  let txn = Transaction::from_change(Change::NewTable(systemTimeTable));
  println!("{:?}", txn);
  db.register_transaction(txn);



  
  /*
  thread::spawn(move || {
    let mut i = 1;
    loop {
      thread::sleep(Duration::from_millis(1000));
      let cur_time = time::now();
      let timer_id = 1;
      let txn = Transaction::from_changeset(vec![
        Change::Add(AddChange::new(time, timer_id, year, Value::from_u64((cur_time.tm_year + 1900) as u64))),
        Change::Add(AddChange::new(time, timer_id, month, Value::from_u64((cur_time.tm_mon + 1) as u64))),
        Change::Add(AddChange::new(time, timer_id, day, Value::from_u64(cur_time.tm_mday as u64))),
        Change::Add(AddChange::new(time, timer_id, weekday, Value::from_u64(cur_time.tm_wday as u64))),
        Change::Add(AddChange::new(time, timer_id, hour, Value::from_u64(cur_time.tm_hour as u64))),
        Change::Add(AddChange::new(time, timer_id, minute, Value::from_u64(cur_time.tm_min as u64))),
        Change::Add(AddChange::new(time, timer_id, second, Value::from_u64(cur_time.tm_sec as u64))),
        Change::Add(AddChange::new(time, timer_id, nsecond, Value::from_u64(cur_time.tm_nsec as u64))),
        Change::Add(AddChange::new(time, timer_id, tic, Value::from_u64(i))),
      ]);
      println!("{:?}", txn);
      db.register_transaction(txn);
      println!("{:?}", db);
      i += 1;
    }
  });

  loop{}*/
}