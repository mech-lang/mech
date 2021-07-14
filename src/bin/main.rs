
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
use tokio::time::{sleep,Duration};
use tokio_stream;
use futures::future::join_all;

fn advance(x: &Vec<i64>, vx: &Vec<i64>) -> Vec<i64> {
  x.iter().zip(vx).map(|(x,y)| x + y).collect()
}

fn accelerate(vx: &Vec<i64>, gravity: &Vec<i64>) -> Vec<i64> {
  vx.iter().map(|x| x + gravity[0]).collect()
}

fn filter1(x: &Vec<i64>) -> Vec<i64> {
  x.iter().map(|x| (*x < 100) as i64).collect()
}

fn filter2(x: &Vec<i64>) -> Vec<i64> {
  x.iter().map(|x| (*x > 500) as i64).collect()
}

fn bounce1(x: &Vec<i64>, ix: &Vec<i64>) -> Vec<i64> {
  x.iter().zip(ix).map(|(x,y)| if *y == 1 {
    100
  } else {
    *x
  }).collect()
}

fn bounce2(x: &Vec<i64>, ix: &Vec<i64>) -> Vec<i64> {
  x.iter().zip(ix).map(|(x,y)| if *y == 1 {
    500
  } else {
    *x
  }).collect()
}

fn bounce3(vx: &Vec<i64>, ix: &Vec<i64>) -> Vec<i64> {
  vx.iter().zip(ix).map(|(x,y)| if *y == 1 {
    -*x
  } else {
    *x
  }).collect()
}

fn dampen(vx: &Vec<i64>, ix: &Vec<i64>) -> Vec<i64> {
  vx.iter().zip(ix).map(|(x,y)| if *y == 1 {
    *x * 90 / 100
  } else {
    *x
  }).collect()
}

async fn do_y(y: Vec<i64>, vy: Vec<i64>) -> (Vec<i64>,Vec<i64>) {
  let gravity = vec![1];
  let y2 = advance(&y,&vy);
  let vy2 = advance(&vy,&gravity);
  let iy1 = filter1(&y2);
  let iy2 = filter2(&y2);
  let y3= bounce1(&y2,&iy1);
  let y4 = bounce2(&y3,&iy2);
  let vy3 = bounce3(&vy2, &iy1);
  let vy4 = bounce3(&vy3, &iy2);
  let vy5 = dampen(&vy4, &iy2);
  (y4,vy5)
}

async fn do_x(x: Vec<i64>, vx: Vec<i64>) -> (Vec<i64>,Vec<i64>) {
  let x2 = advance(&x,&vx);
  let ix1 = filter1(&x2);
  let ix2 = filter2(&x2);
  let x3 = bounce1(&x2,&ix1);
  let x4 = bounce2(&x3,&ix2);
  let vx2 = bounce3(&vx, &ix1);
  let vx3 = bounce3(&vx2, &ix2);
  (x4,vx3)
}

use tokio_stream::StreamExt;

#[tokio::main]
async fn main() {
  let sizes: Vec<usize> = vec![1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7].iter().map(|x| *x as usize).collect();
  
  let start_ns0 = time::precise_time_ns();
  for n in sizes {
    for _ in 0..1 {
    let x: Vec<i64> = vec![0;n];
    let y: Vec<i64> = vec![0;n];
    let vx: Vec<i64> = vec![1;n];
    let vy: Vec<i64> = vec![1;n];
    let bounds: Vec<i64> = vec![500, 500];
    
    let x_fut = tokio::task::spawn(do_x(x,vx));
    let y_fut = tokio::task::spawn(do_y(y,vy));

    let start_ns = time::precise_time_ns();
    let (x2,y2) = tokio::join!(x_fut,y_fut);
    let end_ns = time::precise_time_ns();

    let time = (end_ns - start_ns) as f64;
    println!("{:e} - {:0.2e} ms ({:0.2?}Hz)", n, time / 1_000_000.0 / n as f64, 1.0 / (time / 1_000_000_000.0));
    }
  }
  let end_ns0 = time::precise_time_ns();
  let time = (end_ns0 - start_ns0) as f64;
  println!("{:0.4?} s", time / 1e9);


  
  /*
  for n in &sizes {
    let mut v: Vec<u64> = vec![0;*n];
    let start_ns = time::precise_time_ns();
    let x: Vec<u64> = v.iter().map(|x| {
      short_running_computation_sync(x)
    }).collect();
    let end_ns = time::precise_time_ns();
    let time = (end_ns - start_ns) as f64;
    println!("{:e} BASE   {:0.2e} ms", n, time / 1_000_000.0);
  }
  for n in &sizes {
    let mut v: Vec<u64> = vec![0;*n];
    let start_ns = time::precise_time_ns();
    let x: Vec<u64> = v.par_iter().map(|x| {
      short_running_computation_sync(x)
    }).collect();
    let end_ns = time::precise_time_ns();
    let time = (end_ns - start_ns) as f64;
    println!("{:e} PARA   {:0.2e} ms", n, time / 1_000_000.0);
  }
  for n in &sizes {
    let mut futures = vec![];
    let mut v: Vec<u64> = vec![0;*n];
    //let mut stream = tokio_stream::iter(&v);
    //let mut i = 0;
    for i in 1..*n {
      let fut = tokio::task::spawn(short_running_computation(i as u64));
      futures.push(fut);
    }
    let start_ns = time::precise_time_ns();
    let results = join_all(futures).await;
    let end_ns = time::precise_time_ns();
    let time = (end_ns - start_ns) as f64;
    println!("{:e} ASYNC  {:0.2e} ms", n, time / 1_000_000.0);
  }*/
}

/*
#[tokio::main]
pub async fn main() -> Result<(),String> {

  const n: usize = 10;
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
    let mut stream = tokio_stream::iter(&v);
    let start_ns = time::precise_time_ns();
    let mut stream = tokio_stream::iter(&[1, 2, 3]);

    while let Some(v) = stream.next().await {
        println!("GOT = {:?}", v);
    }
    let end_ns = time::precise_time_ns();
    let time = (end_ns - start_ns) as f64;
    println!("ITER {:0.4?} ms", time / 1_000_000.0);
  }


  Ok(())
}*/