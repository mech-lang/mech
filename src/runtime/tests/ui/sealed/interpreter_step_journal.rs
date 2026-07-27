use mech_interpreter::Interpreter;

fn main() {
  let mut interpreter = Interpreter::new(1, 100);
  let _ = interpreter.step_with_reactive_turn_journal();
}
