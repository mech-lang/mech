extern crate mech_core;
extern crate mech_utilities;
#[macro_use]
extern crate lazy_static;
use mech_core::*;

lazy_static! {
  static ref SEPARATOR: u64 = hash_str("separator");
  static ref STRING: u64 = hash_str("string");
  static ref TABLE: u64 = hash_str("table");
  static ref ROW: u64 = hash_str("row");
  static ref COLUMN: u64 = hash_str("column");
}

pub mod join;
pub mod length;
pub mod split;
