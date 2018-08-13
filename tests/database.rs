extern crate mech_core;

use mech_core::Hasher;
use mech_core::{Core, Transaction, Change};

#[test]
fn create_database() {
    let db = Core::new(1,1);
    assert_eq!("", "");
}