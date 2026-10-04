use mech_core::ReactivePlan;

fn main() {
    let mut plan = ReactivePlan::new();
    drop(plan.advance_reactive_turn_with_journal_and_services());
}
