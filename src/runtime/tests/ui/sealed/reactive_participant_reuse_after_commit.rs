use mech_core::{
  with_reactive_journal_participant, Value,
};

fn main() {
  let _ = with_reactive_journal_participant(
    |mut participant| {
      participant.commit();
      participant.capture_value(&Value::Empty)?;
      Ok(())
    },
  );
}
