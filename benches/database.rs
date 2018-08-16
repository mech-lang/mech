#![feature(test)]

extern crate test;
extern crate mech_core;

use test::Bencher;
use mech_core::{Core, Transaction, Change};
use mech_core::{Table, Value};
use mech_core::Hasher;

#[bench]
fn db_new(b:&mut Bencher) {
    b.iter(|| {
        let mut db = Core::new(1, 1);
    });
}

#[bench]
fn db_new_200_000(b:&mut Bencher) {
    b.iter(|| {
        let mut db = Core::new(200000,200000);
    });
}

#[bench]
fn db_register_table(b: &mut Bencher) {
    let mut db = Core::new(1, 1);
    b.iter(|| {
        let txn = Transaction::from_changeset(vec![
            Change::NewTable{ id: 0, rows: 10, columns: 10 },
        ]);
        db.process_transaction(&txn);
    });
}

#[bench]
fn db_register_add(b: &mut Bencher) {
    let mut db = Core::new(1, 1);
    let students: u64 = Hasher::hash_str("students");  
    let txn = Transaction::from_change(
        Change::NewTable{ id: 0, rows: 10, columns: 10 },
    );
    b.iter(|| {
        let txn = Transaction::from_change(
            Change::Set{table: students, row: 1, column: 1, value: Value::from_u64(100)}
        );
        db.process_transaction(&txn);
    });
}