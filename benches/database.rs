#![feature(test)]

extern crate test;
extern crate mech;

use test::Bencher;
use mech::database::{Database, Transaction, Change, ChangeType};
use mech::table::{Table, Value};

#[bench]
fn db_init(b:&mut Bencher) {
    b.iter(|| {
        let mut db = Database::new(1,1);
        db.init();
    });
}

#[bench]
fn db_init_200_000(b:&mut Bencher) {
    b.iter(|| {
        let mut db = Database::new(200000,200000);
        db.init();
    });
}

#[bench]
fn db_register_txn(b: &mut Bencher) {
    let mut db = Database::new(1,1);
    db.init();
    b.iter(|| {
        /*let raw = vec![("tag", Value::from_str("keyboard/event/keydown")),
                       ("key", Value::from_str("A")),
                           ("code", Value::from_u64(42))];
        let key = Entity::from_raw(raw);
        let changes = key.make_changeset(ChangeType::Add);
        let txn = Transaction::from_changeset(changes);
        db.register_transaction(txn);*/
    });
}

#[bench]
// Register 1000 txns
fn db_register_txn_1000(b: &mut Bencher) {
    let mut db = Database::new(1,1);
    db.init();
    b.iter(|| {
        /*for _ in 1..1000 {
            let raw = vec![("tag", Value::from_str("keyboard/event/keydown")),
                            ("key", Value::from_str("A")),
                            ("code", Value::from_u64(14))];
            let key = Entity::from_raw(raw);
            let changes = key.make_changeset(ChangeType::Add);
            let txn = Transaction::from_changeset(changes);    
        }*/
    });
}