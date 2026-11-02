use mech_core::{ValueCell, with_reactive_journal_participant};

fn main() {
  let cell = ValueCell::unit();
  drop(with_reactive_journal_participant(
    |mut participant| {
      participant.commit();
      participant.capture_value_cell(&cell)?;
      Ok(())
    },
  ));
}
