
use std::sync::Arc;
use std::cell::RefCell;
use std::fmt;
use std::ptr;

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
use futures::stream::futures_unordered::FuturesUnordered;
use tokio_stream::StreamExt;

fn add_vectors(x: &Vec<i64>, y: &Vec<i64>) -> Vec<i64> {
  x.iter().zip(y).map(|(x,y)| x + y).collect()
}

fn add_scalar(x: &Vec<i64>, y: i64) -> Vec<i64> {
  x.iter().map(|x| x + y).collect()
}

fn less_than_scalar(x: &Vec<i64>, y: i64) -> Vec<bool> {
  x.iter().map(|x| (*x < y)).collect()
}

fn greater_than_scalar(x: &Vec<i64>, y: i64) -> Vec<bool> {
  x.iter().map(|x| (*x > y)).collect()
}

fn set_scalar(x: &Vec<i64>, ix: &Vec<bool>, v: i64) -> Vec<i64> {
  x.iter().zip(ix).map(|(x,y)| if *y == true {
    v
  } else {
    *x
  }).collect()
}

fn bounce3(vx: &Vec<i64>, ix: &Vec<bool>) -> Vec<i64> {
  vx.iter().zip(ix).map(|(x,y)| if *y == true {
    -*x
  } else {
    *x
  }).collect()
}

fn dampen(vx: &Vec<i64>, ix: &Vec<bool>) -> Vec<i64> {
  vx.iter().zip(ix).map(|(x,y)| if *y == true {
    *x * 90 / 100
  } else {
    *x
  }).collect()
}

async fn do_y(y: Vec<i64>, vy: Vec<i64>) -> (Vec<i64>,Vec<i64>) {
  let y2 = add_vectors(&y,&vy);
  let vy2 = add_scalar(&vy,1);
  let iy1 = less_than_scalar(&y2,0);
  let iy2 = greater_than_scalar(&y2,500);
  let y3= set_scalar(&y2,&iy1,0);
  let y4 = set_scalar(&y3,&iy2,500);
  let vy3 = bounce3(&vy2, &iy1);
  let vy4 = bounce3(&vy3, &iy2);
  let vy5 = dampen(&vy4, &iy2);
  (y4,vy5)
}

async fn do_x(x: Vec<i64>, vx: Vec<i64>) -> (Vec<i64>,Vec<i64>) {
  let x2 = add_vectors(&x,&vx);
  let ix1 = less_than_scalar(&x2,0);
  let ix2 = greater_than_scalar(&x2,500);
  let x3 = set_scalar(&x2,&ix1,0);
  let x4 = set_scalar(&x3,&ix2,500);
  let vx2 = bounce3(&vx, &ix1);
  let vx3 = bounce3(&vx2, &ix2);
  (x4,vx3)
}

fn par_add_vectors(x: &Vec<i64>, y: &Vec<i64>) -> Vec<i64> {
  x.par_iter().zip(y).map(|(x,y)| x + y).collect()
}

fn par_add_scalar(x: &Vec<i64>, y: i64) -> Vec<i64> {
  x.par_iter().map(|x| x + y).collect()
}

fn par_less_than_scalar(x: &Vec<i64>, y: i64) -> Vec<bool> {
  x.par_iter().map(|x| (*x < y)).collect()
}

fn par_greater_than_scalar(x: &Vec<i64>, y: i64) -> Vec<bool> {
  x.par_iter().map(|x| (*x > y)).collect()
}

fn par_set_scalar(x: &Vec<i64>, ix: &Vec<bool>, v: i64) -> Vec<i64> {
  x.par_iter().zip(ix).map(|(x,y)| if *y == true {
    v
  } else {
    *x
  }).collect()
}

fn par_bounce3(vx: &Vec<i64>, ix: &Vec<bool>) -> Vec<i64> {
  vx.par_iter().zip(ix).map(|(x,y)| if *y == true {
    -*x
  } else {
    *x
  }).collect()
}

fn par_dampen(vx: &Vec<i64>, ix: &Vec<bool>) -> Vec<i64> {
  vx.par_iter().zip(ix).map(|(x,y)| if *y == true {
    *x * 90 / 100
  } else {
    *x
  }).collect()
}

async fn par_do_y(y: Vec<i64>, vy: Vec<i64>) -> (Vec<i64>,Vec<i64>) {
  let y2 = par_add_vectors(&y,&vy);
  let vy2 = par_add_scalar(&vy,1);
  let iy1 = par_less_than_scalar(&y2,0);
  let iy2 = par_greater_than_scalar(&y2,500);
  let y3= par_set_scalar(&y2,&iy1,0);
  let y4 = par_set_scalar(&y3,&iy2,500);
  let vy3 = par_bounce3(&vy2, &iy1);
  let vy4 = par_bounce3(&vy3, &iy2);
  let vy5 = par_dampen(&vy4, &iy2);
  (y4,vy5)
}

async fn par_do_x(x: Vec<i64>, vx: Vec<i64>) -> (Vec<i64>,Vec<i64>) {
  let x2 = par_add_vectors(&x,&vx);
  let ix1 = par_less_than_scalar(&x2,0);
  let ix2 = par_greater_than_scalar(&x2,500);
  let x3 = par_set_scalar(&x2,&ix1,0);
  let x4 = par_set_scalar(&x3,&ix2,500);
  let vx2 = par_bounce3(&vx, &ix1);
  let vx3 = par_bounce3(&vx2, &ix2);
  (x4,vx3)
}


