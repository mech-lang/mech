extern crate core;
extern crate time;
extern crate rand;

use std::time::SystemTime;
use std::thread::{self};
use std::time::*;
use rand::{Rng, thread_rng};

fn main() {
  println!("{:?}");
  /*
  let mut db = make_db(10);
  
  let system_timer_change = Hasher::hash_str("system/timer/change");
  let mut mean = 0.0;

  thread::spawn(move || {
    let mut i = 1;    
    loop {   
      let cur_time = time::now();
      
      let timer_id = 1;

      let txn = Transaction::from_changeset(vec![
        Change::Add{table: system_timer_change, row: timer_id, column: 1, value: Value::from_u64(cur_time.tm_hour as u64)},
        Change::Add{table: system_timer_change, row: timer_id, column: 2, value: Value::from_u64(cur_time.tm_min as u64)},
        Change::Add{table: system_timer_change, row: timer_id, column: 3, value: Value::from_u64(cur_time.tm_sec as u64)},
        Change::Add{table: system_timer_change, row: timer_id, column: 4, value: Value::from_u64(cur_time.tm_nsec as u64)},
      ]);     

      thread::sleep(Duration::from_millis(10));    

      let start_ns = time::precise_time_ns();  
      db.process_transaction(&txn);
      let end_ns = time::precise_time_ns();
      
      let delta = end_ns - start_ns;
      let delta_sec = delta as f64 / 1.0e9;
      let scaled = 0.001 / delta_sec;
      if db.capacity() >= 100.0 {
        mean += scaled as f64;
        i += 1;
      }
      
      println!("{:?}", db.runtime);
      println!("{:?}", db);
      println!("Mean Steady State Round Frequency: {:0.6} KHz", mean as f64 / (i as f64 - 1.0));
      println!("Capacity: {:0.2}%", db.capacity());
    }
    
  });

  //println!("{:?}", db);
  //println!("{:?}", db.runtime);
  loop{}
  */
}