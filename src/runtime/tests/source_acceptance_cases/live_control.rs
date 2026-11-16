//! Planning examples must not freeze control flow over explicitly live inputs.

use super::roundtrip;
use std::collections::{BTreeMap, BTreeSet};

use mech_core::{ReactiveInstanceId, ResidentValueRef, ValueData};
use mech_engine::ProgramArtifact;
use mech_engine::resident::{
    ActivationFacts, CapturedSignalInput, ReactiveInstance, ResidentExecutionError, activate,
};
use mech_runtime::{RuntimeBuilder, RuntimeHostInputValue, SourceDocument};
use mech_syntax::document::{ParseConfig, Revision};

fn planned(source: &str, initial: f64) -> ProgramArtifact {
    let document = SourceDocument::parse_resolved(
        "live-control.mec",
        Revision(0),
        source,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(
        document.is_strictly_clean(),
        "{source}: {:?}",
        document.snapshot().diagnostics.as_slice(),
    );
    RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .unwrap()
        .compile_document_artifact_with_inputs(
            &document,
            &BTreeMap::from([("input".to_owned(), RuntimeHostInputValue::F64(initial))]),
            &BTreeSet::from(["live".to_owned()]),
        )
        .unwrap_or_else(|error| panic!("planning input {initial}: {error:?}"))
        .into_artifact()
}

fn activated(artifact: &ProgramArtifact) -> ReactiveInstance {
    assert_eq!(artifact.inputs().len(), 1, "live input disappeared");
    activate(
        ReactiveInstanceId::new(0x4c495645, 0),
        artifact,
        &mech_stdlib::runtime_catalog(),
        &ActivationFacts::default(),
    )
    .unwrap()
}

fn try_run(instance: &mut ReactiveInstance, value: f64) -> Result<(), ResidentExecutionError> {
    assert_eq!(instance.plan.inputs.len(), 1);
    let slot = instance.plan.inputs[0].slot;
    instance
        .turn(&[CapturedSignalInput {
            slot,
            value: ResidentValueRef::F64(&[value]),
        }])
        .map(|_| ())
}

fn run(instance: &mut ReactiveInstance, value: f64) {
    try_run(instance, value).unwrap_or_else(|error| panic!("runtime input {value}: {error:?}"));
}

fn assert_output(instance: &ReactiveInstance, artifact: &ProgramArtifact, expected: f64) {
    let output = instance
        .copied_output(artifact.outputs().len() - 1)
        .unwrap();
    let ValueData::F64(actual) = output.data() else {
        panic!("expected scalar f64: {output:?}");
    };
    assert_eq!(actual.to_f64(), expected);
}

#[test]
fn external_input_function_match_changes_arms_after_planning() {
    let source = "live := input\nchoose(x<f64>) => <f64>\n  | 0.0 => 100.0\n  | x => x + 1.0.\nresult := choose(live)\nresult\n";
    for initial in [0.0, 1.0, 8.0] {
        let original = planned(source, initial);
        let decoded = roundtrip(&original);
        for artifact in [&original, &decoded] {
            let mut instance = activated(artifact);
            for value in [0.0, 1.0, 8.0, 0.0, -1.0, 2.0, 0.0] {
                run(&mut instance, value);
                assert_output(
                    &instance,
                    artifact,
                    if value == 0.0 { 100.0 } else { value + 1.0 },
                );
            }
        }
    }
}

#[test]
fn external_input_tail_recursion_uses_runtime_count_after_planning() {
    let source = "live := input\nsum-down(n<f64>, acc<f64>) => <f64>\n  | (0.0, acc) => acc\n  | (n, acc) => sum-down(n - 1.0, acc + n).\nresult := sum-down(live, 0.0)\nresult\n";
    // A negative example would not terminate if planning ran this live call.
    for initial in [-1.0, 0.0, 2.0, 8.0] {
        let original = planned(source, initial);
        let decoded = roundtrip(&original);
        for artifact in [&original, &decoded] {
            let mut instance = activated(artifact);
            for value in [0.0, 1.0, 2.0, 8.0, 128.0, 3.0, 0.0] {
                run(&mut instance, value);
                assert_output(&instance, artifact, value * (value + 1.0) / 2.0);
            }
        }
    }
}

#[test]
fn external_input_guards_and_nested_matches_use_current_values() {
    let source = "live := input\nchoose(x<f64>) => <f64>\n  | x => (x? | value, value < 0.0 => 100.0 - value | 0.0 => 1000.0 | value => ((value > 4.0)? | true => value * 2.0 | false => value + 1.0)).\nresult := choose(live)\nresult\n";
    for initial in [-2.0, 0.0, 2.0, 8.0] {
        let original = planned(source, initial);
        let decoded = roundtrip(&original);
        for artifact in [&original, &decoded] {
            let mut instance = activated(artifact);
            for value in [-2.0, 0.0, 2.0, 8.0, -1.0, 4.0, 5.0, 0.0] {
                let expected = if value < 0.0 {
                    100.0 - value
                } else if value == 0.0 {
                    1000.0
                } else if value > 4.0 {
                    value * 2.0
                } else {
                    value + 1.0
                };
                run(&mut instance, value);
                assert_output(&instance, artifact, expected);
            }
        }
    }
}

#[test]
fn external_input_does_not_evaluate_an_unselected_partial_function() {
    let source = "live := input\npartial(x<f64>) => <f64>\n  | 0.0 => 100.0.\nchoose(x<f64>) => <f64>\n  | 0.0 => partial(x)\n  | x => x + 1.0.\nresult := choose(live)\nresult\n";
    for initial in [0.0, 1.0, 8.0] {
        let original = planned(source, initial);
        let decoded = roundtrip(&original);
        for artifact in [&original, &decoded] {
            let mut instance = activated(artifact);
            for value in [1.0, 0.0, 8.0, -1.0, 0.0] {
                run(&mut instance, value);
                assert_output(
                    &instance,
                    artifact,
                    if value == 0.0 { 100.0 } else { value + 1.0 },
                );
            }
        }
    }
}

#[test]
fn external_input_failed_match_keeps_publication_and_recovers() {
    let source = "live := input\npartial(x<f64>) => <f64>\n  | 0.0 => 100.0\n  | 1.0 => 200.0.\nresult := partial(live)\nresult\n";
    // Even a planning example outside the function's domain must not execute
    // the live call or specialize away a valid runtime arm.
    for initial in [0.0, 1.0, 8.0] {
        let original = planned(source, initial);
        let decoded = roundtrip(&original);
        for artifact in [&original, &decoded] {
            let mut instance = activated(artifact);
            for (valid, expected) in [(0.0, 100.0), (1.0, 200.0), (0.0, 100.0)] {
                run(&mut instance, valid);
                assert_output(&instance, artifact, expected);
                let epoch = instance.published_epoch();
                assert!(try_run(&mut instance, 8.0).is_err());
                assert_eq!(instance.published_epoch(), epoch);
                assert_output(&instance, artifact, expected);
            }
            run(&mut instance, 1.0);
            assert_output(&instance, artifact, 200.0);
        }
    }
}
