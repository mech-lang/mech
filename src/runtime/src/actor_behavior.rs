//! Actor behavior drivers.
//!
//! This is a Rust-side bridge used before Mech syntax is wired to runtime host
//! calls. A behavior driver runs during an actor turn with the actor turn already
//! bound into `RuntimeContext`.
//!
//! The driver receives a narrow runtime surface, not the whole runtime type.
//! This lets it call host functions and runtime services while keeping actor
//! behavior execution mediated by context, capabilities, budgets, events, and
//! transactions.

use mech_core::{
  MResult, Value, Ref
};

use crate::actor::ActorTurn;
use crate::context::RuntimeContext;
use crate::host::HostCall;
use crate::services::RuntimeServices;

// -----------------------------------------------------------------------------
// Runtime surface exposed to actor behavior drivers
// -----------------------------------------------------------------------------

pub trait ActorBehaviorRuntime: RuntimeServices {
  fn call_host_with_context(
    &mut self,
    context: &mut RuntimeContext,
    call: HostCall,
  ) -> MResult<Value>;
}

// -----------------------------------------------------------------------------
// Actor behavior driver
// -----------------------------------------------------------------------------

pub trait ActorBehaviorDriver: std::fmt::Debug + Send {
  fn run_actor_turn(
    &mut self,
    runtime: &mut dyn ActorBehaviorRuntime,
    context: &mut RuntimeContext,
    turn: &ActorTurn,
  ) -> MResult<()>;
}

// -----------------------------------------------------------------------------
// No-op driver
// -----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default)]
pub struct NoActorBehaviorDriver;

impl NoActorBehaviorDriver {
  pub fn new() -> Self {
    Self
  }
}

impl ActorBehaviorDriver for NoActorBehaviorDriver {
  fn run_actor_turn(
    &mut self,
    _runtime: &mut dyn ActorBehaviorRuntime,
    _context: &mut RuntimeContext,
    _turn: &ActorTurn,
  ) -> MResult<()> {
    Ok(())
  }
}

// -----------------------------------------------------------------------------
// Host-call behavior driver
// -----------------------------------------------------------------------------

/// Simple Rust-side actor behavior driver used to prove scheduled actor turns can
/// invoke runtime host functions transactionally.
///
/// It performs:
///
/// - `actor/message/kind`
/// - `actor/message/payload`
/// - `actor/state/get`
/// - `actor/state/put(new_state)`
///
/// The returned values are intentionally ignored here. The event stream and
/// transaction record are the proof.
#[derive(Clone, Debug)]
pub struct HostCallActorBehaviorDriver {
  pub new_state: String,
  pub read_message: bool,
  pub read_state: bool,
  pub write_state: bool,
}

impl HostCallActorBehaviorDriver {
  pub fn new(new_state: impl Into<String>) -> Self {
    Self {
      new_state: new_state.into(),
      read_message: true,
      read_state: true,
      write_state: true,
    }
  }

  pub fn without_message_reads(mut self) -> Self {
    self.read_message = false;
    self
  }

  pub fn without_state_read(mut self) -> Self {
    self.read_state = false;
    self
  }

  pub fn without_state_write(mut self) -> Self {
    self.write_state = false;
    self
  }
}

impl ActorBehaviorDriver for HostCallActorBehaviorDriver {
  fn run_actor_turn(
    &mut self,
    runtime: &mut dyn ActorBehaviorRuntime,
    context: &mut RuntimeContext,
    turn: &ActorTurn,
  ) -> MResult<()> {
    turn.validate()?;

    if self.read_message {
      runtime.call_host_with_context(
        context,
        HostCall::new("actor/message/kind", Vec::new()),
      )?;

      runtime.call_host_with_context(
        context,
        HostCall::new("actor/message/payload", Vec::new()),
      )?;
    }

    if self.read_state {
      runtime.call_host_with_context(
        context,
        HostCall::new("actor/state/get", Vec::new()),
      )?;
    }

    if self.write_state {
      runtime.call_host_with_context(
        context,
        HostCall::new(
          "actor/state/put",
          vec![Value::String(Ref::new(self.new_state.clone()))],
        ),
      )?;
    }

    Ok(())
  }
}