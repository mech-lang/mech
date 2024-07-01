extern crate time;
use std::time::Instant;
use mech_core::statemachines::*;
use mech_core::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use hashbrown::HashMap;
use std::collections::VecDeque;
use core::ops::Mul;
use core::fmt::*;
use num_traits::identities::*;
use num_traits::Zero;
use std::ops::*;

extern crate nalgebra as na;
use na::{Vector3, DVector, DMatrix, Rotation3, Matrix2x3, Matrix6, Matrix2};

#[derive(Debug, Clone)]
enum Table<T>
where T: Zero + One + Clone + PartialEq + Debug + AddAssign + MulAssign + 'static {
  Vector3(Vector3<T>),
  Matrix2x3(Matrix2x3<T>),
  Matrix6(Matrix6<T>),
  DVector(DVector<T>),
  DMatrix(DMatrix<T>),
}

impl<T> Table<T>
where T: Zero + One + Clone + PartialEq + Debug + AddAssign + MulAssign + 'static {

  fn new(rows: usize, cols: usize, default: T) -> Table<T> {
    match (rows, cols) {
      (1,3) => Table::Vector3(Vector3::from_element(default)),
      (2,3) => Table::Matrix2x3(Matrix2x3::from_element(default)),
      (6,6) => Table::Matrix6(Matrix6::from_element(default)),
      _ => Table::Vector3(Vector3::from_element(default)),
    }
  }

  fn size(&self) -> (usize,usize) {
    match self {
      Table::Vector3(_) => (1,3),
      Table::Matrix2x3(_) => (2,3),
      Table::Matrix6(_) => (6,6),
      Table::DVector(v) => (v.nrows(),1),
      Table::DMatrix(v) => (v.nrows(),v.ncols()),
    }
  }

  fn nrows(&self) -> usize {
    let (nrows, _) = self.size();
    nrows
  }

  fn ncols(&self) -> usize {
    let (_, ncols) = self.size();
    ncols
  }

  fn mat_mul(&self, rhs: &Self) -> std::result::Result<Self,MechError> {
    match (self,rhs) {
      (Table::Matrix6(mat_l),Table::Matrix6(mat_r)) => {
        let result = mat_l * mat_r;
        Ok(Table::Matrix6(result))
      }
      (Table::DMatrix(mat_l),Table::DMatrix(mat_r)) => {
        let result = mat_l * mat_r;
        Ok(Table::DMatrix(result))
      }
      _ => Err(MechError {
        tokens: vec![],
        id: 1234,
        kind: MechErrorKind::None,
        msg: String::from(""),
      }),
    }
  }

  fn add(&self, rhs: &Self) -> std::result::Result<Self,MechError> {
    match (self,rhs) {
      (Table::Matrix6(l),Table::Matrix6(r)) => {
        let result = l + r;
        Ok(Table::Matrix6(result))
      }
      (Table::DVector(l),Table::DVector(r)) => {
        let result = l + r;
        Ok(Table::DVector(result))
      }
      _ => Err(MechError {
        tokens: vec![],
        id: 1234,
        kind: MechErrorKind::None,
        msg: String::from(""),
      }),
    }
  }

  fn add_mut(&mut self, rhs: &Self) -> std::result::Result<(),MechError> {
    match (self,rhs) {
      (Table::Matrix6(l),Table::Matrix6(r)) => {
        *l += r;
        Ok(())
      }
      (Table::DVector(l),Table::DVector(r)) => {
        *l += r;
        Ok(())
      }
      _ => Err(MechError {
        tokens: vec![],
        id: 1234,
        kind: MechErrorKind::None,
        msg: String::from(""),
      }),
    }
  }

  fn add_scalar_mut(&mut self, rhs: T) -> std::result::Result<(),MechError> {
    match self {
      Table::Matrix6(l) => {
        l.add_scalar_mut(rhs);
        Ok(())
      }
      Table::DVector(l) => {
        l.add_scalar_mut(rhs);
        Ok(())
      }
      _ => Err(MechError {
        tokens: vec![],
        id: 1234,
        kind: MechErrorKind::None,
        msg: String::from(""),
      }),
    }
  }

}

