extern crate mech_core;
extern crate mech_program;
use mech_core::{Interner, Transaction};
use mech_core::Value;

// ## Watchers

pub trait Watcher {
  fn get_name(& self) -> String;
  fn get_columns(& self) -> usize;
  fn set_name(&mut self, &str);
  fn on_change(&mut self, store: &mut Interner, diff: Transaction);
}

pub mod timer;