use mech_program::{
  MechProgram, MechProgramConfig, ProgramReactiveTurnJournal,
};

fn main() {
  let mut program = MechProgram::new(MechProgramConfig::default());
  let mut journal = ProgramReactiveTurnJournal::default();
  let _ = program.step_with_reactive_turn_journal(&mut journal);
  let _ = program.advance_reactive_turn_with_journal(&mut journal);
  let _ = program.update_inputs_and_advance_turn_with_journal(
    &[],
    &mut journal,
  );
}
