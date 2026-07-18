use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};

use mech_core::{MResult, MechError, MechErrorKind, Ref, Value};
use mech_program::ProgramSolveOutcome;

pub const DEFAULT_HOST_INPUT_CAPACITY: usize = 1024;

#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeHostInputValue {
  Empty,
  Bool(bool),
  String(String),
  U8(u8),
  U16(u16),
  U32(u32),
  U64(u64),
  U128(u128),
  I8(i8),
  I16(i16),
  I32(i32),
  I64(i64),
  I128(i128),
  F32(f32),
  F64(f64),
  Index(usize),
}

impl RuntimeHostInputValue {
  pub fn into_mech_value(self) -> MResult<Value> {
    match self {
      RuntimeHostInputValue::Empty => Ok(Value::Empty),
      #[cfg(feature = "bool")]
      RuntimeHostInputValue::Bool(value) => Ok(Value::Bool(Ref::new(value))),
      #[cfg(not(feature = "bool"))]
      RuntimeHostInputValue::Bool(_) => Err(input_error("RuntimeHostInputValueUnsupported", "bool host input values require the `bool` feature")),
      #[cfg(feature = "string")]
      RuntimeHostInputValue::String(value) => Ok(Value::String(Ref::new(value))),
      #[cfg(not(feature = "string"))]
      RuntimeHostInputValue::String(_) => Err(input_error("RuntimeHostInputValueUnsupported", "string host input values require the `string` feature")),
      #[cfg(feature = "u8")]
      RuntimeHostInputValue::U8(value) => Ok(Value::U8(Ref::new(value))),
      #[cfg(not(feature = "u8"))]
      RuntimeHostInputValue::U8(_) => Err(input_error("RuntimeHostInputValueUnsupported", "u8 host input values require the `u8` feature")),
      #[cfg(feature = "u16")]
      RuntimeHostInputValue::U16(value) => Ok(Value::U16(Ref::new(value))),
      #[cfg(not(feature = "u16"))]
      RuntimeHostInputValue::U16(_) => Err(input_error("RuntimeHostInputValueUnsupported", "u16 host input values require the `u16` feature")),
      #[cfg(feature = "u32")]
      RuntimeHostInputValue::U32(value) => Ok(Value::U32(Ref::new(value))),
      #[cfg(not(feature = "u32"))]
      RuntimeHostInputValue::U32(_) => Err(input_error("RuntimeHostInputValueUnsupported", "u32 host input values require the `u32` feature")),
      #[cfg(feature = "u64")]
      RuntimeHostInputValue::U64(value) => Ok(Value::U64(Ref::new(value))),
      #[cfg(not(feature = "u64"))]
      RuntimeHostInputValue::U64(_) => Err(input_error("RuntimeHostInputValueUnsupported", "u64 host input values require the `u64` feature")),
      #[cfg(feature = "u128")]
      RuntimeHostInputValue::U128(value) => Ok(Value::U128(Ref::new(value))),
      #[cfg(not(feature = "u128"))]
      RuntimeHostInputValue::U128(_) => Err(input_error("RuntimeHostInputValueUnsupported", "u128 host input values require the `u128` feature")),
      #[cfg(feature = "i8")]
      RuntimeHostInputValue::I8(value) => Ok(Value::I8(Ref::new(value))),
      #[cfg(not(feature = "i8"))]
      RuntimeHostInputValue::I8(_) => Err(input_error("RuntimeHostInputValueUnsupported", "i8 host input values require the `i8` feature")),
      #[cfg(feature = "i16")]
      RuntimeHostInputValue::I16(value) => Ok(Value::I16(Ref::new(value))),
      #[cfg(not(feature = "i16"))]
      RuntimeHostInputValue::I16(_) => Err(input_error("RuntimeHostInputValueUnsupported", "i16 host input values require the `i16` feature")),
      #[cfg(feature = "i32")]
      RuntimeHostInputValue::I32(value) => Ok(Value::I32(Ref::new(value))),
      #[cfg(not(feature = "i32"))]
      RuntimeHostInputValue::I32(_) => Err(input_error("RuntimeHostInputValueUnsupported", "i32 host input values require the `i32` feature")),
      #[cfg(feature = "i64")]
      RuntimeHostInputValue::I64(value) => Ok(Value::I64(Ref::new(value))),
      #[cfg(not(feature = "i64"))]
      RuntimeHostInputValue::I64(_) => Err(input_error("RuntimeHostInputValueUnsupported", "i64 host input values require the `i64` feature")),
      #[cfg(feature = "i128")]
      RuntimeHostInputValue::I128(value) => Ok(Value::I128(Ref::new(value))),
      #[cfg(not(feature = "i128"))]
      RuntimeHostInputValue::I128(_) => Err(input_error("RuntimeHostInputValueUnsupported", "i128 host input values require the `i128` feature")),
      #[cfg(feature = "f32")]
      RuntimeHostInputValue::F32(value) => Ok(Value::F32(Ref::new(value))),
      #[cfg(not(feature = "f32"))]
      RuntimeHostInputValue::F32(_) => Err(input_error("RuntimeHostInputValueUnsupported", "f32 host input values require the `f32` feature")),
      #[cfg(feature = "f64")]
      RuntimeHostInputValue::F64(value) => Ok(Value::F64(Ref::new(value))),
      #[cfg(not(feature = "f64"))]
      RuntimeHostInputValue::F64(_) => Err(input_error("RuntimeHostInputValueUnsupported", "f64 host input values require the `f64` feature")),
      RuntimeHostInputValue::Index(value) => Ok(Value::Index(Ref::new(value))),
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RuntimeHostInputSource {
  base_uri: String,
  path: String,
}

impl RuntimeHostInputSource {
  pub fn new(base_uri: impl Into<String>, path: impl Into<String>) -> MResult<Self> {
    let raw_base_uri = base_uri.into();
    let base_uri = crate::resource::canonicalize_resource_base_uri(&raw_base_uri)?;
    let path = path.into().trim_matches('/').to_string();
    Ok(Self { base_uri, path })
  }

  pub fn base_uri(&self) -> &str {
    &self.base_uri
  }

  pub fn path(&self) -> &str {
    &self.path
  }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeHostInputUpdate {
  pub source: RuntimeHostInputSource,
  pub value: RuntimeHostInputValue,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeHostInput {
  pub updates: Vec<RuntimeHostInputUpdate>,
}

impl RuntimeHostInput {
  pub fn new(updates: Vec<RuntimeHostInputUpdate>) -> MResult<Self> {
    let input = Self { updates };
    input.validate()?;
    Ok(input)
  }

  pub fn single(source: RuntimeHostInputSource, value: RuntimeHostInputValue) -> Self {
    Self { updates: vec![RuntimeHostInputUpdate { source, value }] }
  }

  pub fn validate(&self) -> MResult<()> {
    if self.updates.is_empty() {
      return Err(input_error("RuntimeHostInputEmpty", "host input packet must contain at least one update"));
    }
    let mut sources = HashSet::with_capacity(self.updates.len());
    for update in &self.updates {
      if !sources.insert(update.source.clone()) {
        return Err(input_error("RuntimeHostInputDuplicateSource", "host input packet contains duplicate sources"));
      }
    }
    Ok(())
  }
}

#[derive(Clone, Debug)]
pub struct RuntimeHostInputOutcome {
  pub update_count: usize,
  pub ignored_update_count: usize,
  pub binding_count: usize,
  pub solve: Option<ProgramSolveOutcome>,
}

#[derive(Debug)]
pub(crate) struct RuntimeHostInputQueueState {
  pub(crate) queue: VecDeque<RuntimeHostInput>,
  pub(crate) capacity: usize,
  pub(crate) closed: bool,
}

impl RuntimeHostInputQueueState {
  pub(crate) fn new(capacity: usize) -> Self {
    Self { queue: VecDeque::new(), capacity, closed: false }
  }
}

#[derive(Clone, Debug)]
pub struct RuntimeIngress {
  queue: RuntimeHostInputQueue,
}

impl RuntimeIngress {
  pub(crate) fn new(queue: RuntimeHostInputQueue) -> Self { Self { queue } }

  pub fn submit(&self, input: RuntimeHostInput) -> MResult<()> {
    input.validate()?;
    let mut guard = self.queue.lock().map_err(|_| input_error("RuntimeIngressUnavailable", "host input queue lock is poisoned"))?;
    if guard.closed {
      return Err(input_error("RuntimeIngressClosed", "host input queue is closed"));
    }
    if guard.queue.len() >= guard.capacity {
      return Err(input_error("RuntimeIngressFull", "host input queue is full"));
    }
    guard.queue.push_back(input);
    Ok(())
  }

  pub fn is_closed(&self) -> MResult<bool> {
    Ok(self.queue.lock().map_err(|_| input_error("RuntimeIngressUnavailable", "host input queue lock is poisoned"))?.closed)
  }
}

/// Platform-neutral active host input driver.
///
/// `attach`, `start`, and `stop` are called on the runtime thread. `start`
/// must be idempotent or reject an already-live state clearly, and `stop` must
/// be idempotent. Background workers must not retain a runtime pointer or
/// `Value`; they submit only owned `RuntimeHostInput` packets through cloned
/// `RuntimeIngress` handles.
pub trait RuntimeHostInputDriver: std::fmt::Debug {
  fn drives(&self, source: &RuntimeHostInputSource) -> bool;
  fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()>;
  fn start(&mut self) -> MResult<()>;
  fn stop(&mut self) -> MResult<()>;
  fn is_live(&self) -> bool;
}

pub(crate) type RuntimeHostInputQueue = Arc<Mutex<RuntimeHostInputQueueState>>;

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

  fn source(path: &str) -> RuntimeHostInputSource {
    RuntimeHostInputSource::new("test://clock/ticks/", path).unwrap()
  }

  fn packet(path: &str, value: f64) -> RuntimeHostInput {
    RuntimeHostInput::single(source(path), RuntimeHostInputValue::F64(value))
  }

  fn assert_send_sync<T: Send + Sync>() {}


  #[test]
  fn source_constructor_canonicalizes_base_and_path() {
    let source = RuntimeHostInputSource::new("test://clock/ticks/", "/value/").unwrap();
    assert_eq!(source.base_uri(), "test://clock/ticks");
    assert_eq!(source.path(), "value");
  }

  #[test]
  fn source_constructor_rejects_invalid_resource_uris() {
    assert!(RuntimeHostInputSource::new("clock/ticks", "value").is_err());
    assert!(RuntimeHostInputSource::new("://clock", "value").is_err());
    assert!(RuntimeHostInputSource::new("test://", "value").is_err());
  }

  #[test]
  fn host_input_transport_is_send_sync() {
    assert_send_sync::<RuntimeHostInputValue>();
    assert_send_sync::<RuntimeHostInput>();
    assert_send_sync::<RuntimeIngress>();
  }

  #[test]
  fn cloned_ingress_preserves_fifo_and_enforces_capacity() {
    let queue = Arc::new(Mutex::new(RuntimeHostInputQueueState::new(2)));
    let ingress = RuntimeIngress::new(queue.clone());
    let cloned = ingress.clone();
    ingress.submit(packet("a", 1.0)).unwrap();
    cloned.submit(packet("b", 2.0)).unwrap();
    let error = format!("{:?}", ingress.submit(packet("c", 3.0)).unwrap_err());
    assert!(error.contains("RuntimeIngressFull"));

    let mut guard = queue.lock().unwrap();
    assert_eq!(guard.queue.pop_front().unwrap().updates[0].source.path(), "a");
    assert_eq!(guard.queue.pop_front().unwrap().updates[0].source.path(), "b");
  }

  #[test]
  fn closed_ingress_rejects_new_submissions_but_preserves_queued_packets() {
    let queue = Arc::new(Mutex::new(RuntimeHostInputQueueState::new(2)));
    let ingress = RuntimeIngress::new(queue.clone());
    ingress.submit(packet("a", 1.0)).unwrap();
    queue.lock().unwrap().closed = true;
    let error = format!("{:?}", ingress.submit(packet("b", 2.0)).unwrap_err());
    assert!(error.contains("RuntimeIngressClosed"));
    assert!(ingress.is_closed().unwrap());
    assert_eq!(queue.lock().unwrap().queue.len(), 1);
  }

  #[test]
  fn empty_packets_and_duplicate_sources_are_rejected() {
    assert!(RuntimeHostInput::new(Vec::new()).is_err());
    let duplicate = source("value");
    let error = format!("{:?}", RuntimeHostInput::new(vec![
      RuntimeHostInputUpdate { source: duplicate.clone(), value: RuntimeHostInputValue::F64(1.0) },
      RuntimeHostInputUpdate { source: duplicate, value: RuntimeHostInputValue::F64(2.0) },
    ]).unwrap_err());
    assert!(error.contains("RuntimeHostInputDuplicateSource"));
  }
}
