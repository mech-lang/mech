use mech_core::with_reactive_journal_participant;

fn main() {
  let _escaped = with_reactive_journal_participant(
    |participant| Ok(participant),
  );
}
