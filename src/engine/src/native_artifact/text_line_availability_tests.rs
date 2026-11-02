use super::super::tests::{plans, scan};
use super::*;
use crate::native_artifact::fusion_semantics::semantic_actions;

#[test]
fn each_original_and_decoded_read_is_guarded_in_its_current_visit() {
    for predicate in ["i > n", "n < i"] {
        for plan in plans(&scan(predicate, "n + 1u64", "")) {
            let frontiers = plan.text_line_read_frontiers();
            assert_eq!(frontiers.len(), 1);
            let frontier = &frontiers[0];
            let actions = semantic_actions(&plan).unwrap();
            let defs = definitions(&plan, &actions);
            let aggregates = plan.scalar_aggregates();
            assert_eq!(frontier.reads().len(), 1);
            for read in frontier.reads() {
                assert!(!read.guards().is_empty());
                for &ordinal in read.guards() {
                    let guard = &frontier.count().guards()[ordinal as usize];
                    assert_eq!(
                        index_identity(&defs, &aggregates, plan.blocks().len(), guard.index()),
                        index_identity(&defs, &aggregates, plan.blocks().len(), read.index())
                    );
                    assert_eq!(
                        plan.blocks()[guard.branch() as usize].iteration(),
                        Some(read.iteration())
                    );
                }
            }
        }
    }
}

#[test]
fn guarding_a_smaller_index_does_not_prove_a_later_line_available() {
    let source = scan("i > n", "n + 1u64", "").replace("lines[i]", "lines[i + 1u64]");
    for plan in plans(&source) {
        assert_eq!(plan.text_line_count_frontiers().len(), 1);
        assert!(plan.text_line_read_frontiers().is_empty());
    }
}

#[test]
fn a_different_state_test_cannot_hide_a_real_unguarded_path() {
    for mut plan in plans(&scan("i > n", "n + 1u64", "")) {
        let read = plan.text_line_read_frontiers().pop().unwrap();
        let iteration = read.reads()[0].iteration();
        let NativeControlLoop::Tail(tail) = &plan.iterations()[iteration as usize] else {
            unreachable!()
        };
        let carrier = tail.carrier();
        let test = plan
            .blocks
            .iter_mut()
            .find_map(|block| match &mut block.action {
                super::super::Action::Structural {
                    source,
                    path,
                    observation:
                        super::super::super::NativeStructuralObservation::EnumOrdinal { ordinal },
                    ..
                } if *source == carrier && path.is_empty() && *ordinal == 0 => Some(ordinal),
                _ => None,
            })
            .unwrap();
        // The first arm now tests Done, while the following arm still tests
        // Step. A Step read can genuinely bypass the count guard: this is not
        // the contradictory repeated-state path of the unmodified plan.
        *test = 1;
        plan.validate_current_visits().unwrap();
        assert_eq!(plan.text_line_count_frontiers().len(), 1);
        assert!(plan.text_line_read_frontiers().is_empty());
    }
}

#[test]
fn a_read_before_its_guard_cannot_use_a_later_successful_check() {
    let source = scan("i > n", "n + 1u64", "").replace(
        "| i > n -> :Done(n + 1u64)\n    | lines[i] == \"stop\" -> :Done(i)",
        "| lines[i] == \"stop\" -> :Done(i)\n    | i > n -> :Done(n + 1u64)",
    );
    for plan in plans(&source) {
        assert_eq!(plan.text_line_count_frontiers().len(), 1);
        assert!(plan.text_line_read_frontiers().is_empty());
    }
}

#[test]
fn a_check_in_an_earlier_state_visit_does_not_protect_a_later_visit() {
    let source = "+> text\n+> sequence\n#Walk(lines<[string]>, n<u64>) => <u64>\n  | :Check(i<u64>)\n  | :Read(i<u64>)\n  | :Done(out<u64>).\n#Walk(lines, n) -> :Check(1u64)\n  :Check(i)\n    | i > n -> :Done(n + 1u64)\n    | * -> :Read(i + 1u64)\n  :Read(i)\n    | lines[i] == \"stop\" -> :Done(i)\n    | * -> :Check(i + 1u64)\n  :Done(out) => out.\nlines := text/lines(input<string>)\n#Walk(lines, sequence/length(lines))\n";
    for plan in plans(source) {
        let count = plan.text_line_count_frontiers().pop().unwrap();
        let region = &plan.borrowed_string_regions()[0];
        let NativeControlLoop::Tail(tail) = &plan.iterations()[region.iteration() as usize] else {
            unreachable!()
        };
        let cut = count
            .guards()
            .iter()
            .map(|guard| (guard.branch(), guard.available()))
            .collect::<Vec<_>>();
        let seen = traversal(&plan, region.iteration(), tail, &cut).unwrap();
        let reader = region
            .actions()
            .iter()
            .find_map(|(site, action)| {
                matches!(action, Borrowed::MatrixElement { .. }).then_some(*site)
            })
            .unwrap();
        assert!(
            seen[reader as usize],
            "the later state is reachable without any check in this traversal"
        );
        assert!(plan.text_line_read_frontiers().is_empty());
    }
}

#[test]
fn prefix_availability_does_not_claim_monotonic_or_nonzero_indices() {
    for source in [
        scan("i > n", "n + 1u64", "").replace(":Step(1u64)", ":Step(0u64)"),
        scan("i > n", "n + 1u64", "").replace(":Step(i + 1u64)", ":Step(i - 1u64)"),
    ] {
        for plan in plans(&source) {
            assert_eq!(plan.text_line_read_frontiers().len(), 1);
        }
    }
}
