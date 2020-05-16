extern crate mech_core;
extern crate serde; // 1.0.68
#[macro_use]
extern crate serde_derive; // 1.0.68

use mech_core::{Core, Value, Quantity, ToQuantity, QuantityMath, make_quantity};

extern crate hashbrown;
use hashbrown::hash_map::HashMap;
use serde::*;
use serde::ser::{Serialize, Serializer, SerializeSeq, SerializeMap};
use std::rc::Rc;
use std::cell::RefCell;
extern crate core;
use core::fmt;
use std::time::{Duration, SystemTime};


pub struct Table {
  pub store:  Rc<RefCell<Store>>,
  pub rows: usize,
  pub columns: usize,
  pub data: Vec<usize>,
}

impl Table {

  pub fn new(store: Rc<RefCell<Store>>, rows: usize, columns: usize) -> Table {
    Table {
      store,
      rows,
      columns,
      data: vec![0; rows*columns],
    }
  }

  
  pub fn index(&self, row: usize, column: usize) -> Option<usize> {
    if row <= self.rows && column <= self.columns && row > 0 && column > 0 {
      Some((row - 1) * self.columns + (column - 1))
    } else {
      None
    }
    
  }

  pub fn get(&self, row: usize, column: usize) -> Option<usize> {
    match self.index(row, column) {
      Some(ix) => Some(self.data[ix]),
      None => None,
    }
  }

  
  pub fn set(&mut self, row: usize, column: usize, value: Value) {
    let mut s = self.store.borrow_mut();
    let ix = self.index(row, column).unwrap();
    let old_address = self.data[ix];
    s.dereference(old_address);
    let new_address = s.intern(value);
    self.data[ix] = new_address;
  }

}

impl fmt::Debug for Table {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let rows = if self.rows > 10 {
      10
    } else {
      self.rows
    };
    for i in 0..rows {
      write!(f, "│ ", )?;
      for j in 0..self.columns {
        match self.get(i+1,j+1) {
          Some(x) => {
            let value = &self.store.borrow().data[x];
            write!(f, "{:?} │ ", value)?;
          },
          _ => (),
        }
        
      }
      write!(f, "\n")?;
    }
    
    Ok(())
  }
}

pub struct Store {
  capacity: usize,
  next: usize,
  free_end: usize,
  free_next: usize,
  free: Vec<usize>,
  data_end: usize,
  reference_counts: Vec<u16>,
  data: Vec<Value>,
}


impl Store {
  pub fn new(capacity: usize) -> Store {
    let mut rc = vec![0; capacity];
    rc[0] = 1;
    Store {
      capacity,
      next: 1,
      free_end: 0,
      free_next: 0,
      free: vec![0; capacity],
      data_end: 1,
      reference_counts: rc,
      data: vec![Value::Empty; capacity],
    }
  }

  pub fn dereference(&mut self, address: usize) {
    if address == 0 {
      // Do nothing
    } else if self.reference_counts[address] == 1 {
      self.reference_counts[address] = 0;
      self.free[self.free_end] = address;
      if self.free_end + 1 == self.free.len() {
        self.free_end = 0;
      } else {
        self.free_end += 1;
      }
    } else {
      self.reference_counts[address] = self.reference_counts[address] - 1;
    }
  }

  pub fn intern(&mut self, value: Value) -> usize {
    self.reference_counts[self.next] = 1;
    let address = self.next;
    self.data[address] = value;
    if self.data_end + 1 == self.capacity {
      self.next = self.free[self.free_next];
      if self.free_next + 1 == self.free.len() {
        self.free_next = 0;
      } else {
        self.free_next += 1;
      }
    } else {
      self.data_end += 1;
      self.next = self.data_end;
    }
    address
  }


}

impl fmt::Debug for Store {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "capacity: {:?}\n", self.capacity)?;
    write!(f, "next: {:?}\n", self.next)?;
    write!(f, "end: {:?}\n", self.data_end)?;
    write!(f, "free-next: {:?}\n", self.free_next)?;
    write!(f, "free-end: {:?}\n", self.free_end)?;
    write!(f, "free: {:?}\n", self.free)?;
    write!(f, "rc  : {:?}\n", self.reference_counts)?;
    write!(f, "data: {:?}\n", self.data)?;
    
    Ok(())
  }
}

fn main() {
  
  let mut store = Rc::new(RefCell::new(Store::new(4000*4 * 4)));
  let balls = 4000;

  let mut table = Table::new(store.clone(),balls,4);
  for i in 1..balls+1 {
    table.set(i,1,Value::from_u64(i as u64));
    table.set(i,2,Value::from_u64(i as u64));
    table.set(i,3,Value::from_u64(20));
    table.set(i,4,Value::from_u64(0));
  }
   
  println!("{:?}\n", table);

  let mut gravity = Table::new(store.clone(),1,1);  
  gravity.set(1,1,Value::from_u64(9));  

  println!("{:?}\n", gravity);

  let rounds = 1000.0;
  let start_ns = time::precise_time_ns();
  for j in 0..rounds as usize {
    for i in 1..balls+1 {
      let v3;
      {
        let s = store.borrow();
        let v1 = &s.data[table.get(i,1).unwrap()];
        let v2 = &s.data[table.get(i,3).unwrap()];
        v3 = v1.as_quantity().unwrap().add(v2.as_quantity().unwrap()).unwrap();
      }
      let v3 = Value::from_quantity(v3);
      table.set(i,1,v3);
    
      let v3;
      {
        let s = store.borrow();
        let v1 = &s.data[table.get(i,2).unwrap()];
        let v2 = &s.data[table.get(i,4).unwrap()];
        v3 = v1.as_quantity().unwrap().add(v2.as_quantity().unwrap()).unwrap();
      }
      let v3 = Value::from_quantity(v3);
      table.set(i,2,v3);
    
      let v3;
      {
        let s = store.borrow();
        let v1 = &s.data[table.get(i,4).unwrap()];
        let v2 = &s.data[gravity.get(1,1).unwrap()];
        v3 = v1.as_quantity().unwrap().add(v2.as_quantity().unwrap()).unwrap();
      }
      let v3 = Value::from_quantity(v3);
      table.set(i,4,v3);
    }
  }
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f64 / 1000000.0;   
  let per_iteration_time = time / rounds;
  println!("{:?}s total", time / 1000.0);  
  println!("{:?}ms per iteration", per_iteration_time);  

  println!("{:?}\n", table);
  //println!("{:?}", store);

}