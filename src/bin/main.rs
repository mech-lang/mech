#![feature(get_mut_unchecked)]

extern crate mech_core;

//use mech_core::{Register, Table, TableId, Database, TableIterator, IndexRepeater, TableIndex, ValueIterator, Value, ValueMethods, Store, IndexIterator, ConstantIterator};
use std::sync::Arc;
use std::cell::RefCell;

extern crate seahash;
extern crate time;
extern crate nalgebra;
extern crate rayon;

use nalgebra::base::Matrix2;
use nalgebra::base::DMatrix;
use rayon::prelude::*;

use std::thread;
use std::time::Duration;

fn long_running_computation(x: &u64) -> u64 {
  let ten_millis = Duration::from_millis(10);
  let now = time::Instant::now();
  thread::sleep(ten_millis);
  x + 1
}

#[tokio::main]
pub async fn main() -> Result<(),String> {

  // New runtime
  // requirements:
  // pass all tests
  // robust units
  // all number types
  // Unicode
  // async blocks
  // parallel operators
  // rewind capability
  // pre-serialized memory layout
  // performance target: 10 million updates per 60Hz cycle
  // stack allocated tables
  // matrix library in std

  const n: usize = 1000;

  /*
  {
    let dm = DMatrix::from_element(n,1,0.0);
    let start_ns = time::precise_time_ns();
    let c = dm.add_scalar(1.0);
    let end_ns = time::precise_time_ns();
    let time = (end_ns - start_ns) as f64;
    println!("MATRIX {:0.9?} ms", time / 1_000_000.0);
  }*/
  {
    let mut v: Vec<u64> = vec![0;n];
    let start_ns = time::precise_time_ns();
    let x: Vec<u64> = v.iter().map(|x| long_running_computation(x)).collect();
    let end_ns = time::precise_time_ns();
    let time = (end_ns - start_ns) as f64;
    println!("ITER {:0.9?} ms", time / 1_000_000.0);
  }
  {
    let mut v: Vec<u64> = vec![0;n];
    let start_ns = time::precise_time_ns();
    let x: Vec<u64> = v.par_iter().map(|x| long_running_computation(x)).collect();
    let end_ns = time::precise_time_ns();
    let time = (end_ns - start_ns) as f64;
    println!("PARA {:0.9?} ms", time / 1_000_000.0);
  }

  Ok(())
}