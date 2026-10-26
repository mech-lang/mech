//! Same validators, old fresh-inventory order versus one plan-bound inventory.
use super::*;
use crate::native_artifact::{
    guarded_histogram_query, histogram_sink, iteration_validation, string_element_map_projection,
    string_span_projection, text_join_sink,
};

fn expanded_then_counted(plan: &Plan) -> Result<ValidationInputs<'_, Plan>, &'static str> {
    // The old span/map validators expanded actions BEFORE streaming counts.
    let actions = semantic_actions(plan)?;
    let counts = semantic_use_counts(plan)?;
    let cached = OnceCell::new();
    assert!(cached.set(Ok(actions)).is_ok());
    Ok(ValidationInputs {
        inputs: SemanticInputs {
            plan,
            counts: OnceCell::from(Ok(counts)),
            actions: cached,
        },
    })
}

fn fresh(plan: &Plan) -> Result<(), &'static str> {
    // Keep the former construction/failure order exactly. The guarded-query
    // validator never used counts; its only inventory was actions then reads.
    text_join_sink::validate(&ValidationInputs::new(plan)?)?;
    guarded_histogram_query::validate(&ValidationInputs {
        inputs: SemanticInputs {
            plan,
            counts: OnceCell::from(Ok(Vec::new())),
            actions: OnceCell::new(),
        },
    })?;
    histogram_sink::validate(&ValidationInputs::new(plan)?)?;
    string_span_projection::validate(&expanded_then_counted(plan)?)?;
    string_element_map_projection::validate(&expanded_then_counted(plan)?)
}

fn same(plan: &Plan) -> Result<(), &'static str> {
    let reused = iteration_validation::validate_fusions(plan);
    assert_eq!(reused, fresh(plan));
    reused
}

#[test]
fn validation_inventory_reuse_matches_fresh_original_and_decoded_plans() {
    for source in SOURCES {
        for plan in plans(source) {
            same(&plan).unwrap();
            plan.validate_current_visits().unwrap();
            let inputs = ValidationInputs::new(&plan).unwrap();
            assert!(std::ptr::eq(inputs.plan(), &plan));
            assert_eq!(inputs.counts(), semantic_use_counts(&plan).unwrap());
            assert!(inputs.inputs.actions.get().is_none());
            text_join_sink::validate(&inputs).unwrap();
            assert!(inputs.inputs.actions.get().is_none());
            guarded_histogram_query::validate(&inputs).unwrap();
            assert!(inputs.inputs.actions.get().is_some());
            let first = inputs.actions().unwrap();
            assert!(std::ptr::eq(first, inputs.actions().unwrap()));
            assert_eq!(
                reads(&plan, first),
                reads(&plan, &semantic_actions(&plan).unwrap())
            );
        }
    }
}

#[test]
fn validation_inventory_reuse_rejects_malformed_and_omitted_virtual_phases() {
    for source in SOURCES {
        for plan in plans(source) {
            for (site, block) in plan.blocks().iter().enumerate() {
                if virtual_kind(block.action()).is_none() {
                    continue;
                }
                let mut omitted = plan.clone();
                omitted.blocks[site].action = Action::Scalar(Scalar::None);
                assert!(same(&omitted).is_err());
                assert!(omitted.validate_current_visits().is_err());
                let mut wrong = plan.clone();
                wrong.blocks[site].action = Action::HistogramQueryStep {
                    collection: u32::MAX,
                    step: u32::MAX,
                };
                assert_eq!(same(&wrong), Err(BAD));
                assert!(wrong.validate_current_visits().is_err());
            }
            let mut invalid_read = plan.clone();
            invalid_read.blocks[0].action = Action::Scalar(Scalar::Copy {
                source: Value(u32::MAX),
                output: Value(0),
            });
            assert_eq!(same(&invalid_read), Err(BAD));
            assert!(invalid_read.validate_current_visits().is_err());
        }
    }
}

#[test]
fn validation_inventory_reuse_keeps_cold_and_escaping_reads_visible() {
    for source in [SOURCES[2], SOURCES[4], SOURCES[5]] {
        for plan in plans(source) {
            let (id, collection) = plan
                .collections()
                .iter()
                .enumerate()
                .find(|(_, collection)| !matches!(collection.result(), ResultKind::Materialized))
                .unwrap();
            let value = collection.output();
            let mut escaped = plan.clone();
            let mut outputs = escaped.outputs.to_vec();
            let mut output = outputs[0].clone();
            output.value = value;
            output.ty = escaped.values[value.get() as usize].clone();
            outputs.push(output);
            escaped.outputs = outputs.into();
            assert!(same(&escaped).is_err(), "escaped collection {id}");
            assert!(escaped.validate_current_visits().is_err());

            let mut cold = plan.clone();
            let mut blocks = cold.blocks.to_vec();
            blocks.push(crate::native_artifact::NativeIterationBlock {
                origin: cold.blocks[collection.finish() as usize].origin(),
                action: Action::Scalar(Scalar::Copy {
                    source: value,
                    output: value,
                }),
                terminator: Terminator::Scalar(Term::Reject),
                iteration: collection.parent(),
            });
            cold.blocks = blocks.into();
            assert!(same(&cold).is_err(), "cold collection {id}");
            assert!(cold.validate_current_visits().is_err());
        }
    }
}

#[test]
fn validation_inventory_reuse_preserves_first_selected_diagnostic() {
    // The native tuple constructor accepts shallow atomic fields, not a Map.
    // Keep the full Histogram observable through an ordinary, unguarded key
    // read; the separate lookup still selects the guarded-query strategy.
    let source = SOURCES[5].replace(
        "lookup(source<[string]>, query<string>)",
        "items := source<[string]>\njoined := text/join([piece | piece <- items], \"\")\nhistogram := map/histogram([piece | piece <- items])\nmatched := lookup(items, query<string>)\n(joined, histogram{query}, matched)",
    );
    for plan in plans(&source) {
        same(&plan).unwrap();
        plan.validate_current_visits().unwrap();
        let join = plan
            .collections()
            .iter()
            .position(|collection| matches!(collection.result(), ResultKind::TextJoin(_)))
            .unwrap();
        let histogram = plan
            .collections()
            .iter()
            .position(|collection| matches!(collection.result(), ResultKind::Histogram(_)))
            .unwrap();
        let query = plan
            .collections()
            .iter()
            .position(|collection| {
                matches!(collection.result(), ResultKind::GuardedHistogramQuery(_))
            })
            .unwrap();

        let mut later_bad = plan.clone();
        let ResultKind::Histogram(sink) = &mut later_bad.collections[histogram].result else {
            unreachable!()
        };
        sink.consumer = u32::MAX;
        assert_eq!(
            same(&later_bad),
            Err("invalid selected native full Histogram sink")
        );

        let ResultKind::GuardedHistogramQuery(sink) = &mut later_bad.collections[query].result
        else {
            unreachable!()
        };
        sink.steps[0].site = u32::MAX;
        assert_eq!(
            same(&later_bad),
            Err("invalid native guarded histogram query demand")
        );

        let ResultKind::TextJoin(sink) = &mut later_bad.collections[join].result else {
            unreachable!()
        };
        sink.consumer = u32::MAX;
        assert_eq!(
            same(&later_bad),
            Err("invalid native selected TextJoin sink")
        );
        let inputs = ValidationInputs::new(&later_bad).unwrap();
        assert_eq!(
            text_join_sink::validate(&inputs),
            Err("invalid native selected TextJoin sink")
        );
        assert!(
            inputs.inputs.actions.get().is_none(),
            "later expansion must remain lazy after the earlier failure"
        );
    }
}