fn main() {
  
  let mut a = Table::new(6,6,1 as u8);
  let b = Table::new(6,6,0 as u8);

  let n1 = 1e7 as usize;
  let q = 200;
  let mut xm = DMatrix::from_element(q, q, 1 as f32);
  let mut vm = DMatrix::from_element(q, q, 2 as f32);
  let mut outm = DMatrix::from_element(q, q, 1 as f32);

  let mut x = Table::DVector(DVector::from_element(n1, 1 as f32));
  let vx = Table::DVector(DVector::from_element(n1, 1 as f32));
  let mut y = Table::DVector(DVector::from_element(n1, 1 as f32));
  let mut vy = Table::DVector(DVector::from_element(n1, 1 as f32));
  let mut bx = Table::DVector(DVector::from_element(n1, 1 as f32));
  let mut by = Table::DVector(DVector::from_element(n1, 1 as f32));
  let mut g = Table::DVector(DVector::from_element(n1, 1 as f32));
  let mut aa = DVector::from_element(n1, 1 as f32);
  let mut bb = DVector::from_element(n1, 1 as f32);



  let n = 1e0 as usize;
  let mut total_time = VecDeque::new();  
  //loop {
  let mut max = 0.0;
  let mut min = 0.0;
  for _ in 0..100000 {
    let now = Instant::now();
    vm.mul_to(&xm,&mut outm);
    //  x.add_mut(&vx);
    //  y.add_mut(&vy);
    //  vy.add_mut(&g);
    //let ix = aa > bb;
    //let iy = y > by;
    let elapsed_time = now.elapsed();

    let cycle_duration = elapsed_time.as_nanos() as f64;
    total_time.push_back(cycle_duration);
    if total_time.len() > 10000 {
      total_time.pop_front();
    }
    if cycle_duration > max {
      max = cycle_duration;
    }
    if cycle_duration < min || min == 0.0 {
      min = cycle_duration;
    }
    let average_time: f64 = total_time.iter().sum::<f64>() / total_time.len() as f64; 
    println!("{:e} - {:0.2?}Hz", n, 1.0 / (average_time / 1_000_000_000.0));
  }
  println!("Max: {:?} Min: {:?}", 1.0 / (max / 1_000_000_000.0), 1.0 / (min / 1_000_000_000.0));



  
  let n = 1e5 as usize;
  
  let now = Instant::now();
  for _ in 0..n {
    let result = match &a.mat_mul(&b) {
      Ok(result) => result.clone(),
      _ => panic!("Oh No!"),
    };
    a = result;
  }
  let elapsed_time = now.elapsed();
  
  println!("{:?}", elapsed_time.as_nanos() as f64 / n as f64);
  //println!("{:?}", one);
  
  /*
  //println!("{:?}", vec);

  let code = r#"
  traffic_light(x,t) := { <1s>=>🟢=[6s]=>🟡=[10s]=>🔴=[10s]=>🟢 }

  t1 = traffic_light"#;

  let gn_id = hash_str("🟢");
  let yw_id = hash_str("🟡");
  let rd_id = hash_str("🔴");

  let gn_state = State::Id(gn_id);
  let yw_state = State::Id(yw_id);
  let rd_state = State::Id(rd_id);

  let mut machine: StateMachine = StateMachine::from_state(gn_state);
  machine.add_transition((gn_state, Event::TimerExpired),(yw_state,Output::SetTimer(6)));
  machine.add_transition((yw_state, Event::TimerExpired),(rd_state,Output::SetTimer(10)));
  machine.add_transition((rd_state, Event::TimerExpired),(gn_state,Output::SetTimer(10)));
  //println!("{:#?}", machine);

  
  let mut event_queue: Vec<Event> = vec![
    Event::TimerExpired
  ];

  let mut total_time = VecDeque::new();  
  loop {
    break;
    match event_queue.pop() {
      Some(event) => {
        let now = Instant::now();
        match machine.consume(event) {
          Ok(output) => {
            match output {
              Output::SetTimer(time) => {
                println!("{:?}", time);
                event_queue.push(Event::TimerExpired);
              }
              Output::Value(value) => {
                
              }
              Output::None => {
                  
              }
            }
          }
          Err(error) => {
            println!("{:?}", error);
            break;
          }
        }
        let elapsed_time = now.elapsed();
        println!("{:?}", elapsed_time);
        let cycle_duration = elapsed_time.as_nanos() as f64;
        total_time.push_back(cycle_duration);
        if total_time.len() > 1000 {
          total_time.pop_front();
        }
        let average_time: f64 = total_time.iter().sum::<f64>() / total_time.len() as f64; 
        println!("{:0.2?}Hz", 1.0 / (cycle_duration / 1_000_000_000.0));
      }
      None => {
        break;
      }
    }
  }
*/
  //println!("{:#?}", machine);
/*
    // Unsuccessful request
    let machine = Arc::new(Mutex::new(machine));
    {
        let mut lock = machine.lock().unwrap();
        let res = lock.consume(Input::Unsuccessful).unwrap();
        assert_eq!(res, Output::SetTimer);
        assert_eq!(lock.state(), &State::Open);
    }


    
    // Set up a timer
    let machine_wait = machine.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::new(5, 0));
        let mut lock = machine_wait.lock().unwrap();
        let res = lock.consume(Input::TimerTriggered).unwrap();
        assert_eq!(res, Output::None);
        assert_eq!(lock.state(), &State::HalfOpen);
    });

    // Try to pass a request when the circuit breaker is still open
    let machine_try = machine.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::new(1, 0));
        let mut lock = machine_try.lock().unwrap();
        let res = lock.consume(Input::Successful);
        assert!(matches!(res, Err(TransitionError::Impossible)));
        assert_eq!(lock.state(), &State::Open);
    });

    // Test if the circit breaker was actually closed
    std::thread::sleep(Duration::new(7, 0));
    {
        let mut lock = machine.lock().unwrap();
        let res = lock.consume(Input::Successful).unwrap();
        assert_eq!(res, Output::None);
        assert_eq!(lock.state(), &State::Closed);
    }*/
  println!("Success!");
}