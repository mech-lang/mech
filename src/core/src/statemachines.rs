// Adapted from rust-fsm (MIT License https://github.com/eugene-babichenko/rust-fsm/commit/1a7d22b7d139bf810938366cf895b6cffe057436)

use core::fmt;
use std::error::Error;
use hashbrown::HashMap;

pub trait StateMachineImpl {
  type Input;
  type State;
  type Output;
  #[allow(clippy::declare_interior_mutable_const)]
  const INITIAL_STATE: Self::State;
  fn transition(state: &Self::State, input: &Self::Input, transitions: &HashMap<(Self::State,Self::Input),(Self::State,Option<Self::Output>)>) -> Option<(Self::State,Option<Self::Output>)>;
}

pub struct StateMachine<T: StateMachineImpl> {
  state: T::State,
  transitions: HashMap<(T::State,T::Input),(T::State, Option<T::Output>)>
}

#[derive(Debug, Clone)]
pub struct TransitionImpossibleError;

impl<T> StateMachine<T>
where
  T: StateMachineImpl,
{
  pub fn new() -> Self {
    Self::from_state(T::INITIAL_STATE)
  }

  pub fn from_state(state: T::State) -> Self {
    Self { 
      state, 
      transitions: HashMap::new() 
    }
  }

  pub fn consume(
    &mut self,
    input: &T::Input,
  ) -> Result<Option<T::Output>, TransitionImpossibleError> {
    if let Some((state,output)) = T::transition(&self.state, input, &self.transitions) {
      self.state = state;
      Ok(output)
    } else {
      Err(TransitionImpossibleError)
    }
  }

  pub fn state(&self) -> &T::State {
    &self.state
  }

  pub fn transitions(&self) -> &HashMap<(T::State,T::Input),(T::State, Option<T::Output>)> {
    &self.transitions
  }

  pub fn transitions_mut(&mut self) -> &mut HashMap<(T::State,T::Input),(T::State, Option<T::Output>)> {
    &mut self.transitions
  }

}

impl<T> Default for StateMachine<T>
where
  T: StateMachineImpl,
{
  fn default() -> Self {
    Self::new()
  }
}

impl fmt::Display for TransitionImpossibleError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(
      f,
      "cannot perform a state transition from the current state with the provided input"
    )
  }
}

#[cfg(feature = "std")]
impl Error for TransitionImpossibleError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    None
  }
}