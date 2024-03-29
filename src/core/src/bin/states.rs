/// A dummy implementation of the Circuit Breaker pattern to demonstrate
/// capabilities of this library.
/// https://martinfowler.com/bliki/CircuitBreaker.html
use mech_core::statemachines::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use hashbrown::HashMap;

fn main() {
    let mut machine: StateMachine = StateMachine::new();
    let mut transitions = machine.transitions_mut();
    transitions.insert((State::Closed, Input::Unsuccessful),(State::Open,Output::SetTimer));
    transitions.insert((State::Open, Input::TimerTriggered),(State::HalfOpen,Output::None));
    transitions.insert((State::HalfOpen, Input::Successful),(State::Closed,Output::None));
    transitions.insert((State::HalfOpen, Input::Unsuccessful),(State::Open,Output::SetTimer));

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
    }
    println!("Success!");
}