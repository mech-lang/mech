extern crate mech;

use mech::Hasher;
use mech::{Core, Transaction, Change};
use mech::{Runtime};


#[test]
fn create_runtime() {
    let runtime = Runtime::new();
    assert_eq!("", "");
}