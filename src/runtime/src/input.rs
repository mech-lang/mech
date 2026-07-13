use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use mech_core::{MResult, MechError, MechErrorKind, Value};
use mech_program::ProgramInputId;

#[derive(Clone, Debug)]
pub struct RuntimeHostInput {
  pub source: RuntimeHostInputSource,
  pub sequence: Option<u64>,
  pub timestamp_ns: Option<u128>,
  pub value: Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RuntimeHostInputSource {
  pub instance: String,
  pub context: String,
  pub path: String,
}

#[derive(Clone, Debug)]
pub struct RuntimeLiveInputBinding {
  pub source: RuntimeHostInputSource,
  pub program_input: ProgramInputId,
}

#[derive(Clone, Debug)]
pub struct RuntimeHostInputOutcome {
  pub source: RuntimeHostInputSource,
  pub binding_count: usize,
  pub changed_input_count: usize,
  pub solved: bool,
  pub plan_steps: usize,
}

#[derive(Clone, Debug)]
pub struct RuntimeIngress {
  queue: Arc<Mutex<Option<VecDeque<RuntimeHostInput>>>>,
}

impl RuntimeIngress {
  pub(crate) fn new(queue: Arc<Mutex<Option<VecDeque<RuntimeHostInput>>>>) -> Self { Self { queue } }

  pub fn submit(&self, input: RuntimeHostInput) -> MResult<()> {
    let mut guard = self.queue.lock().map_err(|_| input_error("RuntimeIngressUnavailable", "host input queue lock is poisoned"))?;
    let Some(queue) = guard.as_mut() else {
      return Err(input_error("RuntimeIngressClosed", "host input queue is closed"));
    };
    queue.push_back(input);
    Ok(())
  }
}

pub trait RuntimeHostInputDriver: std::fmt::Debug {
  fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()>;
  fn start(&mut self) -> MResult<()>;
  fn stop(&mut self) -> MResult<()>;
  fn is_live(&self) -> bool;
}

pub(crate) type RuntimeHostInputQueue = Arc<Mutex<Option<VecDeque<RuntimeHostInput>>>>;

#[derive(Debug, Clone)]
pub struct RuntimeHostInputError { pub name: &'static str, pub message: String }
impl MechErrorKind for RuntimeHostInputError {
  fn name(&self) -> &str { self.name }
  fn message(&self) -> String { self.message.clone() }
}
pub(crate) fn input_error(name: &'static str, message: impl Into<String>) -> MechError {
  MechError::new(RuntimeHostInputError { name, message: message.into() }, None)
}

#[cfg(test)]
mod tests {
  use super::*;
  use mech_core::Ref;

  fn input(sequence: u64) -> RuntimeHostInput {
    RuntimeHostInput {
      source: RuntimeHostInputSource { instance: "i".into(), context: "c".into(), path: "p".into() },
      sequence: Some(sequence),
      timestamp_ns: None,
      value: Value::F64(Ref::new(sequence as f64)),
    }
  }

  #[test]
  fn cloned_ingress_preserves_order_and_bounded_drain() {
    let queue = Arc::new(Mutex::new(Some(VecDeque::new())));
    let ingress = RuntimeIngress::new(queue.clone());
    let cloned = ingress.clone();
    ingress.submit(input(1)).unwrap();
    cloned.submit(input(2)).unwrap();

    let mut guard = queue.lock().unwrap();
    let queue = guard.as_mut().unwrap();
    assert_eq!(queue.pop_front().unwrap().sequence, Some(1));
    assert_eq!(queue.pop_front().unwrap().sequence, Some(2));
  }

  #[test]
  fn closed_ingress_returns_clear_error() {
    let queue = Arc::new(Mutex::new(None));
    let ingress = RuntimeIngress::new(queue);
    let error = format!("{:?}", ingress.submit(input(1)).unwrap_err());
    assert!(error.contains("RuntimeIngressClosed"));
  }
}
