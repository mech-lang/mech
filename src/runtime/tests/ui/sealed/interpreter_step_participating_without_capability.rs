use mech_core::NoMechExecutionServices;
use mech_interpreter::Interpreter;

struct ForgedParticipant;

fn main() {
  let mut interpreter = Interpreter::new(1, 100);
  let mut services = NoMechExecutionServices;
  let mut forged = ForgedParticipant;
  let _ = interpreter.step_reactive_turn_participating(
    0,
    1,
    &mut forged,
    &mut services,
  );
}
