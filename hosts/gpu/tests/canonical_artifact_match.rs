#![cfg(feature = "canonical-native")]
//! Actual canonical scalar Match JIT and Object qualification; no source rewrite.
use mech_core::{
    ManagedMemoryBudget, ReactiveInstanceId, ValueData, ValueDataDraft,
    snapshot::{SnapshotValidationContext, ValueDraft},
};
use mech_engine::native_artifact::{
    NativeMatchAction, NativeScalarValue, lower_scalar_match_artifact,
};
use mech_engine::resident::{ActivationFacts, CapturedValueInput, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_gpu::native_artifact::{
    JitMatchArtifact, NativeExecutionError, NativeExecutionLimits, NativeMatchExecutionError,
    emit_match_object,
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
fn compile_mode(source: &str, interactive: bool) -> (ProgramArtifact, usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4d415443), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let syntax = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let product = if interactive {
        CanonicalSourceFrontend
            .compile_interactive_document_with_catalog(&syntax, mech_stdlib::source_catalog())
    } else {
        CanonicalSourceFrontend
            .compile_document_with_catalog(&syntax, mech_stdlib::source_catalog())
    }
    .unwrap();
    let output = product
        .document_outputs()
        .iter()
        .rev()
        .find(|v| v.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    (product.compile_artifact().unwrap(), output)
}
fn artifacts(source: &str) -> ([ProgramArtifact; 2], usize) {
    artifacts_mode(source, false)
}

fn artifacts_mode(source: &str, interactive: bool) -> ([ProgramArtifact; 2], usize) {
    let (original, output) = compile_mode(source, interactive);
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    ([original, decoded], output)
}
fn inputs(
    artifact: &ProgramArtifact,
    named: &[(&str, NativeScalarValue)],
) -> Vec<NativeScalarValue> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            named
                .iter()
                .find(|(key, _)| *key == name)
                .unwrap_or_else(|| panic!("missing input {name}"))
                .1
        })
        .collect()
}
fn reference(
    artifact: &ProgramArtifact,
    args: &[NativeScalarValue],
) -> Result<Vec<NativeScalarValue>, String> {
    let mut instance = activate(
        ReactiveInstanceId::new(0x4d415443, 0),
        artifact,
        &mech_stdlib::source_catalog(),
        &ActivationFacts::default(),
    )
    .map_err(|e| format!("{e:?}"))?;
    let values = artifact
        .inputs()
        .iter()
        .zip(args)
        .map(|(input, value)| {
            ValueDraft {
                schema: input.schema,
                shape_values: Box::new([]),
                data: match value {
                    NativeScalarValue::Bool(v) => ValueDataDraft::Bool(*v),
                    NativeScalarValue::U64(v) => ValueDataDraft::U64(*v),
                    NativeScalarValue::I64(v) => ValueDataDraft::I64(*v),
                    NativeScalarValue::F64(v) => ValueDataDraft::F64(*v),
                },
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap()
        })
        .collect::<Vec<_>>();
    let captured = instance
        .plan
        .inputs
        .iter()
        .map(|input| {
            let ordinal = artifact
                .inputs()
                .iter()
                .position(|v| v.slot == input.artifact_slot)
                .unwrap();
            CapturedValueInput {
                slot: input.slot,
                value: &values[ordinal],
            }
        })
        .collect::<Vec<_>>();
    instance
        .prepare_turn_values(&captured)
        .and_then(|prepared| prepared.publish())
        .map_err(|e| format!("{e:?}"))?;
    Ok((0..artifact.outputs().len())
        .map(
            |index| match instance.copied_output(index).unwrap().data() {
                ValueData::Bool(v) => NativeScalarValue::Bool(*v),
                ValueData::U64(v) => NativeScalarValue::U64(*v),
                ValueData::I64(v) => NativeScalarValue::I64(*v),
                ValueData::F64(v) => NativeScalarValue::F64(*v),
                v => panic!("unexpected scalar {v:?}"),
            },
        )
        .collect())
}

fn check(source: &str, named: &[(&str, NativeScalarValue)], expected: NativeScalarValue) {
    let (artifacts, last) = artifacts(source);
    for artifact in artifacts {
        let args = inputs(&artifact, named);
        let ordinary = reference(&artifact, &args).unwrap();
        assert_eq!(ordinary[last], expected);
        let plan = lower_scalar_match_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let jit = JitMatchArtifact::compile(&plan).unwrap();
        let mut outputs = vec![NativeScalarValue::U64(0xdead); plan.outputs().len()];
        let report = jit
            .evaluate_into(&args, &mut outputs, NativeExecutionLimits::default(), None)
            .unwrap();
        assert_eq!(outputs, ordinary);
        assert!(report.visits > 0);
        let object = emit_match_object(&plan).unwrap();
        assert_eq!(
            object.descriptor().typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        assert!(!object.bytes().is_empty());
        assert_eq!(object.metadata()["executed"], false);
    }
}
#[test]
fn scalar_matches_preserve_literals_bindings_and_raw_u64_values() {
    use NativeScalarValue::{Bool, U64};
    let source =
        "(value<u64>? | 18446744073709551615u64 => value<u64> - 1u64 | found => found + 1u64)\n";
    check(source, &[("value", U64(u64::MAX))], U64(u64::MAX - 1));
    check(source, &[("value", U64((1 << 53) + 1))], U64((1 << 53) + 2));
    let source = "(flag<bool>? | true => !other<bool> | false => other<bool>)\n";
    check(
        source,
        &[("flag", Bool(true)), ("other", Bool(false))],
        Bool(true),
    );
    check(
        source,
        &[("flag", Bool(false)), ("other", Bool(false))],
        Bool(false),
    );
}
#[test]
fn nested_guards_and_bodies_execute_in_lazy_source_order() {
    use NativeScalarValue::{Bool, U64};
    let source = "(flag<bool>? | true, (gate<bool>? | true => value<u64> / divisor<u64> == 1u64 | false => false) => value<u64> + 2u64 | true => 11u64 | false => (value<u64>? | 0u64 => 12u64 | * => value<u64> + 1u64))\n";
    check(
        source,
        &[
            ("flag", Bool(true)),
            ("gate", Bool(false)),
            ("value", U64(8)),
            ("divisor", U64(0)),
        ],
        U64(11),
    );
    check(
        source,
        &[
            ("flag", Bool(false)),
            ("gate", Bool(true)),
            ("value", U64(0)),
            ("divisor", U64(0)),
        ],
        U64(12),
    );
    check(
        source,
        &[
            ("flag", Bool(true)),
            ("gate", Bool(true)),
            ("value", U64(8)),
            ("divisor", U64(8)),
        ],
        U64(10),
    );
    let (artifacts, _) = artifacts(source);
    for artifact in artifacts {
        let args = inputs(
            &artifact,
            &[
                ("flag", Bool(true)),
                ("gate", Bool(true)),
                ("value", U64(8)),
                ("divisor", U64(0)),
            ],
        );
        assert!(reference(&artifact, &args).is_err());
        let plan = lower_scalar_match_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let jit = JitMatchArtifact::compile(&plan).unwrap();
        let mut outputs = vec![U64(77); plan.outputs().len()];
        let error = jit
            .evaluate_into(&args, &mut outputs, NativeExecutionLimits::default(), None)
            .unwrap_err();
        let NativeMatchExecutionError::Arithmetic { origin } = error else {
            panic!("{error:?}")
        };
        assert!(origin.block().is_some());
        assert!(origin.local().is_some());
        assert!(outputs.iter().all(|value| *value == U64(77)));
    }
}
#[test]
fn unused_scheduled_checked_node_is_retained_before_match() {
    use NativeScalarValue::{Bool, U64};
    let source = "unused := value<u64> / divisor<u64>\n(flag<bool>? | true => value<u64> + 1u64 | false => 0u64)\n";
    let (artifacts, _) = artifacts(source);
    for artifact in artifacts {
        let plan = lower_scalar_match_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert!(plan.blocks().iter().any(|block| matches!(
            block.action(),
            NativeMatchAction::Primitive {
                op: mech_engine::native_artifact::NativeScalarOp::U64Div,
                ..
            }
        )));
        let jit = JitMatchArtifact::compile(&plan).unwrap();
        let args = inputs(
            &artifact,
            &[
                ("flag", Bool(false)),
                ("value", U64(8)),
                ("divisor", U64(0)),
            ],
        );
        let mut outputs = vec![U64(77); plan.outputs().len()];
        let NativeMatchExecutionError::Arithmetic { origin } = jit
            .evaluate_into(&args, &mut outputs, NativeExecutionLimits::default(), None)
            .unwrap_err()
        else {
            panic!("unused division did not fail")
        };
        assert_eq!(origin.node(), artifact.nodes()[0].node);
        assert_eq!(origin.block(), None);
        assert!(outputs.iter().all(|value| *value == U64(77)));
    }
}
#[test]
fn first_arithmetic_error_precedes_later_cfg_work_denial_and_recovers() {
    use NativeScalarValue::{Bool, U64};
    let source = "(flag<bool>? | true => value<u64> / divisor<u64> | false => value<u64> + 1u64)\n";
    let (artifacts, _) = artifacts(source);
    for artifact in artifacts {
        let plan = lower_scalar_match_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let jit = JitMatchArtifact::compile(&plan).unwrap();
        let good = inputs(
            &artifact,
            &[("flag", Bool(true)), ("value", U64(8)), ("divisor", U64(1))],
        );
        let bad = inputs(
            &artifact,
            &[("flag", Bool(true)), ("value", U64(8)), ("divisor", U64(0))],
        );
        let mut outputs = vec![U64(77); plan.outputs().len()];
        let report = jit
            .evaluate_into(&good, &mut outputs, NativeExecutionLimits::default(), None)
            .unwrap();
        let mut low = 0;
        let mut high = report.work;
        while low < high {
            let middle = low + (high - low) / 2;
            let mut candidate = vec![U64(77); outputs.len()];
            match jit
                .evaluate_into(
                    &bad,
                    &mut candidate,
                    NativeExecutionLimits {
                        max_work: middle,
                        ..NativeExecutionLimits::default()
                    },
                    None,
                )
                .unwrap_err()
            {
                NativeMatchExecutionError::Arithmetic { .. } => high = middle,
                NativeMatchExecutionError::Work { .. } => low = middle + 1,
                error => panic!("{error:?}"),
            }
            assert!(candidate.iter().all(|value| *value == U64(77)));
        }
        let limits = NativeExecutionLimits {
            max_work: low,
            ..NativeExecutionLimits::default()
        };
        outputs.fill(U64(77));
        assert!(matches!(
            jit.evaluate_into(&bad, &mut outputs, limits, None),
            Err(NativeMatchExecutionError::Arithmetic { .. })
        ));
        assert!(matches!(
            jit.evaluate_into(&good, &mut outputs, limits, None),
            Err(NativeMatchExecutionError::Work { .. })
        ));
        assert!(outputs.iter().all(|value| *value == U64(77)));
        jit.evaluate_into(&good, &mut outputs, NativeExecutionLimits::default(), None)
            .unwrap();
        assert_eq!(outputs, reference(&artifact, &good).unwrap());
    }
}
#[test]
fn full_call_denials_keep_all_outputs_atomic_and_object_pairing_is_exact() {
    use NativeScalarValue::{Bool, U64};
    let source = "selected := (flag<bool>? | true => value<u64> + 1u64 | false => value<u64> - 1u64)\nother := !flag<bool>\nselected\n";
    let (artifacts, _) = artifacts_mode(source, true);
    let plans = artifacts
        .iter()
        .map(|artifact| {
            lower_scalar_match_artifact(artifact, &mech_stdlib::source_catalog()).unwrap()
        })
        .collect::<Vec<_>>();
    let original = emit_match_object(&plans[0]).unwrap();
    let decoded = emit_match_object(&plans[1]).unwrap();
    assert_eq!(original.bytes(), decoded.bytes());
    assert_eq!(original.identity_bytes(), decoded.identity_bytes());
    assert_eq!(original.identity_symbol(), decoded.identity_symbol());
    for (artifact, plan) in artifacts.iter().zip(&plans) {
        let jit = JitMatchArtifact::compile(plan).unwrap();
        let args = inputs(artifact, &[("flag", Bool(true)), ("value", U64(8))]);
        for (limits, budget) in [
            (
                NativeExecutionLimits {
                    max_work: 0,
                    ..NativeExecutionLimits::default()
                },
                None,
            ),
            (
                NativeExecutionLimits {
                    max_transport_bytes: 0,
                    ..NativeExecutionLimits::default()
                },
                None,
            ),
            (
                NativeExecutionLimits {
                    max_native_stack_bytes: 0,
                    ..NativeExecutionLimits::default()
                },
                None,
            ),
            (
                NativeExecutionLimits::default(),
                Some(ManagedMemoryBudget::new(0)),
            ),
        ] {
            let mut outputs = vec![U64(77); plan.outputs().len()];
            let error = jit
                .evaluate_into(&args, &mut outputs, limits, budget.as_ref())
                .unwrap_err();
            assert!(
                matches!(
                    error,
                    NativeMatchExecutionError::Work { .. }
                        | NativeMatchExecutionError::Base(
                            NativeExecutionError::TransportLimit { .. }
                                | NativeExecutionError::StackLimit { .. }
                                | NativeExecutionError::Memory { .. }
                        )
                ),
                "{error:?}"
            );
            assert!(outputs.iter().all(|value| *value == U64(77)));
        }
        let mut outputs = vec![U64(77); plan.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, NativeExecutionLimits::default(), None)
            .unwrap();
        assert_eq!(outputs, reference(artifact, &args).unwrap());
    }
}

#[test]
fn signed_and_float_literals_use_language_equality_and_preserve_raw_values() {
    use NativeScalarValue::{F64, I64};
    use mech_core::snapshot::F64Bits;
    let signed = "(value<i64>? | 0i64 => 7i64 | found => found)\n";
    for value in [i64::MIN, -1, 0, 1, i64::MAX] {
        check(
            signed,
            &[("value", I64(value))],
            I64(if value == 0 { 7 } else { value }),
        );
    }
    let floating = "(value<f64>? | 0.0 => 7.0 | found => found)\n";
    for bits in [
        0,
        1u64 << 63,
        1,
        0x3ff0_0000_0000_0000,
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        0x7ff8_1234_5678_9abc,
        0x7ff0_0000_0000_0001,
    ] {
        let value = F64Bits::from_bits(bits);
        let expected = if value.to_f64() == 0.0 {
            F64Bits::from_f64(7.0)
        } else {
            value
        };
        check(floating, &[("value", F64(value))], F64(expected));
    }
}

#[path = "support/canonical_cast_cases.rs"]
mod cast_cases;

#[test]
fn all_six_selected_casts_compose_with_match_and_keep_errors_atomic() {
    use NativeScalarValue::{Bool, F64, I64, U64};
    use mech_core::snapshot::F64Bits;
    for cast in cast_cases::CASTS {
        let source = cast.source.replace(
            "\nresult\n",
            "\n(flag<bool>? | true => result | false => result)\n",
        );
        for artifact in artifacts(&source).0 {
            let plan =
                lower_scalar_match_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert!(plan.blocks().iter().any(|block| matches!(block.action(),
                NativeMatchAction::Primitive { op, .. } if *op == cast.op)));
            assert!(
                plan.outputs()
                    .iter()
                    .any(|output| output.ty().kind() == cast.output)
            );
            let jit = JitMatchArtifact::compile(&plan).unwrap();
            for flag in [false, true] {
                for value in cast_cases::samples(cast) {
                    let args = inputs(&artifact, &[("value", value), ("flag", Bool(flag))]);
                    let mut outputs = vec![U64(77); plan.outputs().len()];
                    let actual = jit.evaluate_into(
                        &args,
                        &mut outputs,
                        NativeExecutionLimits::default(),
                        None,
                    );
                    match reference(&artifact, &args) {
                        Ok(expected) => {
                            actual.unwrap();
                            assert_eq!(
                                outputs, expected,
                                "{} flag={flag} input={value:?}",
                                cast.name
                            );
                        }
                        Err(_) => {
                            assert!(
                                matches!(actual, Err(NativeMatchExecutionError::Arithmetic { .. })),
                                "{} flag={flag} input={value:?}: {actual:?}",
                                cast.name
                            );
                            assert!(outputs.iter().all(|value| *value == U64(77)));
                        }
                    }
                }
            }
            // Zero converts successfully in every direction after earlier
            // failed calls, using the same compiled entry and fresh staging.
            let zero = match cast.input {
                mech_engine::native_artifact::NativeScalarKind::U64 => U64(0),
                mech_engine::native_artifact::NativeScalarKind::I64 => I64(0),
                mech_engine::native_artifact::NativeScalarKind::F64 => F64(F64Bits::from_bits(0)),
                _ => unreachable!(),
            };
            let args = inputs(&artifact, &[("value", zero), ("flag", Bool(true))]);
            let mut outputs = vec![U64(77); plan.outputs().len()];
            jit.evaluate_into(&args, &mut outputs, NativeExecutionLimits::default(), None)
                .unwrap();
            assert_eq!(outputs, reference(&artifact, &args).unwrap());

            // An equal-width zero word of a different numeric kind is not a
            // typed input, even though all its transport bits would agree.
            let ordinal = plan
                .inputs()
                .iter()
                .position(|input| input.declaration().ty().kind() == cast.input)
                .unwrap();
            let mut wrong = args.clone();
            wrong[ordinal] = if matches!(zero, U64(_)) {
                I64(0)
            } else {
                U64(0)
            };
            outputs.fill(U64(77));
            assert!(
                matches!(jit.evaluate_into(&wrong, &mut outputs, NativeExecutionLimits::default(), None),
                Err(NativeMatchExecutionError::Base(NativeExecutionError::InputType { input, expected }))
                    if input == ordinal && expected == cast.input)
            );
            assert!(outputs.iter().all(|value| *value == U64(77)));
        }
    }
}

#[test]
fn signed_checked_division_stays_lazy_and_recovers_after_selected_failures() {
    use NativeScalarValue::{Bool, I64, U64};
    let source = "(flag<bool>? | true => left<i64> / right<i64> | false => left<i64>)\n";
    for artifact in artifacts(source).0 {
        let plan = lower_scalar_match_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let jit = JitMatchArtifact::compile(&plan).unwrap();
        for (left, right) in [(i64::MIN, -1), (8, 0)] {
            let skipped = inputs(
                &artifact,
                &[
                    ("flag", Bool(false)),
                    ("left", I64(left)),
                    ("right", I64(right)),
                ],
            );
            let mut outputs = vec![U64(77); plan.outputs().len()];
            jit.evaluate_into(
                &skipped,
                &mut outputs,
                NativeExecutionLimits::default(),
                None,
            )
            .unwrap();
            assert_eq!(outputs, reference(&artifact, &skipped).unwrap());
            let selected = inputs(
                &artifact,
                &[
                    ("flag", Bool(true)),
                    ("left", I64(left)),
                    ("right", I64(right)),
                ],
            );
            assert!(reference(&artifact, &selected).is_err());
            outputs.fill(U64(77));
            let error = jit
                .evaluate_into(
                    &selected,
                    &mut outputs,
                    NativeExecutionLimits::default(),
                    None,
                )
                .unwrap_err();
            assert!(
                matches!(error, NativeMatchExecutionError::Arithmetic { origin } if origin.block().is_some())
            );
            assert!(outputs.iter().all(|value| *value == U64(77)));
        }
        let good = inputs(
            &artifact,
            &[("flag", Bool(true)), ("left", I64(-8)), ("right", I64(2))],
        );
        let mut outputs = vec![U64(77); plan.outputs().len()];
        jit.evaluate_into(&good, &mut outputs, NativeExecutionLimits::default(), None)
            .unwrap();
        assert_eq!(outputs, reference(&artifact, &good).unwrap());
    }
}
