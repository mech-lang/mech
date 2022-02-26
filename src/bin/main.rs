#![allow(warnings)]
#![feature(iter_intersperse)]
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

use rayon::prelude::*;
use std::collections::VecDeque;
use std::thread;
use mech_core::*;
use mech_core::function::table;

use std::fmt::*;
use num_traits::*;
use std::ops::*;

fn main() -> std::result::Result<(),MechError> {
 
  let sizes: Vec<usize> = vec![1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7].iter().map(|x| *x as usize).collect();

  let start_ns0 = time::precise_time_ns();
  let n = 1e6 as usize;

  // Create a core
  let mut core = Core::new();

  {
    // #time/timer += [period: 60Hz]
    let mut time_timer = Table::new(hash_str("time/timer"),1,2);
    time_timer.set_col_kind(0,ValueKind::f32);
    time_timer.set_col_kind(1,ValueKind::f32);
    time_timer.set_raw(0,0,Value::f32(60.0));
    core.insert_table(time_timer);

    // #gravity = 1
    let mut gravity = Table::new(hash_str("gravity"),1,1);
    gravity.set_col_kind(0,ValueKind::f32);
    gravity.set_raw(0,0,Value::f32(1.0));
    core.insert_table(gravity);

    // -80%
    let mut const1 = Table::new(hash_str("-0.8"),1,1);
    const1.set_col_kind(0,ValueKind::f32);
    const1.set_raw(0,0,Value::f32(-0.8));
    core.insert_table(const1);

    // 500
    let mut const2 = Table::new(hash_str("500.0"),1,1);
    const2.set_col_kind(0,ValueKind::f32);
    const2.set_raw(0,0,Value::f32(500.0));
    core.insert_table(const2);

    // 0
    let mut const3 = Table::new(hash_str("0.0"),1,1);
    const3.set_col_kind(0,ValueKind::f32);
    const3.set_raw(0,0,Value::f32(0.0));
    core.insert_table(const3);

    // Create balls
    // #balls = [x: 0:n y: 0:n vx: 3.0 vy: 4.0]
    let mut balls = Table::new(hash_str("balls"),n,4);
    balls.set_col_kind(0,ValueKind::f32);
    balls.set_col_kind(1,ValueKind::f32);
    balls.set_col_kind(2,ValueKind::f32);
    balls.set_col_kind(3,ValueKind::f32);
    for i in 0..n {
      balls.set_raw(i,0,Value::f32(i as f32));
      balls.set_raw(i,1,Value::f32(i as f32));
      balls.set_raw(i,2,Value::f32(3.0));
      balls.set_raw(i,3,Value::f32(4.0));
    }
    core.insert_table(balls);
  }

  // Table
  let (x,y,vx,vy) = {
    match core.get_table_by_id(hash_str("balls")) {
      Ok(balls_rc) => {
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
    match core.get_table_by_id(hash_str("gravity")) {
      Ok(gravity_rc) => {
        let gravity = gravity_rc.borrow();
        gravity.get_column_unchecked(0)
      }
      _ => std::process::exit(1),
    }
  };

  let c1 = {
    match core.get_table_by_id(hash_str("-0.8")) {
      Ok(const1_rc) => {
        let const1 = const1_rc.borrow();
        const1.get_column_unchecked(0)
      }
      _ => std::process::exit(1),
    }
  };

  let c500 = {
    match core.get_table_by_id(hash_str("500.0")) {
      Ok(const1_rc) => {
        let const1 = const1_rc.borrow();
        const1.get_column_unchecked(0)
      }
      _ => std::process::exit(1),
    }
  };

  let c0 = {
    match core.get_table_by_id(hash_str("0.0")) {
      Ok(const1_rc) => {
        let const1 = const1_rc.borrow();
        const1.get_column_unchecked(0)
      }
      _ => std::process::exit(1),
    }
  };
  
  // Temp Vars
  let mut vy2 = Column::f32(ColumnV::new(vec![0.0; n]));
  let mut iy = Column::Bool(ColumnV::new((vec![false; n])));
  let mut iyy = Column::Bool(ColumnV::new((vec![false; n])));
  let mut iy_or = Column::Bool(ColumnV::new((vec![false; n])));
  let mut ix = Column::Bool(ColumnV::new((vec![false; n])));
  let mut ixx = Column::Bool(ColumnV::new((vec![false; n])));
  let mut ix_or = Column::Bool(ColumnV::new((vec![false; n])));
  let mut vx2 = Column::f32(ColumnV::new((vec![0.0; n])));

  // Update the block positions on each tick of the timer  
  let mut block1 = Block::new();
  block1.add_tfm(Transformation::NewTable{table_id: TableId::Local(hash_str("block1")), rows: 1, columns: 1});
  block1.triggers.insert((TableId::Global(hash_str("time/timer")),TableIndex::All,TableIndex::All));
  block1.input.insert((TableId::Global(hash_str("gravity")),TableIndex::All,TableIndex::All));
  block1.input.insert((TableId::Global(hash_str("ball")),TableIndex::All,TableIndex::All));
  block1.output.insert((TableId::Global(hash_str("ball")),TableIndex::All,TableIndex::All));
  match (&x,&vx,&y,&vy,&g) {
    (Column::f32(x),Column::f32(vx),Column::f32(y),Column::f32(vy),Column::f32(g)) => {
      // #ball.x := #ball.x + #ball.vx
      block1.plan.push(math::ParAddVVIP{out: x.clone(), arg: vx.clone()});
      // #ball.y := #ball.y + #ball.vy    
      block1.plan.push(math::ParAddVVIP{out: y.clone(), arg: vy.clone()});
      // #ball.vy := #ball.vy + #gravity
      block1.plan.push(math::ParAddVSIP{out: vy.clone(), arg: g.clone()});
    }
    _ => (),
  }

  // Keep the balls within the boundary height
  let mut block2 = Block::new();
  block2.add_tfm(Transformation::NewTable{table_id: TableId::Local(hash_str("block2")), rows: 1, columns: 1});
  block2.triggers.insert((TableId::Global(hash_str("time/timer")),TableIndex::All,TableIndex::All));
  block2.input.insert((TableId::Global(hash_str("ball")),TableIndex::All,TableIndex::All));
  block2.output.insert((TableId::Global(hash_str("ball")),TableIndex::All,TableIndex::All));
  match (&y,&iy,&iyy,&iy_or,&c1,&vy2,&vy,&c500,&c0) {
    (Column::f32(y),Column::Bool(iy),Column::Bool(iyy),Column::Bool(iy_or),Column::f32(c1),Column::f32(vy2),Column::f32(vy),Column::f32(m500),Column::f32(m0)) => {
      // iy = #ball.y > #boundary.height
      block2.plan.push(compare::ParGreaterVS{lhs: (y.clone(),0,y.len()-1), rhs: (m500.clone(),0,0), out: iy.clone()});
      // iyy = #ball.y < 0
      block2.plan.push(compare::ParLessVS{lhs: (y.clone(),0,y.len()-1), rhs: (m0.clone(),0,0), out: iyy.clone()});
      // #ball.y{iy} := #boundary.height
      block2.plan.push(table::ParSetVSB{arg: m500.clone(), ix: 0, out:  y.clone(), oix: iy.clone()});
      // #ball.vy{iy | iyy} := #ball.vy * -80%
      block2.plan.push(logic::ParOrVV{lhs: iy.clone(), rhs: iyy.clone(), out: iy_or.clone()});
      block2.plan.push(math::ParMulVS{lhs: vy.clone(), rhs: c1.clone(), out: vy2.clone()});
      block2.plan.push(table::ParSetVVB{arg: vy2.clone(), out: vy.clone(), oix: iy_or.clone()});
    }
    _ => (),
  }

  // Keep the balls within the boundary width
  let mut block3 = Block::new();
  block3.add_tfm(Transformation::NewTable{table_id: TableId::Local(hash_str("block3")), rows: 1, columns: 1});
  block3.triggers.insert((TableId::Global(hash_str("time/timer")),TableIndex::All,TableIndex::All));
  block3.input.insert((TableId::Global(hash_str("ball")),TableIndex::All,TableIndex::All));
  block3.output.insert((TableId::Global(hash_str("ball")),TableIndex::All,TableIndex::All));
  match (&x,&ix,&ixx,&ix_or,&vx,&c1,&vx2,&c500,&c0) {
    (Column::f32(x),Column::Bool(ix),Column::Bool(ixx),Column::Bool(ix_or),Column::f32(vx),Column::f32(c1),Column::f32(vx2),Column::f32(m500),Column::f32(m0)) => {
      // ix = #ball.x > #boundary.width
      block3.plan.push(compare::ParGreaterVS{lhs: (x.clone(),0,x.len()-1), rhs: (m500.clone(),0,0), out: ix.clone()});
      // ixx = #ball.x < 0
      block3.plan.push(compare::ParLessVS{lhs: (x.clone(),0,x.len()-1), rhs: (m0.clone(),0,0), out: ixx.clone()});
      // #ball.x{ix} := #boundary.width
      block3.plan.push(table::ParSetVSB{arg: m500.clone(), ix: 0, out: x.clone(), oix: ix.clone()});
      // #ball.vx{ix | ixx} := #ball.vx * -80%
      block3.plan.push(logic::ParOrVV{lhs: ix.clone(), rhs: ixx.clone(), out: ix_or.clone()});
      block3.plan.push(math::ParMulVS{lhs: vx.clone(), rhs: c1.clone(), out: vx2.clone()});
      block3.plan.push(table::ParSetVVB{arg: vx2.clone(), out: vx.clone(), oix: ix_or.clone()});
    }
    _ => (),
  }

  //println!("{:?}", block1);
  let block1_ref = Rc::new(RefCell::new(block1));
  core.insert_block(block1_ref.clone());

  //println!("{:?}", block2);
  let block2_ref = Rc::new(RefCell::new(block2));
  core.insert_block(block2_ref.clone());

  //println!("{:?}", block3);
  let block3_ref = Rc::new(RefCell::new(block3));
  core.insert_block(block3_ref.clone());

  core.schedule_blocks();

  //println!("{:?}", core);
let mut total_time = VecDeque::new();  
  for i in 0..5000 {
    let txn = vec![Change::Set((hash_str("time/timer"), 
      vec![(TableIndex::Index(1), TableIndex::Index(2), Value::f32(i as f32))]))];
    
    let start_ns = time::precise_time_ns();
    core.process_transaction(&txn)?;
    let end_ns = time::precise_time_ns();

    let cycle_duration = (end_ns - start_ns) as f32;
    total_time.push_back(cycle_duration);
    if total_time.len() > 1000 {
      total_time.pop_front();
    }
    let average_time: f32 = total_time.iter().sum::<f32>() / total_time.len() as f32; 
    println!("{:e} - {:0.2?}Hz", n, 1.0 / (average_time / 1_000_000_000.0));
  }
  let average_time: f32 = total_time.iter().sum::<f32>() / total_time.len() as f32; 
  println!("{:e} - {:0.2?}Hz", n, 1.0 / (average_time / 1_000_000_000.0));
  let end_ns0 = time::precise_time_ns();
  let time = (end_ns0 - start_ns0) as f32;
  println!("{:0.4?} s", time / 1e9);
  println!("{:?}", core);

  Ok(())
}