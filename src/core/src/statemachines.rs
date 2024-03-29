// Adapted from rust-fsm (MIT License https://github.com/eugene-babichenko/rust-fsm/commit/1a7d22b7d139bf810938366cf895b6cffe057436)

use core::fmt;
use std::error::Error;
use hashbrown::HashMap;

#[derive(Debug,Copy,Clone,PartialEq,Eq,Hash)]
 pub enum Input {
    Successful,
    Unsuccessful,
    TimerTriggered,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum State {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TransitionError {
  Impossible,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Output {
  SetTimer,
  None,
}

pub struct StateMachine {
  state: State,
  transitions: HashMap<(State,Input),(State,Output)>
}

impl StateMachine {

  pub fn new() -> Self {
    StateMachine::from_state(State::Closed)
  }

  pub fn from_state(state: State) -> Self {
    Self { 
      state, 
      transitions: HashMap::new() 
    }
  }

  pub fn consume(&mut self, input: Input) -> Result<Output, TransitionError> {
    match self.transitions.get(&(self.state,input)) {
      Some((state,output)) => {
        self.state = *state;
        Ok(*output)
      }
      None => {
        Err(TransitionError::Impossible)
      }
    }
  }

  pub fn state(&self) -> &State {
    &self.state
  }

  pub fn transitions(&self) -> &HashMap<(State,Input),(State,Output)> {
    &self.transitions
  }

  pub fn transitions_mut(&mut self) -> &mut HashMap<(State,Input),(State,Output)> {
    &mut self.transitions
  }

}

impl Default for StateMachine {
  fn default() -> Self {
    Self::new()
  }
}