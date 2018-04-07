#![feature(test)]

extern crate test;
extern crate mech;

use test::Bencher;
use mech::database::{Database, Transaction, Change, ChangeType};
use mech::table::{Entity, Attribute, Value};

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
        let raw = vec![("tag", Value::from_str("keyboard/event/keydown")),
                           ("key", Value::from_str("A")),
                           ("code", Value::from_u64(42))];
        let key = Entity::from_raw(raw);
        let changes = key.make_changeset(ChangeType::Add);
        let txn = Transaction::from_changeset(changes);
        db.register_transaction(txn);
    });
}

#[bench]
// Register 1000 txns
fn db_register_txn_1000(b: &mut Bencher) {
    let mut db = Database::new(1,1);
    db.init();
    b.iter(|| {
        for _ in 1..1000 {
            let raw = vec![("tag", Value::from_str("keyboard/event/keydown")),
                            ("key", Value::from_str("A")),
                            ("code", Value::from_u64(14))];
            let key = Entity::from_raw(raw);
            let changes = key.make_changeset(ChangeType::Add);
            let txn = Transaction::from_changeset(changes);    
            db.register_transaction(txn);
        }
    });
}


/*
#[bench]
fn db_register_txn_1000(b: &mut Bencher) {
    let n = 1_000;
    let mut txns = generate_random_transaction(n, 1);
    b.iter(|| {
        let mut db = Database::new(n, n);
        db.init();
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
            value: Value::from_u64(0),
            transaction: 0,
        };
        vec.push(change);
    }
    vec
}*/