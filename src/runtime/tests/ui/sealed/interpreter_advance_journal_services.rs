use mech_interpreter::Interpreter;

fn main() {
  let mut interpreter = Interpreter::new(1, 100);
  let _ =
    interpreter.advance_reactive_turn_with_journal_and_services();
}
