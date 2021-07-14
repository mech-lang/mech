
use std::sync::Arc;
use std::cell::RefCell;
use std::fmt;

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
use std::collections::VecDeque;

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

fn par_advance(x: &Vec<i64>, vx: &Vec<i64>) -> Vec<i64> {
  x.par_iter().zip(vx).map(|(x,y)| x + y).collect()
}

fn par_accelerate(vx: &Vec<i64>, gravity: &Vec<i64>) -> Vec<i64> {
  vx.par_iter().map(|x| x + gravity[0]).collect()
}

fn par_filter1(x: &Vec<i64>) -> Vec<i64> {
  x.par_iter().map(|x| (*x < 100) as i64).collect()
}

fn par_filter2(x: &Vec<i64>) -> Vec<i64> {
  x.par_iter().map(|x| (*x > 500) as i64).collect()
}

fn par_bounce1(x: &Vec<i64>, ix: &Vec<i64>) -> Vec<i64> {
  x.par_iter().zip(ix).map(|(x,y)| if *y == 1 {
    100
  } else {
    *x
  }).collect()
}

fn par_bounce2(x: &Vec<i64>, ix: &Vec<i64>) -> Vec<i64> {
  x.par_iter().zip(ix).map(|(x,y)| if *y == 1 {
    500
  } else {
    *x
  }).collect()
}

fn par_bounce3(vx: &Vec<i64>, ix: &Vec<i64>) -> Vec<i64> {
  vx.par_iter().zip(ix).map(|(x,y)| if *y == 1 {
    -*x
  } else {
    *x
  }).collect()
}

fn par_dampen(vx: &Vec<i64>, ix: &Vec<i64>) -> Vec<i64> {
  vx.par_iter().zip(ix).map(|(x,y)| if *y == 1 {
    *x * 90 / 100
  } else {
    *x
  }).collect()
}

async fn par_do_y(y: Vec<i64>, vy: Vec<i64>) -> (Vec<i64>,Vec<i64>) {
  let gravity = vec![1];
  let y2 = par_advance(&y,&vy);
  let vy2 = par_advance(&vy,&gravity);
  let iy1 = par_filter1(&y2);
  let iy2 = par_filter2(&y2);
  let y3= par_bounce1(&y2,&iy1);
  let y4 = par_bounce2(&y3,&iy2);
  let vy3 = par_bounce3(&vy2, &iy1);
  let vy4 = par_bounce3(&vy3, &iy2);
  let vy5 = par_dampen(&vy4, &iy2);
  (y4,vy5)
}

async fn par_do_x(x: Vec<i64>, vx: Vec<i64>) -> (Vec<i64>,Vec<i64>) {
  let x2 = par_advance(&x,&vx);
  let ix1 = par_filter1(&x2);
  let ix2 = par_filter2(&x2);
  let x3 = par_bounce1(&x2,&ix1);
  let x4 = par_bounce2(&x3,&ix2);
  let vx2 = par_bounce3(&vx, &ix1);
  let vx3 = par_bounce3(&vx2, &ix2);
  (x4,vx3)
}

use tokio_stream::StreamExt;

pub struct Table {
  pub rows: usize,
  pub cols: usize,
  data: Vec<u64>,
}

impl Table {
  pub fn new(rows: usize, cols: usize) -> Table {
    let mut table = Table {
      rows,
      cols,
      data: Vec::with_capacity(rows*cols*2),
    };
    table.data.resize(rows*cols,0);
    table
  }

  pub fn get_linear(&self, ix: usize) -> Option<u64> {
    if ix > self.data.len() {
      None
    } else {
      Some(self.data[ix])
    }
  }

  pub fn set_linear(&mut self, ix: usize, value: u64) -> Result<(),()> {
    if ix > self.data.len() {
      Err(())
    } else {
      self.data[ix] = value;
      Ok(())
    }
  }

  pub fn get(&self, row: usize, col: usize) -> Option<u64> {
    let ix = (col * self.rows) + row;
    if ix > self.data.len() {
      None
    } else {
      Some(self.data[ix])
    }
  }

  pub fn set(&mut self, row: usize, col: usize, value: u64) -> Result<(),()> {
    let ix = (col * self.rows) + row;
    if ix > self.data.len() {
      Err(())
    } else {
      self.data[ix] = value;
      Ok(())
    }
  }
}

impl fmt::Debug for Table {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    for row in 0..self.rows {
      for col in 0..self.cols {
        let v = self.get(row,col).unwrap();
        write!(f,"{:?} ", v)?;
      }
      write!(f,"\n")?;
    }
    Ok(())
  }
}

#[tokio::main]
async fn main() {
  let sizes: Vec<usize> = vec![1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7].iter().map(|x| *x as usize).collect();
  
  let start_ns0 = time::precise_time_ns();
  let n = 4 as usize;
  let x: Vec<i64> = vec![0;n];
  let y: Vec<i64> = vec![0;n];
  let vx: Vec<i64> = vec![1;n];
  let vy: Vec<i64> = vec![1;n];
  let mut balls = Table::new(n,4);
  for i in 0..n {
    balls.set(i,2,1);
    balls.set(i,3,1);
  }
  println!("{:?}", balls);
  let bounds: Vec<i64> = vec![500, 500];
  let mut total_time = VecDeque::new();
  loop {
    let start_ns = time::precise_time_ns();
    let ((x, vx),(y, vy)) = if n <= 10_000 {
      let x2 = do_x(x.clone(),vx.clone()).await;
      let y2 = do_y(y.clone(),vy.clone()).await;
      (x2, y2)
    } else {
      let x_fut = tokio::task::spawn(par_do_x(x.clone(),vx.clone()));
      let y_fut = tokio::task::spawn(par_do_y(y.clone(),vy.clone()));
      let (x2, y2) = tokio::join!(x_fut,y_fut);
      (x2.unwrap(), y2.unwrap())
    };
    let end_ns = time::precise_time_ns();
    let time = (end_ns - start_ns) as f64;
    total_time.push_back(time);
    if total_time.len() > 1000 {
      total_time.pop_front();
    }    
    let average_time: f64 = total_time.iter().sum::<f64>() / total_time.len() as f64;
    println!("{:e} - {:0.2e} ms ({:0.2?}Hz)", n, time / 1_000_000.0 / n as f64, 1.0 / (average_time / 1_000_000_000.0));
  }
  let end_ns0 = time::precise_time_ns();
  let time = (end_ns0 - start_ns0) as f64;
  println!("{:0.4?} s", time / 1e9);

}
