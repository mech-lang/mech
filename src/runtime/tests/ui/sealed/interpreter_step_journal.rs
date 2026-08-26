use mech_engine::Interpreter;

fn main() {
  let mut interpreter = Interpreter::new(1, 100);
  drop(interpreter.step_with_reactive_turn_journal());
}
