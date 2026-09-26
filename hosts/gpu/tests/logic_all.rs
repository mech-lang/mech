#![cfg(feature = "embedding")]

use std::collections::{BTreeMap, BTreeSet};

use mech_core::{CellSlotId, ReactiveInstanceId, ResidentValueRef};
use mech_engine::__resident::{
    ActivationFacts, CapturedSignalInput, ResidentExecutionError, ResidentValueBorrow, activate,
};
use mech_engine::{
    ProgramArtifact, decode_program_artifact_sections, encode_program_artifact_sections,
};
use mech_gpu::{BatchedExecutionError, ComputeLowerer, FixedShapeKernel};
use mech_runtime::{RuntimeBuilder, RuntimeHostInputValue};

const VARIANCE_SOURCE: &str = r#"
+> math/*, logic/all

Variance @compute
-------------------------------------------------------------------------------
covariance := [2f32 -1f32 -2f32; -3f32 2f32 -4f32; -5f32 -6f32 2f32]
~state := 0f32
Σ₊ := covariance - state
variance! := all(Σ₊[[1 5 9]] > 0f32)
state = state + 1f32
state
"#;

fn evaluate(source: &str, names: &[&str]) -> BTreeMap<String, RuntimeHostInputValue> {
    RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap()
        .evaluate_static_tree_symbols(&mech_syntax::parse(source).unwrap(), names)
        .unwrap()
}

fn compile(source: &str, input: &str) -> ProgramArtifact {
    RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap()
        .compile_tree_artifact_with_interface(
            &mech_syntax::parse(source).unwrap(),
            &BTreeMap::new(),
            &BTreeSet::from([input.to_owned()]),
            &["state"],
        )
        .unwrap()
        .into_artifact()
}

#[test]
fn source_all_reduces_scalar_vectors_and_matrices_to_one_boolean() {
    let combined = evaluate(
        "+> math/*, logic/all\nresult := all(sin(0f32) == 0f32)\nresult",
        &["result"],
    );
    assert_eq!(combined["result"], RuntimeHostInputValue::Bool(true));
    for literal in ["true", "false"] {
        let result = evaluate(
            &format!("+> logic/all\nresult := all({literal})\nresult"),
            &["result"],
        );
        assert_eq!(
            result["result"],
            RuntimeHostInputValue::Bool(literal == "true")
        );
    }
    for (rows, columns) in [(1, 9), (9, 1), (2, 3)] {
        for false_position in 0..=rows * columns {
            let values = (0..rows)
                .map(|row| {
                    (0..columns)
                        .map(|column| {
                            if row * columns + column == false_position {
                                "false"
                            } else {
                                "true"
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect::<Vec<_>>()
                .join("; ");
            let result = evaluate(
                &format!("+> logic/all\nresult := all([{values}])\nresult"),
                &["result"],
            );
            assert_eq!(
                result["result"],
                RuntimeHostInputValue::Bool(false_position == rows * columns),
                "{rows}x{columns}, false at {false_position}"
            );
        }
    }
}

#[test]
fn linear_indices_one_five_nine_select_the_covariance_diagonal() {
    let outputs = evaluate(
        "+> logic/all\nΣ₊ := [1f32 -2f32 -3f32; -4f32 5f32 -6f32; -7f32 -8f32 9f32]\ndiagonal := Σ₊[[1 5 9]]\npositive := all(diagonal > 0f32)\n(diagonal, positive)",
        &["diagonal", "positive"],
    );
    let RuntimeHostInputValue::F32Matrix { values, .. } = &outputs["diagonal"] else {
        panic!("linear diagonal selection must retain matrix elements");
    };
    assert_eq!(values, &[1.0, 5.0, 9.0]);
    assert_eq!(outputs["positive"], RuntimeHostInputValue::Bool(true));
}

#[test]
fn source_all_rejects_non_boolean_arguments() {
    for literal in ["1f32", "[1f32 0f32]", "[1 0; 1 1]", "\"true\""] {
        let tree =
            mech_syntax::parse(&format!("+> logic/all\nresult := all({literal})\nresult")).unwrap();
        let result = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_native_plan_catalog())
            .build_compiler()
            .unwrap()
            .evaluate_static_tree_symbols(&tree, &["result"]);
        assert!(result.is_err(), "all must not coerce {literal} to Boolean");
    }
}

#[test]
fn ordinary_resident_accepts_compact_f64_covariance_guard_and_retains_state_on_failure() {
    let source = VARIANCE_SOURCE
        .replace("Variance @compute\n-------------------------------------------------------------------------------\n", "")
        .replace("f32", "");
    assert!(source.contains("variance! := all(Σ₊[[1 5 9]] > 0)"));
    let artifact = compile(&source, "covariance");
    let state_slot = artifact
        .interactive_symbol_bindings()
        .find(|binding| binding.lexical_name == "state")
        .unwrap()
        .storage;
    for position in [0, 4, 8] {
        let mut instance = activate(
            ReactiveInstanceId::new(0, 0),
            &artifact,
            &mech_stdlib::source_native_plan_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        let input_slot = instance.plan.inputs[0].slot;
        let mut covariance = [-10.0; 9];
        for index in [0, 4, 8] {
            covariance[index] = 2.0;
        }
        covariance[position] = 1.0;
        instance
            .turn_without_summary(&[CapturedSignalInput {
                slot: input_slot,
                value: ResidentValueRef::F64(&covariance),
            }])
            .unwrap();
        let epoch = instance.published_epoch();
        assert!(matches!(
            instance.turn_without_summary(&[CapturedSignalInput {
                slot: input_slot,
                value: ResidentValueRef::F64(&covariance),
            }]),
            Err(ResidentExecutionError::Integrity { .. })
        ));
        let ResidentValueBorrow::F64 { values, .. } = instance.state_borrow(state_slot).unwrap()
        else {
            panic!("counter must remain f64")
        };
        assert_eq!(values, &[1.0]);
        assert_eq!(instance.published_epoch(), epoch);
    }
}

#[test]
fn ordinary_resident_all_rejection_preserves_state_and_recovers() {
    for (literal, count) in [
        ("true", 1),
        ("[true true true]", 3),
        ("[true; true; true]", 3),
        ("[true true; true true]", 4),
    ] {
        let source = format!(
            "+> logic/all\ninput := {literal}\n~state := 0\nvalid! := all(input)\nstate = state + 1\nstate"
        );
        let artifact = compile(&source, "input");
        let state_slot = artifact
            .interactive_symbol_bindings()
            .find(|binding| binding.lexical_name == "state")
            .unwrap()
            .storage;
        let mut instance = activate(
            ReactiveInstanceId::new(0, 0),
            &artifact,
            &mech_stdlib::source_native_plan_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        let input_slot = instance.plan.inputs[0].slot;
        let good = vec![1u8; count];
        instance
            .turn_without_summary(&[CapturedSignalInput {
                slot: input_slot,
                value: ResidentValueRef::Bool(&good),
            }])
            .unwrap();
        let accepted_epoch = instance.published_epoch();
        for false_position in 0..count {
            let mut bad = good.clone();
            bad[false_position] = 0;
            assert!(matches!(
                instance.turn_without_summary(&[CapturedSignalInput {
                    slot: input_slot,
                    value: ResidentValueRef::Bool(&bad)
                }]),
                Err(ResidentExecutionError::Integrity { .. })
            ));
            let ResidentValueBorrow::F64 { values, .. } =
                instance.state_borrow(state_slot).unwrap()
            else {
                panic!("counter state must be f64")
            };
            assert_eq!(values, &[1.0], "{literal}: false at {false_position}");
            assert_eq!(instance.published_epoch(), accepted_epoch);
        }
        instance
            .turn_without_summary(&[CapturedSignalInput {
                slot: input_slot,
                value: ResidentValueRef::Bool(&good),
            }])
            .unwrap();
        let ResidentValueBorrow::F64 { values, .. } = instance.state_borrow(state_slot).unwrap()
        else {
            panic!("counter state must be f64")
        };
        assert_eq!(values, &[2.0]);
    }
}

fn compute_case(selection: &str) -> (FixedShapeKernel, CellSlotId) {
    let source = VARIANCE_SOURCE.replace("Σ₊[[1 5 9]] > 0f32", selection);
    let artifact = compile(&source, "covariance");
    let encoded = encode_program_artifact_sections(&artifact).unwrap();
    let decoded = decode_program_artifact_sections(&encoded).unwrap();
    let inputs = case_inputs(None, false);
    let program = ComputeLowerer
        .compile_broadcast(&artifact, &inputs)
        .unwrap();
    let restored = ComputeLowerer.compile_broadcast(&decoded, &inputs).unwrap();
    assert_eq!(program.wgsl(), restored.wgsl());
    let slot = artifact
        .interactive_symbol_bindings()
        .find(|binding| binding.lexical_name == "state")
        .unwrap()
        .storage;
    (program, slot)
}

fn case_inputs(false_position: Option<usize>, diagonal: bool) -> BTreeMap<String, Vec<f32>> {
    let mut values = vec![2f32; 8 * 9];
    if diagonal {
        for lane in 0..8 {
            for index in [1, 2, 3, 5, 6, 7] {
                values[lane * 9 + index] = -10.0;
            }
        }
    }
    if let Some(position) = false_position {
        values[7 * 9 + position] = 1.0;
    }
    BTreeMap::from([("covariance".to_owned(), values)])
}

fn assert_variance_fault(error: BatchedExecutionError) {
    let BatchedExecutionError::Integrity(fault) = error else {
        panic!("expected integrity rejection, found {error}")
    };
    assert_eq!(fault.constraint_name.as_ref(), "variance!");
    assert_eq!(fault.instance, 7);
    assert_eq!(fault.attempted_turn, 2);
}

#[test]
fn nonsquare_comparison_preserves_constant_element_order() {
    let source = r#"
+> logic/all

Thresholds @compute
-------------------------------------------------------------------------------
input := [2f32 3f32 4f32; 5f32 6f32 7f32]
~state := 0f32
candidate := input - state
variance! := all(candidate > [1f32 2f32 3f32; 4f32 5f32 6f32])
state = state + 1f32
state
"#;
    let artifact = compile(source, "input");
    // The low-level broadcast interface stores each lane in column-major order.
    let inputs = BTreeMap::from([("input".to_owned(), vec![2.0, 5.0, 3.0, 6.0, 4.0, 7.0])]);
    let program = ComputeLowerer
        .compile_broadcast(&artifact, &inputs)
        .unwrap();
    let state_slot = artifact
        .interactive_symbol_bindings()
        .find(|binding| binding.lexical_name == "state")
        .unwrap()
        .storage;
    macro_rules! verify {
        ($session:expr) => {{
            let mut session = $session.unwrap();
            session.dispatch_turns(1).unwrap();
            assert_eq!(session.state()[&state_slot], vec![1.0]);
            let BatchedExecutionError::Integrity(fault) = session.dispatch_turns(1).unwrap_err()
            else {
                panic!("expected second-turn threshold rejection")
            };
            assert_eq!(fault.instance, 0);
            assert_eq!(fault.constraint_name.as_ref(), "variance!");
            assert_eq!(fault.attempted_turn, 2);
            assert_eq!(session.state()[&state_slot], vec![1.0]);
        }};
    }
    verify!(program.prepare_cpu(&inputs));
    verify!(program.prepare_simd_cpu(&inputs));
}

fn selections() -> Vec<(&'static str, Vec<usize>, bool)> {
    vec![
        ("Σ₊[1] > 0f32", vec![0], true),
        ("Σ₊[[1 5 9]] > 0f32", vec![0, 4, 8], true),
        ("0f32 < Σ₊[[1 5 9]]", vec![0, 4, 8], true),
        ("Σ₊ > 0f32", (0..9).collect(), false),
        (
            "Σ₊ > [0f32 0f32 0f32; 0f32 0f32 0f32; 0f32 0f32 0f32]",
            (0..9).collect(),
            false,
        ),
    ]
}

#[test]
fn all_compute_cpu_backends_reject_each_false_element_without_publication() {
    for (selection, positions, diagonal) in selections() {
        let (program, state_slot) = compute_case(selection);
        for position in positions {
            let inputs = case_inputs(Some(position), diagonal);
            macro_rules! verify {
                ($session:expr) => {{
                    let mut session = $session.unwrap();
                    session.dispatch_turns(1).unwrap();
                    assert_eq!(session.state()[&state_slot], vec![1.0; 8]);
                    assert_variance_fault(session.dispatch_turns(1).unwrap_err());
                    assert_eq!(session.state()[&state_slot], vec![1.0; 8]);
                }};
            }
            verify!(program.prepare_cpu(&inputs));
            verify!(program.prepare_simd_cpu(&inputs));
            #[cfg(feature = "jit")]
            {
                verify!(program.prepare_jit_cpu(&inputs));
                verify!(program.prepare_jit_simd_cpu(&inputs));
            }
            #[cfg(feature = "aot")]
            {
                verify!(program.prepare_aot_cpu(&inputs));
                verify!(program.prepare_aot_simd_cpu(&inputs));
            }
        }
    }
}

#[cfg(all(feature = "metal-native", target_os = "macos"))]
#[test]
fn all_compute_metal_rejects_each_false_element_without_publication() {
    for (selection, positions, diagonal) in selections() {
        let (program, state_slot) = compute_case(selection);
        for position in positions {
            let mut session = program
                .prepare_metal(&case_inputs(Some(position), diagonal))
                .unwrap();
            session.dispatch_turns(1).unwrap();
            assert_eq!(
                session.read_published_state().unwrap()[&state_slot],
                vec![1.0; 8]
            );
            assert_variance_fault(session.dispatch_turns(1).unwrap_err());
            assert_eq!(
                session.read_published_state().unwrap()[&state_slot],
                vec![1.0; 8]
            );
        }
    }
}

#[cfg(feature = "native")]
#[test]
fn all_compute_wgpu_rejects_each_false_element_without_publication() {
    for (selection, positions, diagonal) in selections() {
        let (program, state_slot) = compute_case(selection);
        for position in positions {
            let mut session = program
                .prepare_resident(&case_inputs(Some(position), diagonal))
                .unwrap();
            session.dispatch_turns(1).unwrap();
            assert_eq!(
                session.read_published_state().unwrap().1[&state_slot],
                vec![1.0; 8]
            );
            assert_variance_fault(session.dispatch_turns(1).unwrap_err());
            assert_eq!(
                session.read_published_state().unwrap().1[&state_slot],
                vec![1.0; 8]
            );
        }
    }
}
