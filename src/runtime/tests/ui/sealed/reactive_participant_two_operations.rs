use mech_core::{
    NoMechExecutionServices, Plan, ReactiveTurnState, with_reactive_journal_participant,
};

fn main() {
    let plan = Plan::new();
    let mut state = ReactiveTurnState::default();
    let mut services = NoMechExecutionServices;
    drop(with_reactive_journal_participant(|mut participant| {
        drop(plan.advance_reactive_turn_participating(
            &mut state,
            &[],
            &mut participant,
            &mut services,
        ));
        participant.commit();
        drop(plan.advance_reactive_turn_participating(
            &mut state,
            &[],
            &mut participant,
            &mut services,
        ));
        Ok(())
    }));
}
