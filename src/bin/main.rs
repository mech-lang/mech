
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

use std::sync::Arc;
use std::cell::RefCell;
use std::fmt;
use std::ptr;
use std::rc::Rc;
use hashbrown::{HashMap, HashSet};
use seahash;

use rayon::prelude::*;
use std::collections::VecDeque;
use std::thread;
use mech_core::{Table, hash_string, Transformation, Block, Core};

fn main() {
  let sizes: Vec<usize> = vec![1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7].iter().map(|x| *x as usize).collect();
  let mut total_time = VecDeque::new();  
  let start_ns0 = time::precise_time_ns();
  let n = 1e6 as usize;

  // Create a core
  let mut core = Core::new();

  {
    // #time/timer += [period: 60Hz]
    let time_timer = Table::new(hash_string("time/timer"),1,2);
    time_timer.set(0,0,60.0);
    core.insert_table(time_timer.clone());

    // #gravity = 1
    let gravity = Table::new(hash_string("gravity"),1,1);
    gravity.set(0,0,1.0);
    core.insert_table(gravity.clone());

    // #gravity = 1
    let cosnt1 = Table::new(hash_string("-0.8"),1,1);
    cosnt1.set(0,0,-0.8);
    core.insert_table(cosnt1.clone());

    // Create balls
    // #balls = [x: 0:n y: 0:n vx: 3.0 vy: 4.0]
    let balls = Table::new(hash_string("balls"),n,4);
    for i in 0..n {
      balls.set(i,0,i as f32);
      balls.set(i,1,i as f32);
      balls.set(i,2,3.0);
      balls.set(i,3,4.0);
    }
    core.insert_table(balls.clone());
  }

  // Table
  let balls = core.get_table("balls").unwrap();
  let mut x = balls.get_column_unchecked(0);
  let mut y = balls.get_column_unchecked(1);
  let mut vx = balls.get_column_unchecked(2);
  let mut vy = balls.get_column_unchecked(3);

  let gravity = core.get_table("gravity").unwrap();
  let mut g = gravity.get_column_unchecked(0);

  let const1 = core.get_table("-0.8").unwrap();
  let mut c1 = const1.get_column_unchecked(0);

  // Temp Vars
  let mut vy2 = Rc::new(RefCell::new(vec![0.0; n]));
  let mut iy = Rc::new(RefCell::new(vec![false; n]));
  let mut iyy = Rc::new(RefCell::new(vec![false; n]));
  let mut iy_or = Rc::new(RefCell::new(vec![false; n]));
  let mut ix = Rc::new(RefCell::new(vec![false; n]));
  let mut ixx = Rc::new(RefCell::new(vec![false; n]));
  let mut ix_or = Rc::new(RefCell::new(vec![false; n]));
  let mut vx2 = Rc::new(RefCell::new(vec![0.0; n]));

  // Update the block positions on each tick of the timer  
  let mut block1 = Block::new();
  // #ball.x := #ball.x + #ball.vx
  block1.add_tfm(Transformation::ParAddVVIP((x.clone(), vx.clone())));
  // #ball.y := #ball.y + #ball.vy    
  block1.add_tfm(Transformation::ParAddVVIP((y.clone(), vy.clone())));
  // #ball.vy := #ball.vy + #gravity
  block1.add_tfm(Transformation::ParAddVSIP((vy.clone(), g.clone())));
  block1.gen_id();

  // Keep the balls within the boundary height
  let mut block2 = Block::new();
  // iy = #ball.y > #boundary.height
  block2.add_tfm(Transformation::ParGreaterThanVS((y.clone(), 500.0, iy.clone())));
  // iyy = #ball.y < 0
  block2.add_tfm(Transformation::ParLessThanVS((y.clone(), 0.0, iyy.clone())));
  // #ball.y{iy} := #boundary.height
  block2.add_tfm(Transformation::ParSetVS((iy.clone(), 500.0, y.clone())));
  // #ball.vy{iy | iyy} := #ball.vy * -0.80
  block2.add_tfm(Transformation::ParOrVV((iy.clone(), iyy.clone(), iy_or.clone())));
  block2.add_tfm(Transformation::ParMultiplyVS((vy.clone(), c1.clone(), vy2.clone())));
  block2.add_tfm(Transformation::ParSetVV((iy_or.clone(), vy2.clone(), vy.clone())));
  block2.gen_id();

  // Keep the balls within the boundary width
  let mut block3 = Block::new();
  // ix = #ball.x > #boundary.width
  block3.add_tfm(Transformation::ParGreaterThanVS((x.clone(), 500.0, ix.clone())));
  // ixx = #ball.x < 0
  block3.add_tfm(Transformation::ParLessThanVS((x.clone(), 0.0, ixx.clone())));
  // #ball.x{ix} := #boundary.width
  block3.add_tfm(Transformation::ParSetVS((ix.clone(), 500.0, x.clone())));
  // #ball.vx{ix | ixx} := #ball.vx * -0.80
  block3.add_tfm(Transformation::ParOrVV((ix.clone(), ixx.clone(), ix_or.clone())));
  block3.add_tfm(Transformation::ParMultiplyVS((vx.clone(), c1.clone(), vx2.clone())));
  block3.add_tfm(Transformation::ParSetVV((ix_or.clone(), vx2.clone(), vx.clone())));
  block3.gen_id();

  core.schedules.insert((hash_string("time/timer"), 0, 1),vec![vec![0],vec![1, 2]]);

  core.insert_block(block1);
  core.insert_block(block2);
  core.insert_block(block3);

  for i in 0..200000 {
    let txn = vec![(hash_string("time/timer"), vec![(0, 1, i as f32)])];
    let start_ns = time::precise_time_ns();

    core.process_transaction(&txn);

    let end_ns = time::precise_time_ns();
    let time = (end_ns - start_ns) as f32;
    total_time.push_back(time);
    if total_time.len() > 1000 {
      total_time.pop_front();
    }
    let average_time: f32 = total_time.iter().sum::<f32>() / total_time.len() as f32; 
    println!("{:e} - {:0.2?}Hz", n, 1.0 / (average_time / 1_000_000_000.0));
  }

  let end_ns0 = time::precise_time_ns();
  let time = (end_ns0 - start_ns0) as f32;
  println!("{:0.4?} s", time / 1e9);
}