
use std::sync::Arc;
use std::cell::RefCell;
use std::fmt;
use std::ptr;
use std::rc::Rc;

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
use map_in_place::MapVecInPlace;

pub type MechFunction = extern "C" fn(arguments: &mut Vec<Vec<f64>>);
pub type Column = Rc<RefCell<Vec<f64>>>;

pub struct Table {
  pub rows: usize,
  pub cols: usize,
  data: Vec<Column>,
}

impl Table {
  pub fn new(rows: usize, cols: usize) -> Table {
    let mut table = Table {
      rows,
      cols,
      data: Vec::with_capacity(cols),
    };
    for col in 0..cols {
      table.data.push(Rc::new(RefCell::new(vec![0.0; rows])));
    }
    table
  }

  pub fn get(&self, row: usize, col: usize) -> Option<f64> {
    if col < self.cols && row < self.rows {
      Some(self.data[col].borrow()[row])
    } else {
      None
    }
  }

  pub fn set(&self, row: usize, col: usize, val: f64) -> Result<(),()> {
    if col < self.cols && row < self.rows {
      self.data[col].borrow_mut()[row] = val;
      Ok(())
    } else {
      Err(())
    }
  }

  pub fn get_column_unchecked(&self, col: usize) -> Column {
    self.data[col].clone()
  }

}

impl fmt::Debug for Table {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    for row in 0..self.rows {
      write!(f,"│ ")?;
      for col in 0..self.cols {
        let v = self.get(row,col).unwrap();
        write!(f,"{:0.2?} │ ", v)?;
      }
      write!(f,"\n")?;
    }
    Ok(())
  }
}

// binop vector-vector          -- lhs: &Vec<f64>,     rhs: &Vec<f64>    out: &mut Vec<f64>
// binop vector-vector in-place -- lhs: &mut Vec<f64>  rhs: &Vec<f64>
// binop vector-scalar          -- lhs: &Vec<f64>,     rhs: f64          out: &mut Vec<f64>
// binop vector-scalar in-place -- lhs: &mut Vec<f64>  rhs: f64
// truth vector-vector          -- lhs: &Vec<bool>     rhs: &Vec<bool>   out: &mut Vec<bool>
// comp  vector-scalar          -- lhs: &Vec<f64>      rhs: f64          out: &mut Vec<bool>
// set   vector-scalar          -- ix: &Vec<bool>      x:   f64          out: &mut Vec<f64>
// set   vector-vector          -- ix: &Vec<bool>      x:   &Vec<f64>    out: &mut Vec<f64>

pub type ArgF64 = Rc<RefCell<Vec<f64>>>;
pub type ArgBool = Rc<RefCell<Vec<bool>>>;
pub type OutF64 = Rc<RefCell<Vec<f64>>>;
pub type OutBool = Rc<RefCell<Vec<bool>>>;

#[derive(Debug)]
enum Transformation {
  ParAddVVIP((OutF64, ArgF64)),  
  ParAddVSIP((OutF64, f64)),
  ParMultiplyVS((ArgF64, f64, OutF64)),
  ParOrVV((ArgBool,ArgBool,OutBool)),
  ParLessThanVS((ArgF64,f64,OutBool)),
  ParGreaterThanVS((ArgF64,f64,OutBool)),
  ParSetVS((ArgBool,f64,OutF64)),
  ParSetVV((ArgBool,ArgF64,OutF64)),
}

impl Transformation {
  pub fn run(&mut self) {
    match &*self {
      // MATH
      Transformation::ParAddVVIP((lhs, rhs)) => { lhs.borrow_mut().par_iter_mut().zip(&(*rhs.borrow())).for_each(|(lhs, rhs)| *lhs += rhs); }
      Transformation::ParAddVSIP((lhs, rhs)) => { lhs.borrow_mut().par_iter_mut().for_each(|lhs| *lhs += rhs); }
      Transformation::ParMultiplyVS((lhs, rhs, out)) => { out.borrow_mut().par_iter_mut().zip(&(*lhs.borrow())).for_each(|(out, lhs)| *out = *lhs * rhs); }
      // COMPARE
      Transformation::ParGreaterThanVS((lhs, rhs, out)) => { out.borrow_mut().par_iter_mut().zip(&(*lhs.borrow())).for_each(|(out, lhs)| *out = *lhs > *rhs); }
      Transformation::ParLessThanVS((lhs, rhs, out)) => { out.borrow_mut().par_iter_mut().zip(&(*lhs.borrow())).for_each(|(out, lhs)| *out = *lhs < *rhs); }
      // LOGIC
      Transformation::ParOrVV((lhs, rhs, out)) => { out.borrow_mut().par_iter_mut().zip(&(*lhs.borrow())).zip(&(*rhs.borrow())).for_each(|((out, lhs),rhs)| *out = *lhs || *rhs); }
      // SET
      Transformation::ParSetVS((ix, rhs, out)) => {
        out.borrow_mut().par_iter_mut().zip(&(*ix.borrow())).for_each(|(out,ix)| {
          if *ix == true {
            *out = *rhs
          }});          
      }
      Transformation::ParSetVV((ix, rhs, out)) => {
        out.borrow_mut().par_iter_mut().zip(&(*ix.borrow())).zip(&(*rhs.borrow())).for_each(|((out,ix),x)| if *ix == true {
          *out = *x
        });          
      }
    }
  }
}

