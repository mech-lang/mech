use mech_core::NoMechExecutionServices;
use mech_engine::Interpreter;

struct ForgedParticipant;

fn main() {
  let mut interpreter = Interpreter::new(1, 100);
  let mut services = NoMechExecutionServices;
  let mut forged = ForgedParticipant;
  drop(interpreter.step_reactive_turn_participating(
    0,
    1,
    &mut forged,
    &mut services,
  ));
}
