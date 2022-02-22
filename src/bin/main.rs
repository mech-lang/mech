
#![allow(warnings)]
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
use mech_core::*;
use mech_core::function::*;

use std::fmt::*;
use num_traits::*;
use std::ops::*;

#[derive(Clone)]
struct Column<T>(Rc<RefCell<Vec<T>>>);

impl<T: Copy> Column<T> {

  pub fn new(vec: Vec<T>) -> Column<T> {
    Column(Rc::new(RefCell::new(vec)))
  }

  pub fn len(&self) -> usize {
    let Column(col) = self;
    col.borrow().len()
  }

  pub fn get_unchecked(&mut self, row: usize) -> T {
    let Column(col) = self;
    let mut c_brrw = col.borrow_mut();
    c_brrw[row]
  }

  pub fn set_unchecked(&mut self, row: usize, value: T) {
    let Column(col) = self;
    let mut c_brrw = col.borrow_mut();
    c_brrw[row] = value;
  }
  
}

impl<T: Debug> fmt::Debug for Column<T> {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let Column(col) = self;
    let col_brrw = col.borrow();
    write!(f,"[")?;
    for c in col_brrw.iter() {
      write!(f,"{:?}, ",c)?;
    }
    write!(f,"]")?;
    Ok(())
  }
}

mech_type!(F32,f32);
mech_type!(U8,u8);
mech_type!(U16,u16);
mech_type!(U32,u32);
mech_type!(U64,u64);
mech_type!(U128,u128);
mech_type!(I8,i8);
mech_type!(I16,i16);
mech_type!(I32,i32);
mech_type!(I64,i64);
mech_type!(I128,i128);

mech_type_conversion!(U8,F32,f32);
mech_type_conversion!(U8,U64,u64);
mech_type_conversion!(U8,U32,u32);
mech_type_conversion!(U8,U16,u16);
mech_type_conversion!(F32,U8,u8);
mech_type_conversion!(U16,U8,u8);
mech_type_conversion!(U32,U8,u8);
mech_type_conversion!(U64,U8,u8);


fn par_add<T,U>(lhs: &Column<T>, rhs: &Column<U>, out: &Column<U>) 
  where T: Copy + Debug + Clone + Add<Output = T> + Into<U> + Sync + Send,
        U: Copy + Debug + Clone + Add<Output = U> + Into<T> + Sync + Send,
{
  let (Column(lhs),Column(rhs),Column(out)) = (lhs,rhs,out);
  out.borrow_mut().par_iter_mut()
     .zip(lhs.borrow().par_iter().map(|x| T::into(*x)))
     .zip(rhs.borrow().par_iter())
     .for_each(|((out, lhs),rhs)| *out = lhs.add(*rhs)); 
}

fn add<T,U,V>(lhs: &Column<T>, rhs: &Column<U>, out: &Column<V>) 
  where T: Copy + Debug + Clone + Add<Output = T> + Into<V>,
        U: Copy + Debug + Clone + Add<Output = U> + Into<V>,
        V: Copy + Debug + Clone + Add<Output = V>
{
  let (Column(lhs),Column(rhs),Column(out)) = (lhs,rhs,out);
  out.borrow_mut().iter_mut()
     .zip(lhs.borrow().iter().map(|x| T::into(*x)))
     .zip(rhs.borrow().iter().map(|x| U::into(*x)))
     .for_each(|((out, lhs),rhs)| *out = lhs.add(rhs)); 
}

fn copy<T,U>(arg: &Column<T>, out: &Column<U>, start: usize) 
  where T: Copy + Debug + Clone + Into<U>,
        U: Copy + Debug + Clone + Into<T>,
{
  let (Column(arg),Column(out)) = (arg,out);
  let mut o = out.borrow_mut();
  o[start..].iter_mut().zip(arg.borrow().iter().map(|x| T::into(*x))).for_each(|(out, arg)| *out = arg.clone()); 
}

#[macro_export]
macro_rules! mech_type {
  ($wrapper:tt,$type:tt) => (
    #[derive(Copy,Clone)]
    struct $wrapper($type);
    impl Add for $wrapper {
      type Output = $wrapper;
      fn add(self, rhs: $wrapper) -> $wrapper {
        let ($wrapper(lhs),$wrapper(rhs)) = (self,rhs);
        $wrapper(lhs + rhs)
      }
    }
    impl fmt::Debug for $wrapper {
      #[inline]
      fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let $wrapper(col) = self;
        write!(f,"{:?}",col)?;
        Ok(())
      }
    }
  )
}

#[macro_export]
macro_rules! mech_type_conversion {
  ($from_wrapper:tt,$to_wrapper:tt,$to_type:tt) => (
    impl From<$from_wrapper> for $to_wrapper {
      fn from(n: $from_wrapper) -> $to_wrapper {
        let $from_wrapper(c) = n;
        $to_wrapper(c as $to_type)
      } 
    }
  )
}

