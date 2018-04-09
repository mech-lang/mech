#![feature(test)]

extern crate test;
extern crate mech;

use test::Bencher;
use mech::database::{Database, Transaction, Change, AddChange, NewTableChange};
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
    let students: u64 = Hasher::hash_str("students");  
    let student1: u64 = Hasher::hash_str("Mark");
    let test1: u64 = Hasher::hash_str("test1");
    b.iter(|| {
        let t1 = NewTableChange::new(String::from("students"), vec![], vec![], 10, 10);
        let c1 = AddChange::new(students, student1, test1, Value::from_u64(100));
        let txn = Transaction::from_changeset(vec![
            Change::Add(c1), 
            Change::NewTable(t1)]);
        db.register_transaction(txn);
    });
}

#[bench]
fn db_register_transaction(b: &mut Bencher) {
    let mut db = Database::new(1, 1, 1);
    let students: u64 = Hasher::hash_str("students");  
    let student1: u64 = Hasher::hash_str("Mark");
    let test1: u64 = Hasher::hash_str("test1");
    b.iter(|| {
        let t1 = NewTableChange::new(String::from("students"), vec![], vec![], 10, 10);
        let c1 = AddChange::new(students, student1, test1, Value::from_u64(100));
        let txn = Transaction::from_changeset(vec![
            Change::Add(c1), 
            Change::NewTable(t1)]);
        db.register_transaction(txn);
    });
}


#[bench]
// Register 1000 txns
fn db_register_txn_1000(b: &mut Bencher) {
    let mut db = Database::new(1, 1, 1);
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