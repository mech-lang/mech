//! Exact source/decoded acceptance for recovery finding G17's fixed-population
//! logical-mask activation facts.
#![cfg(all(feature = "full_source", feature = "resident-routing-source"))]

use mech_core::{ReactiveInstanceId, ResidentValueRef, SchemaBody};
use mech_engine::ProgramArtifact;
use mech_engine::resident::{
    ActivationFacts, CapturedSignalInput, ResidentActivationError, activate,
};
use mech_runtime::{RuntimeBuilder, RuntimeValueSnapshot, SourceDocument};
use mech_syntax::document::{ParseConfig, Revision};

fn compile(source: &str) -> ProgramArtifact {
    let document = SourceDocument::parse_resolved(
        "s8-recovery-logical-activation-facts.mec",
        Revision(0),
        source,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(document.is_strictly_clean());
    RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .unwrap()
        .compile_document(&document)
        .unwrap()
        .artifact()
        .clone()
}

fn roundtrip(artifact: &ProgramArtifact) -> ProgramArtifact {
    mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(artifact).unwrap(),
    )
    .unwrap()
}

fn output(instance: &mech_engine::resident::ReactiveInstance) -> String {
    RuntimeValueSnapshot::from_value(instance.copied_output(0).unwrap())
        .unwrap()
        .format_canonical_inline()
}

fn exact_closed_mask(source: &str, expected: &str) {
    let artifact = compile(source);
    let decoded = roundtrip(&artifact);
    let catalog = mech_stdlib::source_catalog();
    for artifact in [&artifact, &decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x58c, 6),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        for _ in 0..2 {
            instance.turn(&[]).unwrap();
            assert_eq!(output(&instance), expected);
        }
    }
}

#[test]
fn closed_literal_and_computed_masks_publish_exact_source_and_decoded_shapes() {
    exact_closed_mask("x := [1 2 3]\nx[[false true true]]\n", "[2; 3]");
    exact_closed_mask("x := [1 2 3]\nmask := x > 1\nx[mask]\n", "[2; 3]");
}

#[test]
fn closed_comparison_masks_share_broadcast_and_ordering_semantics() {
    for (comparison, expected) in [
        ("x > 1", "[2; 3]"),
        ("x >= 2", "[2; 3]"),
        ("x < 3", "[1; 2]"),
        ("x <= 2", "[1; 2]"),
        ("x == 2", "[2]"),
        ("x != 2", "[1; 3]"),
        ("1 < x", "[2; 3]"),
    ] {
        exact_closed_mask(
            &format!("x := [1 2 3]\nmask := {comparison}\nx[mask]\n"),
            expected,
        );
    }
    exact_closed_mask("x := [1 2; 3 4]\nmask := x >= 3\nx[mask]\n", "[3; 4]");
    exact_closed_mask("x := [42]\nmask := 1 < 2\nx[mask]\n", "[42]");
    exact_closed_mask(
        "x := [1<f32> 2<f32>]\nmask := x > 1<f64>\nx[mask]\n",
        "[2<f32>]",
    );
}

#[test]
fn closed_whole_value_comparisons_have_scalar_populations() {
    exact_closed_mask("x := [42]\np := [1 2] !== [1; 2]\nx[p]\n", "[42]");
    exact_closed_mask(
        "x := [42 43]\np := [1 2] === [1 2]\nmask := [p false]\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask(
        "x := [42 43]\np := [1 2] !== [1 3]\nmask := [p false]\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask(
        "x := [42 43]\np := [:Point :Point] == [:Point :Point]\nmask := [p false]\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask(
        "x := [42 43]\np := [-0.0] === [0.0]\nmask := [p false]\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask(
        "x := [42 43]\np := (-0.0, 1) == (0.0, 1)\nmask := [p true]\nx[mask]\n",
        "[43]",
    );
}

#[test]
fn live_masks_still_require_one_explicit_fixed_population_fact() {
    let artifact = compile("x := [1 2 3]\nx[mask<[bool]:1,3>]\n");
    let decoded = roundtrip(&artifact);
    let catalog = mech_stdlib::source_catalog();
    for artifact in [&artifact, &decoded] {
        let selected = artifact
            .slots()
            .iter()
            .find(|slot| {
                artifact.schemas().get(slot.schema).is_some_and(|schema| {
                    matches!(schema.body(), SchemaBody::Matrix { .. })
                        && !schema.dimension_parameters().is_empty()
                })
            })
            .unwrap();
        assert!(matches!(
            activate(
                ReactiveInstanceId::new(0x58c, 7),
                artifact,
                &catalog,
                &ActivationFacts::default(),
            ),
            Err(ResidentActivationError::UnresolvedShape { slot }) if slot == selected.slot
        ));

        let mut facts = ActivationFacts::default();
        facts.slot_shapes.insert(
            selected.slot,
            artifact
                .schemas()
                .get(selected.schema)
                .unwrap()
                .instantiate_shape(Box::new([2]))
                .unwrap(),
        );
        let mut instance = activate(
            ReactiveInstanceId::new(0x58c, 8),
            artifact,
            &catalog,
            &facts,
        )
        .unwrap();
        let mask_slot = instance
            .plan
            .inputs
            .iter()
            .find(|input| {
                artifact.inputs().iter().any(|declaration| {
                    declaration.name == mech_engine::encode_source_input_name("mask")
                        && declaration.slot == input.artifact_slot
                })
            })
            .unwrap()
            .slot;
        for (mask, expected) in [([0_u8, 1, 1], "[2; 3]"), ([1, 0, 1], "[1; 3]")] {
            instance
                .turn(&[CapturedSignalInput {
                    slot: mask_slot,
                    value: ResidentValueRef::Bool(&mask),
                }])
                .unwrap();
            assert_eq!(output(&instance), expected);
        }
        let previous = output(&instance);
        for mask in [[1_u8, 0, 0], [1, 1, 1], [2, 0, 0]] {
            assert!(
                instance
                    .turn(&[CapturedSignalInput {
                        slot: mask_slot,
                        value: ResidentValueRef::Bool(&mask),
                    }])
                    .is_err()
            );
            assert_eq!(output(&instance), previous);
        }
    }
}
