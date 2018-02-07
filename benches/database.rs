#![feature(test)]

extern crate test;
extern crate mech;

use test::Bencher;
use mech::database::{Database, Transaction, Change, ChangeType, Value};

#[bench]
fn db_init(b:&mut Bencher) {
    b.iter(|| {
        let mut db = Database::new(1,1);
        db.init();
    });
}

#[bench]
fn db_register_txn(b: &mut Bencher) {
    let mut db = Database::new(1,1);
    db.init();
    let mut txns = generate_random_transaction(1, 1);
    b.iter(|| {
        db.register_transactions(&mut txns)
    });
}

#[bench]
fn db_register_txn_1000(b: &mut Bencher) {
    let mut db = Database::new(1000, 1000);
    db.init();
    let mut txns = generate_random_transaction(1000, 1);
    b.iter(|| {
        db.register_transactions(&mut txns);
    });
}

// ## Helper Functions

pub fn generate_random_transaction(transaction_count: usize, change_count: usize) -> Vec<Transaction> {
    let mut txn_vec = Vec::with_capacity(transaction_count);
    for _ in 0..transaction_count {
        let txn = generate_transaction(transaction_count * change_count);
        txn_vec.push(txn);
    }
    txn_vec
}

pub fn generate_transaction(change_count: usize) -> Transaction {
    let mut txn = Transaction::new();
    let changes = generate_changes(change_count);
    txn.adds = changes;
    txn
}

pub fn generate_changes(change_count: usize) -> Vec<Change> {
    let mut vec: Vec<Change> = Vec::with_capacity(change_count);
    for _ in 0..change_count {
        let entity = 0 as u64; 
        let change = Change {
            kind: ChangeType::Add,
            entity,   
            attribute: 0 as u64,
            value: Value::from_int(0),
            marked: false,
        };
        vec.push(change);
    }
    vec
}