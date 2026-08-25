use mech_core::{
  with_reactive_journal_participant,
  NoMechExecutionServices,
};
use mech_engine::Interpreter;

fn main() {
  let mut interpreter = Interpreter::new(1, 100);
  let mut services = NoMechExecutionServices;
  drop(with_reactive_journal_participant(
    |mut participant| {
      drop(interpreter.step_reactive_turn_participating(
        0,
        1,
        &mut participant,
        &mut services,
      ));
      participant.commit();
      drop(interpreter.step_reactive_turn_participating(
        0,
        1,
        &mut participant,
        &mut services,
      ));
      Ok(())
    },
  ));
}
