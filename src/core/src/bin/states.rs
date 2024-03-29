/// A dummy implementation of the Circuit Breaker pattern to demonstrate
/// capabilities of this library.
/// https://martinfowler.com/bliki/CircuitBreaker.html
use mech_core::statemachines::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use hashbrown::HashMap;

#[derive(Debug,Copy,Clone,PartialEq,Eq,Hash)]
enum CBInput {
    Successful,
    Unsuccessful,
    TimerTriggered,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum CBState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, PartialEq, Clone, Copy)]
struct CBOutputSetTimer;

#[derive(Debug)]
struct CBMachine;

impl StateMachineImpl for CBMachine {
    type Input = CBInput;
    type State = CBState;
    type Output = CBOutputSetTimer;
    const INITIAL_STATE: Self::State = CBState::Closed;

    fn transition(state: &Self::State, input: &Self::Input, transitions: &HashMap<(Self::State,Self::Input),(Self::State,Option<Self::Output>)>) -> Option<(Self::State,Option<Self::Output>)> {
        let next_state = transitions.get(&(*state,*input));
        next_state.copied()
    }

}

fn main() {
    let mut machine: StateMachine<CBMachine> = StateMachine::new();
    let mut transitions = machine.transitions_mut();
    transitions.insert((CBState::Closed, CBInput::Unsuccessful),(CBState::Open,Some(CBOutputSetTimer)));
    transitions.insert((CBState::Open, CBInput::TimerTriggered),(CBState::HalfOpen,None));
    transitions.insert((CBState::HalfOpen, CBInput::Successful),(CBState::Closed,None));
    transitions.insert((CBState::HalfOpen, CBInput::Unsuccessful),(CBState::Open,Some(CBOutputSetTimer)));

    // Unsuccessful request
    let machine = Arc::new(Mutex::new(machine));
    {
        let mut lock = machine.lock().unwrap();
        let res = lock.consume(&CBInput::Unsuccessful).unwrap();
        assert_eq!(res, Some(CBOutputSetTimer));
        assert_eq!(lock.state(), &CBState::Open);
    }

    // Set up a timer
    let machine_wait = machine.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::new(5, 0));
        let mut lock = machine_wait.lock().unwrap();
        let res = lock.consume(&CBInput::TimerTriggered).unwrap();
        assert_eq!(res, None);
        assert_eq!(lock.state(), &CBState::HalfOpen);
    });

    // Try to pass a request when the circuit breaker is still open
    let machine_try = machine.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::new(1, 0));
        let mut lock = machine_try.lock().unwrap();
        let res = lock.consume(&CBInput::Successful);
        assert!(matches!(res, Err(TransitionImpossibleError)));
        assert_eq!(lock.state(), &CBState::Open);
    });

    // Test if the circit breaker was actually closed
    std::thread::sleep(Duration::new(7, 0));
    {
        let mut lock = machine.lock().unwrap();
        let res = lock.consume(&CBInput::Successful).unwrap();
        assert_eq!(res, None);
        assert_eq!(lock.state(), &CBState::Closed);
    }
    println!("Success!");
}