use std::sync::Arc;
use std::cell::RefCell;

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

use nalgebra::base::Matrix2;
use nalgebra::base::DMatrix;
use rayon::prelude::*;

use std::thread;
use std::time::Duration;

fn long_running_computation(x: &u64) -> u64 {
  //let ten_millis = Duration::from_micros(1);
  //thread::sleep(ten_millis);
  x + 2
}

#[tokio::main]
pub async fn main() -> Result<(),String> {

  const n: usize = 1000_000;
  {
    let mut v: Vec<u64> = vec![0;n];
    let start_ns = time::precise_time_ns();
    v.iter_mut().for_each(|x| {
      *x = *x + 2;
    });
    let end_ns = time::precise_time_ns();
    let time = (end_ns - start_ns) as f64;
    println!("BASE {:0.4?} ms", time / 1_000_000.0);
  }
  {
    let mut v: Vec<u64> = vec![0;n];
    let start_ns = time::precise_time_ns();
    let x: Vec<u64> = v.iter().map(|x| long_running_computation(x)).collect();
    let end_ns = time::precise_time_ns();
    let time = (end_ns - start_ns) as f64;
    println!("ITER {:0.4?} ms", time / 1_000_000.0);
  }
  {
    let mut v: Vec<u64> = vec![0;n];
    let start_ns = time::precise_time_ns();
    v.par_iter_mut().for_each(|x| {
      *x = long_running_computation(x);
    });
    let end_ns = time::precise_time_ns();
    let time = (end_ns - start_ns) as f64;
    println!("PARA {:0.4?} ms (for)", time / 1_000_000.0);
  }

  Ok(())
}