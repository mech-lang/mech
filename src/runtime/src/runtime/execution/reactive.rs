use super::*;

impl MechRuntime {
  #[cfg(feature = "functions")]
  pub fn step(&mut self, step_id: u64) -> MResult<()> {
    let mut context = self.runtime_context()?;
    self.step_with_context(&mut context, step_id)
  }

  #[cfg(feature = "functions")]
  pub fn step_with_context(
    &mut self,
    context: &mut RuntimeContext,
    step_id: u64,
  ) -> MResult<()> {
    let turn_started = Instant::now();
    self.with_atomic_reactive_turn(
      context,
      "step_with_context",
      |program, services, finalize| {
        program.step_coordinated(
          step_id,
          services,
          || finalize(&()),
        )
      },
      move |runtime, _context, _| {
        runtime.enforce_turn_duration(turn_started)
      },
    )
  }
}
