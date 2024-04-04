// Adapted from rust-fsm (MIT License https://github.com/eugene-babichenko/rust-fsm/commit/1a7d22b7d139bf810938366cf895b6cffe057436)

use crate::Value;
use core::fmt;
use std::error::Error;
use hashbrown::HashMap;
use crate::database::*;

#[derive(Debug,Copy,Clone,PartialEq,Eq,Hash)]
 pub enum Event {
    OnCreate,
    OnDestroy,
    Success,
    Fail,
    TimerExpired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum State {
    Closed,
    HalfOpen,
    Open,
    Id(u64),
    None,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TransitionError {
  Impossible,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Output {
  Value(Value),
  SetTimer(usize),
  None,
}

#[derive(Debug, Clone)]
pub struct StateMachine {
  state: State,
  tables: Database,
  transitions: HashMap<(State,Event),(State,Output)>
}

impl StateMachine {

  pub fn new() -> Self {
    StateMachine::from_state(State::Closed)
  }

  pub fn from_state(state: State) -> Self {
    Self { 
      state,
      tables: Database::new(),
      transitions: HashMap::new() 
    }
  }

  pub fn consume(&mut self, event: Event) -> Result<Output, TransitionError> {
    match self.transitions.get(&(self.state,event)) {
      Some((state,output)) => {
        self.state = *state;
        Ok(output.clone())
      }
      None => {
        Err(TransitionError::Impossible)
      }
    }
  }

  pub fn state(&self) -> &State {
    &self.state
  }

  pub fn add_transition(&mut self, input: (State,Event), output: (State,Output)) -> Option<(State,Output)> {
    self.transitions.insert(input,output)
  }

  pub fn transitions(&self) -> &HashMap<(State,Event),(State,Output)> {
    &self.transitions
  }

  pub fn transitions_mut(&mut self) -> &mut HashMap<(State,Event),(State,Output)> {
    &mut self.transitions
  }

}

impl Default for StateMachine {
  fn default() -> Self {
    Self::new()
  }
}