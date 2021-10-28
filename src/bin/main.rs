
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
use mech_core::{Table, Function, Change, Column, Value, ValueKind, hash_string, Transformation, Block, Core, ParMultiplyVS};

fn main() {
  let sizes: Vec<usize> = vec![1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7].iter().map(|x| *x as usize).collect();
  let mut total_time = VecDeque::new();  
  let start_ns0 = time::precise_time_ns();
  let n = 1e6 as usize;

  // Create a core
  let mut core = Core::new();

  {
    // #time/timer += [period: 60Hz]
    let mut time_timer = Table::new(hash_string("time/timer"),1,2);
    time_timer.set_col_kind(0,ValueKind::F32);
    time_timer.set(0,0,Value::F32(60.0));
    core.insert_table(time_timer.clone());

    // #gravity = 1
    let mut gravity = Table::new(hash_string("gravity"),1,1);
    gravity.set_col_kind(0,ValueKind::F32);
    gravity.set(0,0,Value::F32(1.0));
    core.insert_table(gravity.clone());

    // -0.8
    let mut const1 = Table::new(hash_string("-0.8"),1,1);
    const1.set_col_kind(0,ValueKind::F32);
    const1.set(0,0,Value::F32(-0.8));
    core.insert_table(const1.clone());

    // Create balls
    // #balls = [x: 0:n y: 0:n vx: 3.0 vy: 4.0]
    let mut balls = Table::new(hash_string("balls"),n,4);
    balls.set_col_kind(0,ValueKind::F32);
    balls.set_col_kind(1,ValueKind::F32);
    balls.set_col_kind(2,ValueKind::F32);
    balls.set_col_kind(3,ValueKind::F32);
    for i in 0..n {
      balls.set(i,0,Value::F32(i as f32));
      balls.set(i,1,Value::F32(i as f32));
      balls.set(i,2,Value::F32(3.0));
      balls.set(i,3,Value::F32(4.0));
    }
    core.insert_table(balls.clone());
  }

  // Table
  let (x,y,vx,vy) = {
    match core.get_table_by_id(hash_string("balls")) {
      Some(balls_rc) => {
        let balls = balls_rc.borrow();
        (balls.get_column_unchecked(0),
        balls.get_column_unchecked(1),
        balls.get_column_unchecked(2),
        balls.get_column_unchecked(3))
      }
      _ => std::process::exit(1),
    }
  };

  let g = {
    match core.get_table_by_id(hash_string("gravity")) {
      Some(gravity_rc) => {
        let gravity = gravity_rc.borrow();
        gravity.get_column_unchecked(0)
      }
      _ => std::process::exit(1),
    }
  };

  let c1 = {
    match core.get_table_by_id(hash_string("-0.8")) {
      Some(const1_rc) => {
        let const1 = const1_rc.borrow();
        const1.get_column_unchecked(0)
      }
      _ => std::process::exit(1),
    }
  };
  
  // Temp Vars
  let mut vy2 = Column::F32(Rc::new(RefCell::new(vec![0.0; n])));
  let mut iy = Column::Bool(Rc::new(RefCell::new(vec![false; n])));
  let mut iyy = Column::Bool(Rc::new(RefCell::new(vec![false; n])));
  let mut iy_or = Column::Bool(Rc::new(RefCell::new(vec![false; n])));
  let mut ix = Column::Bool(Rc::new(RefCell::new(vec![false; n])));
  let mut ixx = Column::Bool(Rc::new(RefCell::new(vec![false; n])));
  let mut ix_or = Column::Bool(Rc::new(RefCell::new(vec![false; n])));
  let mut vx2 = Column::F32(Rc::new(RefCell::new(vec![0.0; n])));


  // Update the block positions on each tick of the timer  
  let mut block1 = Block::new();
  match (&x,&vx,&y,&vy,&g) {
    (Column::F32(x),Column::F32(vx),Column::F32(y),Column::F32(vy),Column::F32(g)) => {
      // #ball.x := #ball.x + #ball.vx
      block1.plan.push(Function::ParAddVVIPF32(vec![x.clone(), vx.clone()]));
      // #ball.y := #ball.y + #ball.vy    
      block1.plan.push(Function::ParAddVVIPF32(vec![y.clone(), vy.clone()]));
      // #ball.vy := #ball.vy + #gravity
      block1.plan.push(Function::ParAddVSIPF32(vec![vy.clone(), g.clone()]));
    }
    _ => (),
  }
  block1.gen_id();


  // Keep the balls within the boundary height
  let mut block2 = Block::new();
  match (&y,&iy,&iyy,&iy_or,&c1,&vy2,&vy) {
    (Column::F32(y),Column::Bool(iy),Column::Bool(iyy),Column::Bool(iy_or),Column::F32(c1),Column::F32(vy2),Column::F32(vy)) => {
      // iy = #ball.y > #boundary.height
      block2.plan.push(Function::ParGreaterThanVS((y.clone(), 500.0, iy.clone())));
      // iyy = #ball.y < 0
      block2.plan.push(Function::ParLessThanVS((y.clone(), 0.0, iyy.clone())));
      // #ball.y{iy} := #boundary.height
      block2.plan.push(Function::ParSetVS((iy.clone(), 500.0, y.clone())));
      // #ball.vy{iy | iyy} := #ball.vy * -0.80
      block2.plan.push(Function::ParOrVV(vec![iy.clone(), iyy.clone(), iy_or.clone()]));
      block2.plan.push(ParMultiplyVS::<f32>{lhs: vy.clone(), rhs: c1.clone(), out: vy2.clone()});
      block2.plan.push(Function::ParSetVV((iy_or.clone(), vy2.clone(), vy.clone())));
    }
    _ => (),
  }
  block2.gen_id();

  // Keep the balls within the boundary width
  let mut block3 = Block::new();
  match (&x,&ix,&ixx,&ix_or,&vx,&c1,&vx2) {
    (Column::F32(x),Column::Bool(ix),Column::Bool(ixx),Column::Bool(ix_or),Column::F32(vx),Column::F32(c1),Column::F32(vx2)) => {
      // ix = #ball.x > #boundary.width
      block3.plan.push(Function::ParGreaterThanVS((x.clone(), 500.0, ix.clone())));
      // ixx = #ball.x < 0
      block3.plan.push(Function::ParLessThanVS((x.clone(), 0.0, ixx.clone())));
      // #ball.x{ix} := #boundary.width
      block3.plan.push(Function::ParSetVS((ix.clone(), 500.0, x.clone())));
      // #ball.vx{ix | ixx} := #ball.vx * -0.80
      block3.plan.push(Function::ParOrVV(vec![ix.clone(), ixx.clone(), ix_or.clone()]));
      block3.plan.push(ParMultiplyVS{lhs: vx.clone(), rhs: c1.clone(), out: vx2.clone()});
      block3.plan.push(Function::ParSetVV((ix_or.clone(), vx2.clone(), vx.clone())));
    }
    _ => (),
  }
  block3.gen_id();

  
  core.schedules.insert((hash_string("time/timer"), 0, 1),vec![vec![0],vec![1, 2]]);

  core.insert_block(block1);
  core.insert_block(block2);
  core.insert_block(block3);

  for i in 0..200000 {
    let txn = vec![Change::Set((hash_string("time/timer"), vec![(0, 1, Value::F32(i as f32))]))];
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