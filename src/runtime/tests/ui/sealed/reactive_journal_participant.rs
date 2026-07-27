use mech_core::ReactiveJournalParticipant;

fn forge_participant<'journal>(
) -> ReactiveJournalParticipant<'journal> {
  ReactiveJournalParticipant {}
}

fn main() {
  let _ = forge_participant();
}
