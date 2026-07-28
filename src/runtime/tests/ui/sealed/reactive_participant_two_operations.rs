use mech_core::{
  with_reactive_journal_participant,
  NoMechExecutionServices,
};
use mech_interpreter::Interpreter;

fn main() {
  let mut interpreter = Interpreter::new(1, 100);
  let mut services = NoMechExecutionServices;
  let _ = with_reactive_journal_participant(
    |mut participant| {
      let _ = interpreter.step_reactive_turn_participating(
        0,
        1,
        &mut participant,
        &mut services,
      );
      participant.commit();
      let _ = interpreter.step_reactive_turn_participating(
        0,
        1,
        &mut participant,
        &mut services,
      );
      Ok(())
    },
  );
}
