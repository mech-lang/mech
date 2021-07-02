#![feature(get_mut_unchecked)]

extern crate mech_core;

use mech_core::{Register, Table, TableId, Database, TableIterator, IndexRepeater, TableIndex, ValueIterator, Value, ValueMethods, Store, IndexIterator, ConstantIterator};
use std::sync::Arc;
use std::cell::RefCell;

extern crate seahash;
extern crate time;
extern crate nalgebra;

use nalgebra::base::Matrix2;
use nalgebra::base::DMatrix;

fn main() {

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

  const n: usize = 10_000_000;
  let dm = DMatrix::from_element(n,1,1.0);
  let dx = DMatrix::from_element(n,1,2.0); 
  
  //let mut v: Vec<u32> = vec![0;n];
  //let mut v: [u64;n] = [0;n];
  let start_ns = time::precise_time_ns();
  let c = dm + dx;
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f64;
  println!("{:0.9?} ms", time / 1_000_000.0);

/*
    let n = 10)=000;
    let mut v: Vec<u64> = vec![0;n];
    //println!("{:?}", v);
    let start_ns = time::precise_time_ns();
    let x: Vec<u64> = v.par_iter().map(|x| x + 1).collect();
    let end_ns = time::precise_time_ns();
    let time = (end_ns - start_ns) as f64;
    println!("PARI {:0.9?} ms", time / 1_000_000.0);
    //println!("{:?}",x);
  
    let mut v: Vec<u64> = vec![0;n];
    let start_ns = time::precise_time_ns();
    let x: Vec<u64> = v.iter().map(|x| x + 1).collect();
    let end_ns = time::precise_time_ns();
    let time = (end_ns - start_ns) as f64;
    println!("ITER {:0.9?} ms", time / 1_000_000.0);*/



  
}
