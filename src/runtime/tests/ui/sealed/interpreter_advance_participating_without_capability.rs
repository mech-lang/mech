use mech_core::NoMechExecutionServices;
use mech_engine::Interpreter;

struct ForgedParticipant;

fn main() {
  let mut interpreter = Interpreter::new(1, 100);
  let mut services = NoMechExecutionServices;
  let mut forged = ForgedParticipant;
  let _ = interpreter.advance_reactive_turn_participating(
    &[],
    &mut forged,
    &mut services,
  );
}
