//! Exercise the exact pure compiler state without the GPU package's full-catalog tests.
#[path = "../../../hosts/gpu/src/native_artifact/codegen/common/boolean_predecessors.rs"]
mod boolean_predecessors;
#[path = "../../../hosts/gpu/src/native_artifact/program/compact/owner_reads.rs"]
mod owner_reads;
#[path = "../../../hosts/gpu/src/native_artifact/program/codegen/cfg_cleanup/route_uses/reach.rs"]
mod route_reach;
#[path = "../../../hosts/gpu/src/native_artifact/program/compact/guard_state.rs"]
mod state;

#[test]
fn contradictions_require_a_stable_boolean_home() {
    let mut first = state::State::default();
    let mut second = state::State::default();
    for (home, left, right) in [(0, false, true), (35, false, false), (65, true, false)] {
        first.assume(home, left);
        second.assume(home, right);
    }

    assert!(first.contradicts_where(&second, |home| home == 0));
    assert!(first.contradicts_where(&second, |home| home == 65));
    assert!(!first.contradicts_where(&second, |home| home == 35));
    assert!(!first.contradicts_where(&second, |_| false));
}