pub type MechFunction = extern "C" fn(arguments: &mut Vec<Vec<i64>>);

pub struct Table {
  pub rows: usize,
  pub cols: usize,
  data: Vec<i64>,
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

  pub fn get_linear(&self, ix: usize) -> Option<i64> {
    if ix > self.data.len() {
      None
    } else {
      Some(self.data[ix])
    }
  }

  pub fn set_linear(&mut self, ix: usize, value: i64) -> Result<(),()> {
    if ix > self.data.len() {
      Err(())
    } else {
      self.data[ix] = value;
      Ok(())
    }
  }

  pub fn get(&self, row: usize, col: usize) -> Option<i64> {
    let ix = (col * self.rows) + row;
    if ix > self.data.len() {
      None
    } else {
      Some(self.data[ix])
    }
  }

  pub fn set(&mut self, row: usize, col: usize, value: i64) -> Result<(),()> {
    let ix = (col * self.rows) + row;
    if ix > self.data.len() {
      Err(())
    } else {
      self.data[ix] = value;
      Ok(())
    }
  }

  pub fn get_col(&mut self, col: usize) -> Option<Vec<i64>> {
    if col > self.cols {
      None
    } else {
      Some(self.data[self.rows*col..self.rows*col+self.rows].into())
    }
  }

  pub fn get_col_unchecked(&mut self, col: usize) -> Vec<i64> {
    self.data[self.rows*col..self.rows*col+self.rows].into()
  }

  pub async fn set_col(&mut self, col: usize, data: &Vec<i64>) -> Result<(),()> {
    if col > self.cols || data.len() != self.rows {
      Err(())
    } else {
      let src_len = data.len();
      let dst_len = self.data.len();
      unsafe {
        let dst_ptr = self.data.as_mut_ptr().offset((col * self.rows) as isize);
        let src_ptr = data.as_ptr();
        ptr::copy_nonoverlapping(src_ptr, dst_ptr, src_len);
      }
      Ok(())
    }
  }

  pub async fn set_col_unchecked(&mut self, col: usize, data: &Vec<i64>) {
    let src_len = data.len();
    let dst_len = self.data.len();
    unsafe {
      let dst_ptr = self.data.as_mut_ptr().offset((col * self.rows) as isize);
      let src_ptr = data.as_ptr();
      ptr::copy_nonoverlapping(src_ptr, dst_ptr, src_len);
    }
  }

  pub fn column_iterator(&mut self) -> rayon::slice::ChunksExactMut<'_, i64> {
    self.data.par_chunks_exact_mut(self.rows)
  }

}

pub async fn replace(data: Vec<i64>, dest: &mut [i64]) {
  let src_len = data.len();
  unsafe {
    let dst_ptr = dest.as_mut_ptr();
    let src_ptr = data.as_ptr();
    ptr::copy_nonoverlapping(src_ptr, dst_ptr, src_len);
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
  let n = 1e6 as usize;
  let mut balls = Table::new(n,4);
  for i in 0..n {
    balls.set(i,0,i as i64);
    balls.set(i,1,i as i64);
    balls.set(i,2,3);
    balls.set(i,3,4);
  }
  let mut col = balls.get_col(3).unwrap();
  let bounds: Vec<i64> = vec![500, 500];
  let mut total_time = VecDeque::new();

  loop {
    let start_ns = time::precise_time_ns();
    /*if n <= 10_000 {
      let (x2, vx2) = do_x(balls.get_col_unchecked(0),balls.get_col_unchecked(2)).await;
      let (y2, vy2) = do_y(balls.get_col_unchecked(1),balls.get_col_unchecked(3)).await;
      balls.set_col_unchecked(0,&x2);
      balls.set_col_unchecked(1,&y2);
      balls.set_col_unchecked(2,&vx2);
      balls.set_col_unchecked(3,&vy2);
    } else {*/
      let x_fut = tokio::task::spawn(par_do_x(balls.get_col_unchecked(0),balls.get_col_unchecked(2)));
      let y_fut = tokio::task::spawn(par_do_y(balls.get_col_unchecked(1),balls.get_col_unchecked(3)));
      let (x2,y2) = tokio::join!(x_fut,y_fut);
      let ((x2,vx2),(y2,vy2)) = (x2.unwrap(),y2.unwrap());
      balls.column_iterator().zip(vec![x2,vx2,y2,vy2]).for_each(|(col,x)| {
        replace(x,col);
      });
    //}
    let end_ns = time::precise_time_ns();
    let time = (end_ns - start_ns) as f64;
    total_time.push_back(time);
    if total_time.len() > 1000 {
      total_time.pop_front();
    }
    let average_time: f64 = total_time.iter().sum::<f64>() / total_time.len() as f64; 
    println!("{:e} - {:0.2?}Hz", n, 1.0 / (average_time / 1_000_000_000.0));
  }
  let end_ns0 = time::precise_time_ns();
  let time = (end_ns0 - start_ns0) as f64;
  println!("{:0.4?} s", time / 1e9);
}