fn main() {

  let n = 1e6 as usize;

  let mut f32_1 = Column::<F32>::new(vec![F32(1.0);n]);
  let mut f32_2 = Column::<F32>::new(vec![F32(4.0);n]);
  let mut f32_3 = Column::<F32>::new(vec![F32(0.0);n]);

  let mut u16_1 = Column::<U16>::new(vec![U16(1);n]);
  let mut u16_2 = Column::<U16>::new(vec![U16(4);n]);
  let mut u16_3 = Column::<U16>::new(vec![U16(0);n]);

  let mut u8_1 = Column::<U8>::new(vec![U8(1);n]);
  let mut u8_2 = Column::<U8>::new(vec![U8(4);n]);
  let mut u8_3 = Column::<U8>::new(vec![U8(0);n]);

  let i = 4000;

  println!("FLOAT");
  let start_ns = time::precise_time_ns();
  for _ in 0..i {
    add(&f32_1,&f32_2,&f32_3);
  }
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f32;
  println!("{:0.4?} s", time / 1e9);

  println!("U8");
  let start_ns = time::precise_time_ns();
  for _ in 0..i {
    add(&u8_1,&u8_2,&u8_3);
  }
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f32;
  println!("{:0.4?} s", time / 1e9);
  
  println!("U16");
  let start_ns = time::precise_time_ns();
  for _ in 0..i {
    add(&u16_1,&u16_2,&u16_3);
  }
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f32;
  println!("{:0.4?} s", time / 1e9);

  println!("MIXED");
  let start_ns = time::precise_time_ns();
  for _ in 0..i {
    add(&u8_1,&u16_2,&u16_3);
  }
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f32;
  println!("{:0.4?} s", time / 1e9);

  /*let c2 = Column::<U8>::new(vec![U8(0);9]);
  let c3 = Column::<U32>::new(vec![U32(4),U32(5),,U32(5)]);
  let c4 = Column::<U64>::new(vec![U64(6),U64(7),U64(8),U64(9)]);

  copy(&c1,&c2,0);
  copy(&c3,&c2,c1.len());
  copy(&c4,&c2,c1.len() + c3.len());

  add(c1,c2,c2)


  c1.set_unchecked(2,F32(42.0));

  copy(&c1,&c2,0);*/


/* 
  let sizes: Vec<usize> = vec![1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7].iter().map(|x| *x as usize).collect();
  let mut total_time = VecDeque::new();  
  let start_ns0 = time::precise_time_ns();
  let n = 1e6 as usize;

  // Create a core
  let mut core = Core::new();

  {
    // #time/timer += [period: 60Hz]
    let mut time_timer = Table::new(hash_str("time/timer"),1,2);
    time_timer.set_col_kind(0,ValueKind::F32);
    time_timer.set_col_kind(1,ValueKind::F32);
    time_timer.set_raw(0,0,Value::F32(60.0));
    core.insert_table(time_timer.clone());

    // #gravity = 1
    let mut gravity = Table::new(hash_str("gravity"),1,1);
    gravity.set_col_kind(0,ValueKind::F32);
    gravity.set_raw(0,0,Value::F32(1.0));
    core.insert_table(gravity.clone());

    // -80%
    let mut const1 = Table::new(hash_str("-0.8"),1,1);
    const1.set_col_kind(0,ValueKind::F32);
    const1.set_raw(0,0,Value::F32(-0.8));
    core.insert_table(const1.clone());

    // 500
    let mut const2 = Table::new(hash_str("500.0"),1,1);
    const2.set_col_kind(0,ValueKind::F32);
    const2.set_raw(0,0,Value::F32(500.0));
    core.insert_table(const2.clone());

    // 0
    let mut const3 = Table::new(hash_str("0.0"),1,1);
    const3.set_col_kind(0,ValueKind::F32);
    const3.set_raw(0,0,Value::F32(0.0));
    core.insert_table(const3.clone());

    // Create balls
    // #balls = [x: 0:n y: 0:n vx: 3.0 vy: 4.0]
    let mut balls = Table::new(hash_str("balls"),n,4);
    balls.set_col_kind(0,ValueKind::F32);
    balls.set_col_kind(1,ValueKind::F32);
    balls.set_col_kind(2,ValueKind::F32);
    balls.set_col_kind(3,ValueKind::F32);
    for i in 0..n {
      balls.set_raw(i,0,Value::F32(i as f32));
      balls.set_raw(i,1,Value::F32(i as f32));
      balls.set_raw(i,2,Value::F32(3.0));
      balls.set_raw(i,3,Value::F32(4.0));
    }
    core.insert_table(balls.clone());
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
  block1.add_tfm(Transformation::NewTable{table_id: TableId::Local(hash_str("block1")), rows: 1, columns: 1});
  block1.triggers.insert((TableId::Global(hash_str("time/timer")),TableIndex::All,TableIndex::All));
  block1.input.insert((TableId::Global(hash_str("gravity")),TableIndex::All,TableIndex::All));
  block1.input.insert((TableId::Global(hash_str("ball")),TableIndex::All,TableIndex::All));
  block1.output.insert((TableId::Global(hash_str("ball")),TableIndex::All,TableIndex::All));
  match (&x,&vx,&y,&vy,&g) {
    (Column::F32(x),Column::F32(vx),Column::F32(y),Column::F32(vy),Column::F32(g)) => {
      // #ball.x := #ball.x + #ball.vx
      block1.plan.push(math::ParAddVVIP::<f32>{out: x.clone(), arg: vx.clone()});
      // #ball.y := #ball.y + #ball.vy    
      block1.plan.push(math::ParAddVVIP::<f32>{out: y.clone(), arg: vy.clone()});
      // #ball.vy := #ball.vy + #gravity
      block1.plan.push(math::ParAddVSIP::<f32>{out: vy.clone(), arg: g.clone()});
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
    (Column::F32(y),Column::Bool(iy),Column::Bool(iyy),Column::Bool(iy_or),Column::F32(c1),Column::F32(vy2),Column::F32(vy),Column::F32(m500),Column::F32(m0)) => {
      // iy = #ball.y > #boundary.height
      block2.plan.push(compare::ParGreaterVS::<f32>{lhs: y.clone(), rhs: m500.clone(), out: iy.clone()});
      // iyy = #ball.y < 0
      block2.plan.push(compare::ParLessVS::<f32>{lhs: y.clone(), rhs: m0.clone(), out: iyy.clone()});
      // #ball.y{iy} := #boundary.height
      block2.plan.push(table::ParSetVSB{arg: m500.clone(), ix: 0, out:  y.clone(), oix: iy.clone()});
      // #ball.vy{iy | iyy} := #ball.vy * -80%
      block2.plan.push(logic::ParOrVV{lhs: iy.clone(), rhs: iyy.clone(), out: iy_or.clone()});
      block2.plan.push(math::ParMulVS::<f32>{lhs: vy.clone(), rhs: c1.clone(), out: vy2.clone()});
      block2.plan.push(table::ParSetVVB::<f32>{arg: vy2.clone(), out: vy.clone(), oix: iy_or.clone()});
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
    (Column::F32(x),Column::Bool(ix),Column::Bool(ixx),Column::Bool(ix_or),Column::F32(vx),Column::F32(c1),Column::F32(vx2),Column::F32(m500),Column::F32(m0)) => {
      // ix = #ball.x > #boundary.width
      block3.plan.push(compare::ParGreaterVS::<f32>{lhs: x.clone(), rhs: m500.clone(), out: ix.clone()});
      // ixx = #ball.x < 0
      block3.plan.push(compare::ParLessVS::<f32>{lhs: x.clone(), rhs: m0.clone(), out: ixx.clone()});
      // #ball.x{ix} := #boundary.width
      block3.plan.push(table::ParSetVSB{arg: m500.clone(), ix: 0, out: x.clone(), oix: ix.clone()});
      // #ball.vx{ix | ixx} := #ball.vx * -80%
      block3.plan.push(logic::ParOrVV{lhs: ix.clone(), rhs: ixx.clone(), out: ix_or.clone()});
      block3.plan.push(math::ParMulVS::<f32>{lhs: vx.clone(), rhs: c1.clone(), out: vx2.clone()});
      block3.plan.push(table::ParSetVVB::<f32>{arg: vx2.clone(), out: vx.clone(), oix: ix_or.clone()});
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

  for i in 0..5000 {
    let txn = vec![Change::Set((hash_str("time/timer"), vec![(TableIndex::Index(1), TableIndex::Index(2), Value::F32(i as f32))]))];
    
    let start_ns = time::precise_time_ns();
    core.process_transaction(&txn)?;
    let end_ns = time::precise_time_ns();

    let cycle_duration = (end_ns - start_ns) as f32;
    total_time.push_back(cycle_duration);
    if total_time.len() > 1000 {
      total_time.pop_front();
    }
    //println!("{:?}", core.get_table("balls"));
    //let average_time: f32 = total_time.iter().sum::<f32>() / total_time.len() as f32; 
    //println!("{:e} - {:0.2?}Hz", n, 1.0 / (average_time / 1_000_000_000.0));
  }
  let average_time: f32 = total_time.iter().sum::<f32>() / total_time.len() as f32; 
  println!("{:e} - {:0.2?}Hz", n, 1.0 / (average_time / 1_000_000_000.0));
  let end_ns0 = time::precise_time_ns();
  let time = (end_ns0 - start_ns0) as f32;
  println!("{:0.4?} s", time / 1e9);
  println!("{:?}", core);
*/

}