extern crate mech_core;

use mech_core::Hasher;
use mech_core::{Core, Transaction, Change};
use mech_core::{Runtime};


#[test]
fn create_runtime() {
    let runtime = Runtime::new();
    assert_eq!("", "");
}