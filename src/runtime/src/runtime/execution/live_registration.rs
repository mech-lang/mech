use crate::runtime::{
  LiveRegistrationMode,
  MechRuntime,
};
#[cfg(test)]
use crate::RuntimeContext;
use mech_core::MResult;
#[cfg(test)]
use mech_core::Value;

impl MechRuntime {
  pub(super) fn with_live_registration_mode<T>(
    &mut self,
    mode: LiveRegistrationMode,
    f: impl FnOnce(&mut Self) -> MResult<T>,
  ) -> MResult<T> {
    let previous = std::mem::replace(&mut self.live_registration_mode, mode);
    let result = f(self);
    self.live_registration_mode = previous;
    result
  }

  #[cfg(test)]
  pub(in crate::runtime) fn run_string_with_isolated_registration_for_test(
    &mut self,
    context: &mut RuntimeContext,
    source: &str,
  ) -> MResult<Value> {
    self.with_live_registration_mode(
      LiveRegistrationMode::IsolatedSnapshot,
      |runtime| {
        runtime.run_string_value_with_context(context, source)
      },
    )
  }

  pub fn live_input_binding_count(&self) -> usize {
    self.live_input_bindings.values().map(Vec::len).sum()
  }

  pub fn has_live_input_bindings(&self) -> bool {
    self.live_input_binding_count() > 0
  }
}
