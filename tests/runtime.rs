extern crate mech;

use mech::indexes::Hasher;
use mech::database::{Database, Transaction, Change};
use mech::runtime::{Runtime};


#[test]
fn create_runtime() {
    let runtime = Runtime::new();
    assert_eq!("", "");
}