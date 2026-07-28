use super::*;

impl MechRuntime {
  pub fn persistent_send_count(&self) -> usize {
    self.persistent_sends.len()
  }

  pub(super) fn execute_persistent_sends(&mut self, context: &mut RuntimeContext, turn: &mech_program::ProgramInputTurnOutcome) -> MResult<()> {
    for send in self.persistent_sends.clone() {
      let should_send = match send.schedule {
        RuntimePersistentSendSchedule::EveryAcceptedTurn => true,
        RuntimePersistentSendSchedule::Activation { interpreter_id, barrier_node_id } => turn.interpreter_turns.iter().find(|outcome| outcome.interpreter_id == interpreter_id).map(|outcome| {
          outcome.turn.before_commit.executed_nodes.contains(&barrier_node_id) || outcome.turn.after_commit.executed_nodes.contains(&barrier_node_id)
        }).unwrap_or(false),
      };
      if !should_send { continue; }
      let value = resolve_runtime_value(send.value.borrow().clone());
      self.write_context_resource(context, &send.binding, &send.path, value, RuntimeResourceWriteIntent::Send)?;
    }
    Ok(())
  }
}
