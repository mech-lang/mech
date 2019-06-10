extern crate time;
use std::time::Duration;
use mech_program::{RunLoopMessage, Watcher};
use mech_core::{Interner, Transaction, Change};
use mech_core::Hasher;
use mech_core::Value;
use std::sync::mpsc::{self, Sender};
use std::thread::{self};
use std::collections::{HashMap};
use std::collections::hash_map::{Entry};

pub struct Timer {
  name: String,
  columns: usize,
  outgoing: Sender<RunLoopMessage>,
  timers: HashMap<u64, (usize, Sender<()>)>
}

impl Timer {
  pub fn new(outgoing: Sender<RunLoopMessage>) -> Timer {
    Timer { name: "time/timer".to_string(), outgoing, timers: HashMap::new(), columns: 10 }
  }
}

impl Watcher for Timer {
  fn get_name(& self) -> String {
    self.name.clone()
  }
  fn set_name(&mut self, name: &str) {
    self.name = name.to_string();
  }
  fn get_columns(&self) -> usize {
    self.columns
  }
  fn on_change(&mut self, interner: &mut Interner, diff: Transaction) {
    for remove in diff.removes {

    }
    for change in diff.names {
      match change {
        Change::RenameColumn{table, column_ix, column_alias} => {
          let resolution_column = Hasher::hash_str("period");
          if column_alias == resolution_column {
            let outgoing = self.outgoing.clone();
            let system_timer = Hasher::hash_str(&self.get_name());
            let duration = Duration::from_millis(1000);
            thread::spawn(move || {
              let mut tick = 0;
              let mut last = 0;
              loop {
                thread::sleep(duration); 
                let cur_time = time::now();
                let now = time::precise_time_ns();
                let txn = Transaction::from_changeset(vec![
                  //Change::Set{table, row, column: Hasher::hash_str("year"), value: Value::from_u64(cur_time.tm_year as u64 + 1900)},
                  //Change::Set{table, row, column: Hasher::hash_str("day"), value: Value::from_u64(cur_time.tm_mday as u64)},
                  //Change::Set{table, row, column: Hasher::hash_str("month"), value: Value::from_u64(cur_time.tm_mon as u64 + 1)},
                  //Change::Set{table, row, column: Hasher::hash_str("hour"), value: Value::from_u64(cur_time.tm_hour as u64)},
                  //Change::Set{table, row, column: Hasher::hash_str("minute"), value: Value::from_u64(cur_time.tm_min as u64)},
                  //Change::Set{table, row, column: Hasher::hash_str("second"), value: Value::from_u64(cur_time.tm_sec as u64)},
                  //Change::Set{table, row, column: Hasher::hash_str("nano-second"), value: Value::from_u64(cur_time.tm_nsec as u64)},
                  Change::Set{table: table.clone(), row: mech_core::Index::Index(1), column: mech_core::Index::Alias(Hasher::hash_str("tick")), value: Value::from_u64(tick)},
                  //Change::Set{table, row, column: Hasher::hash_str("dt"), value: Value::from_u64(now - last)},
                ]);     
                tick += 1;
                last = now;
                match outgoing.send(RunLoopMessage::Transaction(txn)) {
                  Err(_) => break,
                  _ => {}
                }
              }
            });
          }
        },
        _ => (),
      }
    }  
  }
}