fn main() {
  let sizes: Vec<usize> = vec![1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7].iter().map(|x| *x as usize).collect();
  
  let start_ns0 = time::precise_time_ns();
  let n = 1e1 as usize;
  let mut balls = Table::new(n,4);
  for i in 0..n {
    balls.set(i,0,i as f64);
    balls.set(i,1,i as f64);
    balls.set(i,2,3.0);
    balls.set(i,3,4.0);
  }
  let mut total_time = VecDeque::new();

  // Table
  let mut x = balls.get_column_unchecked(0);
  let mut y = balls.get_column_unchecked(1);
  let mut vx = balls.get_column_unchecked(2);
  let mut vy = balls.get_column_unchecked(3);

  // Temp Vars
  let mut x2 = Rc::new(RefCell::new(vec![0.0; n]));
  let mut y2 = Rc::new(RefCell::new(vec![0.0; n]));
  let mut vy2 = Rc::new(RefCell::new(vec![0.0; n]));
  let mut iy = Rc::new(RefCell::new(vec![false; n]));
  let mut iyy = Rc::new(RefCell::new(vec![false; n]));
  let mut iy_or = Rc::new(RefCell::new(vec![false; n]));
  let mut ix = Rc::new(RefCell::new(vec![false; n]));
  let mut ixx = Rc::new(RefCell::new(vec![false; n]));
  let mut ix_or = Rc::new(RefCell::new(vec![false; n]));
  let mut vx2 = Rc::new(RefCell::new(vec![0.0; n]));

  // Update the block positions on each tick of the timer  
  let mut block1 = vec![
    // #ball.x := #ball.x + #ball.vx
    Rc::new(RefCell::new(Transformation::ParAddVVIP((x.clone(), vx.clone())))),
    // #ball.y := #ball.y + #ball.vy    
    Rc::new(RefCell::new(Transformation::ParAddVVIP((y.clone(), vy.clone())))),
    // #ball.vy := #ball.vy + #gravity
    Rc::new(RefCell::new(Transformation::ParAddVSIP((vy.clone(), 1.0)))),
  ];
  
  // Keep the balls within the boundary height
  let mut block2 = vec![
    // iy = #ball.y > #boundary.height
    Rc::new(RefCell::new(Transformation::ParGreaterThanVS((y.clone(), 500.0, iy.clone())))),
    // iyy = #ball.y < 0
    Rc::new(RefCell::new(Transformation::ParLessThanVS((y.clone(), 0.0, iyy.clone())))),
    // #ball.y{iy} := #boundary.height
    Rc::new(RefCell::new(Transformation::ParSetVS((iy.clone(), 500.0, y.clone())))),
    // #ball.vy{iy | iyy} := #ball.vy * -0.80
    Rc::new(RefCell::new(Transformation::ParOrVV((iy.clone(), iyy.clone(), iy_or.clone())))),
    Rc::new(RefCell::new(Transformation::ParMultiplyVS((vy.clone(), -0.8, vy2.clone())))),
    Rc::new(RefCell::new(Transformation::ParSetVV((iy_or.clone(), vy2.clone(), vy.clone())))),
  ];

  // Keep the balls within the boundary width
  let mut block3 = vec![
    // ix = #ball.x > #boundary.width
    Rc::new(RefCell::new(Transformation::ParGreaterThanVS((x.clone(), 500.0, ix.clone())))),
    // ixx = #ball.x < 0
    Rc::new(RefCell::new(Transformation::ParLessThanVS((x.clone(), 0.0, ixx.clone())))),
    // #ball.x{ix} := #boundary.width
    Rc::new(RefCell::new(Transformation::ParSetVS((ix.clone(), 500.0, x.clone())))),
    // #ball.vx{ix | ixx} := #ball.vx * -0.80
    Rc::new(RefCell::new(Transformation::ParOrVV((ix.clone(), ixx.clone(), ix_or.clone())))),
    Rc::new(RefCell::new(Transformation::ParMultiplyVS((vx.clone(), -0.8, vx2.clone())))),
    Rc::new(RefCell::new(Transformation::ParSetVV((ix_or.clone(), vx2.clone(), vx.clone())))),
  ];

  let mut blocks = vec![
    Rc::new(RefCell::new(block1)), 
    Rc::new(RefCell::new(block2)), 
    Rc::new(RefCell::new(block3))
  ];

  for _ in 0..2000 {
    let start_ns = time::precise_time_ns();

    for ref mut block in &mut blocks.iter() {
      for ref mut tfm in &mut block.borrow_mut().iter() {
        tfm.borrow_mut().run();
      }
    }
    let end_ns = time::precise_time_ns();
    let time = (end_ns - start_ns) as f64;
    total_time.push_back(time);
    if total_time.len() > 1000 {
      total_time.pop_front();
    }
    
  }
  println!("{:?}", balls);
  let average_time: f64 = total_time.iter().sum::<f64>() / total_time.len() as f64; 
  println!("{:e} - {:0.2?}Hz", n, 1.0 / (average_time / 1_000_000_000.0));
  let end_ns0 = time::precise_time_ns();
  let time = (end_ns0 - start_ns0) as f64;
  println!("{:0.4?} s", time / 1e9);
}