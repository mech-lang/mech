extern crate mech;

use mech::Hasher;
use mech::{Core, Transaction, Change};

#[test]
fn create_database() {
    let db = Core::new(1,1);
    assert_eq!("", "");
}