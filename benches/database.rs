#![feature(test)]

extern crate test;
extern crate mech;

use test::Bencher;
use mech::database::{Database, Transaction, Change};
use mech::table::{Table, Value};
use mech::indexes::Hasher;

#[bench]
fn db_new(b:&mut Bencher) {
    b.iter(|| {
        let mut db = Database::new(1, 1, 1);
    });
}

#[bench]
fn db_new_200_000(b:&mut Bencher) {
    b.iter(|| {
        let mut db = Database::new(200000, 200000,200000);
    });
}

#[bench]
fn db_register_table(b: &mut Bencher) {
    let mut db = Database::new(1, 1, 1);
    b.iter(|| {
        let txn = Transaction::from_changeset(vec![
            Change::NewTable{ tag: String::from("students"), entities: vec![], attributes: vec![], rows: 10, columns: 10 },
        ]);
        db.register_transaction(txn);
    });
}

#[bench]
fn db_register_add(b: &mut Bencher) {
    let mut db = Database::new(1, 1, 1);
    let students: u64 = Hasher::hash_str("students");  
    let txn = Transaction::from_change(
        Change::NewTable{ tag: String::from("students"), entities: vec![], attributes: vec![], rows: 10, columns: 10 },
    );
    b.iter(|| {
        let txn = Transaction::from_change(
            Change::Add{ix: 0, table: students, row: 1, column: 1, value: Value::from_u64(100)}
        );
        db.register_transaction(txn);
    });
}