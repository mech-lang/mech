extern crate mech;

use std::collections::{BTreeSet, BTreeMap};
use std::num::Wrapping;
use mech::database::{Database, Transaction, Change, ChangeType, Value};

fn main() {
    
    println!("Starting:");
    
    // Init the DB
    tic();
    let mut db = Database::new();
    db.init();
    toc();

    let n = 100_000;
    let avg_txn = 50;
    let avg_change = 30;
    let mut txn_time: Vec<f64> = Vec::new();
    let mut gen_time: Vec<f64> = Vec::new();
    let start = tic();
    for i in 0..n {
        // Generate a random transaction
        tic();
        let mut txns = generate_random_transaction(avg_txn, avg_change);
        gen_time.push(toc());

        // Process transactions
        tic();
        db.register_transactions(&mut txns);
        txn_time.push(toc());
    }

    // Do it again
    let stop = tic();
    println!("Finished!");
    let run_time = (stop - start) / 1000.0;
    println!("Runtime: {:?}", run_time);
    println!("{:?}", db);
    let avg_gen_time = gen_time.into_iter().fold(0.0, |r,x| r + x);
    println!("Gen Time: {:?}", avg_gen_time / n as f64);
    let avg_txn_time = txn_time.into_iter().fold(0.0, |r,x| r + x);
    println!("Insert Time: {:?}", avg_txn_time / n as f64);
    println!("kTxns/s: {:?}", db.transactions.len() as f64 / run_time / 1000.0);

}

pub fn generate_random_transaction(m: u32, n: u32) -> Vec<Transaction> {
    let mut seed = tic() as u32;
    let r1 = rand_between(&mut seed, 1, m);
    let r2 = rand_between(&mut seed, 1, n);      
    let mut txn_vec = Vec::with_capacity(r1 as usize);
    for i in 0..r1 {
        let txn = generate_transaction(r2);
        txn_vec.push(txn);
    }
    txn_vec
}

pub fn generate_transaction(n: u32) -> Transaction {

    let mut txn = Transaction::new();
    let changes = generate_changes(n);
    txn.adds = changes;
    txn

}

pub fn generate_changes(n: u32) -> Vec<Change> {
    let mut vec: Vec<Change> = Vec::with_capacity(n as usize);
    for i in 0..n {
        let entity = 0 as u64; 
        let change = Change {
            kind: ChangeType::Add,
            entity,   
            attribute: i as u64,
            value: Value::from_int(0),
            marked: false,
        };
        vec.push(change);
    }
    vec
}


static mut TIC: u64 = 0;
static mut TOC: u64 = 0;

pub fn tic() -> f64 {
    unsafe {
        //TIC = time::precise_time_ns();
        TIC as f64 / 1_000_000.0
    }
}

pub fn toc() -> f64 {
    unsafe {
        //TOC = time::precise_time_ns();
        let dt = (TOC - TIC) as f64 / 1_000_000.0;
        //println!("{:?}", dt);
        dt
    }
}


fn rand(rseed:&mut u32) -> u32 {
    //*rseed = ((Wrapping(*rseed) * Wrapping(1103515245) + Wrapping(12345)) & Wrapping(0x7fffffff)).0;
    //return *rseed;
    0
}

fn rand_between(rseed:&mut u32, from:u32, to:u32) -> u32 {
    //rand(rseed);
    //let range = (to - from) + 1;
    //from + *rseed % range
    0
}