use super::super::tests::plans;
use super::*;

pub(super) fn scan() -> String {
    "+> text\n+> sequence\n#Locate(rows<[string]>, count<u64>) => <(u64,u64)>\n  | :Skip(i<u64>)\n  | :Take(first<u64>, i<u64>)\n  | :Ready(first<u64>, last<u64>).\n#Locate(rows, count) -> :Skip(1u64)\n  :Skip(i)\n    | i > count -> :Ready(1u64, 1u64)\n    | rows[i] == \"open\" -> :Take(i + 1u64, i + 1u64)\n    | * -> :Skip(i + 1u64)\n  :Take(first, i)\n    | i > count -> :Ready(first, count + 1u64)\n    | rows[i] == \"close\" -> :Ready(first, i)\n    | * -> :Take(first, i + 1u64)\n  :Ready(first, last) => (first, last).\nrows := text/lines(input<string>)\nbounds := #Locate(rows, sequence/length(rows))\nresult := bounds?\n  | (first, last), first < last => text/join(rows[first..last], \"|\")\n  | * => \"\".\nresult\n".into()
}

fn describe(plan: &Plan) -> String {
    let read = plan.text_line_read_frontiers();
    let mut output = format!("reads: {read:?}\n");
    if let Some(read) = read.first() {
        let NativeControlLoop::Tail(tail) =
            &plan.iterations()[read.reads()[0].iteration() as usize]
        else {
            unreachable!()
        };
        let actions = semantic_actions(plan).unwrap();
        let defs = definitions(plan, &actions);
        let aggregates = plan.scalar_aggregates();
        let symbols = Symbols::new(
            plan,
            &actions,
            &defs,
            &aggregates,
            tail,
            read.count().counts(),
        );
        output.push_str(&format!(
            "seed {:?}\nnext {:?}\nreturned {:?}\n",
            symbols.alternatives(tail.seed()),
            symbols.alternatives(tail.next()),
            symbols.alternatives(tail.returned())
        ));
        for (site, block) in plan.blocks().iter().enumerate() {
            if let Some(action) = aggregates.action(site) {
                output.push_str(&format!("{site}: {action:?}\n"));
            }
            if let Action::LibraryCall {
                template,
                operands,
                output: value,
            } = block.action()
                && template.primitive() == Primitive::RangeExclusive
            {
                output.push_str(&format!(
                    "range {operands:?} -> {value:?}, words {:?}\n",
                    operands
                        .iter()
                        .map(|value| symbols.scalar(*value))
                        .collect::<Vec<_>>()
                ));
            }
        }
    }
    output
}

#[test]
fn original_and_decoded_scans_keep_only_the_current_or_future_range_start() {
    for source in [
        scan(),
        scan().replace("first < last", "last > first"),
        scan().replace("i + 1u64", "i + 2u64"),
        scan()
            .replace("#Locate", "#Search")
            .replace(":Skip", ":Before")
            .replace(":Take", ":Inside")
            .replace(":Ready", ":Finished")
            .replace("\"open\"", "\"alpha\"")
            .replace("\"close\"", "\"omega\""),
    ] {
        for plan in plans(&source) {
            let frontiers = plan.text_line_retention_frontiers();
            assert_eq!(frontiers.len(), 1, "{}", describe(&plan));
            let frontier = &frontiers[0];
            assert_eq!(frontier.states().len(), 3);
            assert_eq!(frontier.result_fields(), (0, 1));
            let states = frontier.states();
            assert_eq!(states[0].index_field(), Some(1));
            assert_eq!(states[0].keep_fields(), [1]);
            assert_eq!(states[1].index_field(), Some(2));
            assert_eq!(states[1].keep_fields(), [1, 2]);
            assert_eq!(states[2].returned_range(), Some((1, 2)));
            assert!(states[2].keep_fields().is_empty());
        }
    }
}

#[test]
fn ordinal_arithmetic_is_checked_and_never_combines_two_dynamic_words() {
    let current = Word {
        atom: Some(Atom::State(1, 2)),
        add: 0,
    };
    assert_eq!(
        current.plus(Word::constant(7)).unwrap(),
        Word { add: 7, ..current }
    );
    assert!(current.plus(current).is_none());
    assert!(Word::constant(u64::MAX).plus(Word::constant(1)).is_none());
    assert!(
        Word {
            add: u64::MAX,
            ..current
        }
        .plus(Word::constant(1))
        .is_none()
    );
}

#[test]
fn backwards_or_restarting_indices_cannot_discard_a_prefix() {
    for source in [
        scan().replace(":Skip(i + 1u64)", ":Skip(i - 1u64)"),
        scan().replace(":Skip(i + 1u64)", ":Skip(1u64)"),
        scan().replace(":Take(first, i + 1u64)", ":Take(first, i - 1u64)"),
        scan().replace(":Skip(1u64)", ":Skip(0u64)"),
    ] {
        for plan in plans(&source) {
            assert_eq!(plan.text_line_read_frontiers().len(), 1, "{source}");
            assert!(plan.text_line_retention_frontiers().is_empty(), "{source}");
        }
    }
}

#[test]
fn a_future_range_reaching_into_discarded_history_stays_on_snapshot_execution() {
    for source in [
        scan().replace(":Take(i + 1u64, i + 1u64)", ":Take(1u64, i + 1u64)"),
        scan().replace(":Ready(first, i)", ":Ready(1u64, i)"),
        scan().replace(":Take(first, i + 1u64)", ":Take(1u64, i + 1u64)"),
    ] {
        for plan in plans(&source) {
            assert_eq!(plan.text_line_read_frontiers().len(), 1, "{source}");
            assert!(plan.text_line_retention_frontiers().is_empty(), "{source}");
        }
    }
}

#[test]
fn an_empty_return_must_not_revive_an_old_prefix_without_its_original_guard() {
    let source = scan().replace(", first < last =>", ", first <= last =>");
    for plan in plans(&source) {
        assert_eq!(plan.text_line_read_frontiers().len(), 1);
        assert!(plan.text_line_retention_frontiers().is_empty());
    }
}
