extern crate time;
use std::time::Instant;
use mech_core::statemachines::*;
use mech_core::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use hashbrown::HashMap;
use std::collections::VecDeque;

fn main() {

  let code = r#"
  traffic_light(x) := { <1s>=>🟢=[6s]=>🟡=[10s]=>🔴=[10s]=>🟢 }

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
  println!("{:#?}", machine);

  
  let mut event_queue: Vec<Event> = vec![
    Event::TimerExpired
  ];

  let mut total_time = VecDeque::new();  
  loop {
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

    println!("{:#?}", machine);
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