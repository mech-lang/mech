
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

pub type MechFunction = extern "C" fn(arguments: &mut Vec<Vec<f64>>);
pub type Column = Rc<RefCell<Vec<f64>>>;

pub fn hash_string(input: &str) -> u64 {
  seahash::hash(input.to_string().as_bytes()) & 0x00FFFFFFFFFFFFFF
}

#[derive(Clone)]
pub struct Table {
  pub id: u64,
  pub rows: usize,
  pub cols: usize,
  data: Vec<Column>,
}

impl Table {
  pub fn new(id: u64, rows: usize, cols: usize) -> Table {
    let mut table = Table {
      id,
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

pub type ArgF64 = Column;
pub type ArgBool = Rc<RefCell<Vec<bool>>>;
pub type OutF64 = Column;
pub type OutBool = Rc<RefCell<Vec<bool>>>;

#[derive(Debug)]
enum Transformation {
  ParAddVVIP((OutF64, ArgF64)),  
  ParAddVSIP((OutF64, ArgF64)),
  ParMultiplyVS((ArgF64, ArgF64, OutF64)),
  ParOrVV((ArgBool,ArgBool,OutBool)),
  ParLessThanVS((ArgF64,f64,OutBool)),
  ParGreaterThanVS((ArgF64,f64,OutBool)),
  ParCSGreaterThanVS((ArgF64,f64,f64)),
  ParSetVS((ArgBool,f64,OutF64)),
  ParSetVV((ArgBool,ArgF64,OutF64)),
}

impl Transformation {
  pub fn solve(&mut self) {
    match &*self {
      // MATH
      Transformation::ParAddVVIP((lhs, rhs)) => { lhs.borrow_mut().par_iter_mut().zip(&(*rhs.borrow())).for_each(|(lhs, rhs)| *lhs += rhs); }
      Transformation::ParAddVSIP((lhs, rhs)) => { 
        let rhs = rhs.borrow()[0];
        lhs.borrow_mut().par_iter_mut().for_each(|lhs| *lhs += rhs); 
      }
      Transformation::ParMultiplyVS((lhs, rhs, out)) => { 
        let rhs = rhs.borrow()[0];
        out.borrow_mut().par_iter_mut().zip(&(*lhs.borrow())).for_each(|(out, lhs)| *out = *lhs * rhs); 
      }
      // COMPARE
      Transformation::ParGreaterThanVS((lhs, rhs, out)) => { out.borrow_mut().par_iter_mut().zip(&(*lhs.borrow())).for_each(|(out, lhs)| *out = *lhs > *rhs); }
      Transformation::ParLessThanVS((lhs, rhs, out)) => { out.borrow_mut().par_iter_mut().zip(&(*lhs.borrow())).for_each(|(out, lhs)| *out = *lhs < *rhs); }
      Transformation::ParCSGreaterThanVS((lhs, rhs, swap)) => { 
        lhs.borrow_mut().par_iter_mut().for_each(|lhs| if *lhs > *rhs {
          *lhs = *swap;
        }); 
      }

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

pub type Change = (u64, Vec<(usize, usize, f64)>);

pub type Transaction = Vec<Change>;

struct Core {
  blocks: Vec<Rc<RefCell<Block>>>,
  database: Database,
  pub schedules: HashMap<(u64,usize,usize),Vec<Vec<usize>>>,
}

impl Core {

  pub fn new() -> Core {
    Core {
      blocks: Vec::new(),
      database: Database::new(),
      schedules: HashMap::new(),
    }
  }

  pub fn process_transaction(&mut self, txn: &Transaction) -> Result<(),()> {
    let mut registers = Vec::new();
    for (table_id, adds) in txn {
      match self.database.get_table_by_id(table_id) {
        Some(table) => {
          for (row,col,val) in adds {
            match table.set(*row, *col, *val) {
              Err(_) => {
                // Index out of bounds.
                return Err(());
              }
              _ => {
                registers.push((*table_id,*row,*col));
              },
            }
          }
        }
        _ => {
          // Table doesn't exist
          return Err(());
        }
      }
    }
    for register in registers {
      self.step(&register);
    }
    Ok(())
  }

  pub fn insert_table(&mut self, table: Table) -> Option<Table> {
    self.database.insert_table(table)
  }

  pub fn get_table(&mut self, table_name: &str) -> Option<&Table> {
    self.database.get_table(table_name)
  }

  pub fn insert_block(&mut self, block: Block) {
    self.blocks.push(Rc::new(RefCell::new(block)));
  }

  pub fn step(&mut self, register: &(u64,usize,usize)) {
    match &mut self.schedules.get(register) {
      Some(schedule) => {
        for blocks in schedule.iter() {
          for block_ix in blocks {
            self.blocks[*block_ix].borrow_mut().solve();
          }
        }
      }
      _ => (),
    }

  }
}

struct Database {
  tables: HashMap<u64,Table>,
}

impl Database {
  pub fn new() -> Database {
    Database {
      tables: HashMap::new(),
    }
  }

  pub fn insert_table(&mut self, table: Table) -> Option<Table> {
    self.tables.insert(table.id, table)
  }

  pub fn get_table(&mut self, table_name: &str) -> Option<&Table> {
    self.tables.get(&hash_string(table_name))
  }

  pub fn get_table_by_id(&mut self, table_id: &u64) -> Option<&Table> {
    self.tables.get(table_id)
  }

}

pub type Plan = Vec<Rc<RefCell<Transformation>>>;

struct Block {
  id: u64,
  plan: Plan,
}

impl Block {
  pub fn new() -> Block {
    Block {
      id: 0,
      plan: Vec::new(),
    }
  }

  pub fn gen_id(&mut self) -> u64 {
    self.id = hash_string(&format!("{:?}", self.plan));
    self.id
  }

  pub fn id(&self) -> u64 {
    self.id
  }

  pub fn add_tfm(&mut self, tfm: Transformation) {
    self.plan.push(Rc::new(RefCell::new(tfm)));
  }

  pub fn solve(&mut self) {
    for ref mut tfm in &mut self.plan.iter() {
      tfm.borrow_mut().solve();
    }
  }

}

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
      balls.set(i,0,i as f64);
      balls.set(i,1,i as f64);
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

  for i in 0..2000 {
    let txn = vec![(hash_string("time/timer"), vec![(0, 1, i as f64)])];
    let start_ns = time::precise_time_ns();

    core.process_transaction(&txn);

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