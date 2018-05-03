extern crate mech;

use mech::indexes::Hasher;
use mech::database::{Database, Transaction, Change};

#[test]
fn create_database() {
    let db = Database::new(1,1);
    assert_eq!("", "");
}