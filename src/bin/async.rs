
use std::sync::Arc;
use std::cell::RefCell;
use std::fmt;
use std::ptr;

use std::collections::VecDeque;
use rayon::prelude::*;
use std::thread;
use tokio::time::{sleep,Duration};
use tokio_stream;
use futures::future::join_all;

fn par_advance(x: &Vec<i64>, vx: &Vec<i64>) -> Vec<i64> {
  x.par_iter().zip(vx).map(|(x,y)| x + y).collect()
}

fn par_accelerate(vx: &Vec<i64>, gravity: &Vec<i64>) -> Vec<i64> {
  vx.par_iter().map(|x| x + gravity[0]).collect()
}

fn par_filter1(x: &Vec<i64>) -> Vec<i64> {
  x.par_iter().map(|x| (*x < 0) as i64).collect()
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
  let vy2 = par_accelerate(&vy,&gravity);
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

  pub fn set_col(&mut self, col: usize, data: &Vec<i64>) -> Result<(),()> {
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

  pub fn set_col_unchecked(&mut self, col: usize, data: &Vec<i64>) {
    let src_len = data.len();
    let dst_len = self.data.len();
    unsafe {
      let dst_ptr = self.data.as_mut_ptr().offset((col * self.rows) as isize);
      let src_ptr = data.as_ptr();
      ptr::copy_nonoverlapping(src_ptr, dst_ptr, src_len);
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
  let n = 1e1 as usize;
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
  for _ in 0..4000 as usize {
    let start_ns = time::precise_time_ns();
    let x_fut = tokio::task::spawn(par_do_x(balls.get_col_unchecked(0),balls.get_col_unchecked(2)));
    let y_fut = tokio::task::spawn(par_do_y(balls.get_col_unchecked(1),balls.get_col_unchecked(3)));
    let (x2, y2) = tokio::join!(x_fut,y_fut);
    let ((x2, vx2),(y2, vy2)) = (x2.unwrap(), y2.unwrap());
    balls.set_col_unchecked(0,&x2);
    balls.set_col_unchecked(1,&y2);
    balls.set_col_unchecked(2,&vx2);
    balls.set_col_unchecked(3,&vy2);
    let end_ns = time::precise_time_ns();
    let time = (end_ns - start_ns) as f64;
    total_time.push_back(time);
    if total_time.len() > 1000 {
      total_time.pop_front();
    }   
  }
  let average_time: f64 = total_time.iter().sum::<f64>() / total_time.len() as f64; 
  println!("{:e} - {:0.2?}Hz", n, 1.0 / (average_time / 1_000_000_000.0));
  let end_ns0 = time::precise_time_ns();
  let time = (end_ns0 - start_ns0) as f64;
  println!("{:0.4?} s", time / 1e9);
}
