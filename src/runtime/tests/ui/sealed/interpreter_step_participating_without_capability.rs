use mech_core::{NoMechExecutionServices, Plan, ReactiveTurnState};

struct ForgedParticipant;

fn main() {
    let plan = Plan::new();
    let mut state = ReactiveTurnState::default();
    let mut services = NoMechExecutionServices;
    let mut forged = ForgedParticipant;
    drop(plan.advance_reactive_turn_participating(&mut state, &[], &mut forged, &mut services));
}
