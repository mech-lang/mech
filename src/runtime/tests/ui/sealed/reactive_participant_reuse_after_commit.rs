use mech_core::{
  with_reactive_journal_participant, LegacyValue,
};

fn main() {
  drop(with_reactive_journal_participant(
    |mut participant| {
      participant.commit();
      participant.capture_value(&LegacyValue::Empty)?;
      Ok(())
    },
  ));
}
