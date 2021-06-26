#![feature(get_mut_unchecked)]

extern crate mech_core;

use mech_core::{Register, Table, TableId, Database, TableIterator, IndexRepeater, TableIndex, ValueIterator, Value, ValueMethods, Store, IndexIterator, ConstantIterator};
use std::sync::Arc;
use std::cell::RefCell;

extern crate seahash;
extern crate time;
extern crate nalgebra;

use nalgebra::base::Matrix2;

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

  const n: usize = 1000;
  //let mut v: Vec<u32> = vec![0;n];
  //let mut v: [u64;n] = [0;n];
  let start_ns = time::precise_time_ns();
  let a = Matrix2::new(0.0, 1.0, 2.0, 3.0);
  let b = Matrix2::new(4.0, 5.0, 6.0, 7.0);
  let expected = Matrix2::new(0.0, 5.0, 12.0, 21.0);
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f64;
  println!("{:0.9?} ns", time);


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
