use mech_engine::Interpreter;

fn main() {
  let mut interpreter = Interpreter::new(1, 100);
  drop(interpreter.advance_reactive_turn_with_journal_and_services());
}
