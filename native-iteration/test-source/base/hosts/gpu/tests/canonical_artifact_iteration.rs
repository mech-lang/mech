#![cfg(feature = "canonical-native")]
//! Ordinary canonical counted iteration through real Cranelift loops.
//! Scalar subproof only; managed matrix/native resources are separate work.
use mech_core::{
    ManagedMemoryBudget, ReactiveInstanceId, ValueData, ValueDataDraft,
    snapshot::{SnapshotValidationContext, ValueDraft},
};
use mech_engine::native_artifact::{
    NativeIterationAction, NativeMatchAction, NativeScalarValue, lower_scalar_iteration_artifact,
};
use mech_engine::resident::{ActivationFacts, CapturedValueInput, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_gpu::native_artifact::{
    JitIterationArtifact, NativeExecutionError, NativeExecutionLimits,
    NativeIterationExecutionError, emit_iteration_object,
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
        let plan =
            lower_scalar_iteration_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        plan.validate_current_visits().unwrap();
        let jit = JitIterationArtifact::compile(&plan).unwrap();
        let mut output = vec![NativeScalarValue::U64(77); plan.outputs().len()];
        let report = jit
            .evaluate_into(&args, &mut output, NativeExecutionLimits::default(), None)
            .unwrap();
        assert_eq!(output, ordinary);
        assert!(report.visits > 0);
        let object = emit_iteration_object(&plan).unwrap();
        assert!(!object.bytes().is_empty());
        assert_eq!(
            object.descriptor().typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        assert_eq!(
            object.descriptor().iterations().len(),
            plan.iterations().len()
        );
        assert_eq!(object.metadata()["executed"], false);
    }
}
#[test]
fn exact_zero_and_over_twenty_four_counts_execute_real_loops() {
    use NativeScalarValue::U64;
    for count in [0, 1, 25, 64] {
        let source =
            format!("step(x<u64>) => (next<u64>) := next := x + 1u64.\nseed<u64> |{count}> step\n");
        check(
            &source,
            &[("seed", U64((1 << 53) + 1))],
            U64((1 << 53) + 1 + count),
        );
    }
    // The checked operation is retained but is not executed at count zero.
    check(
        "step(x<u64>) => (next<u64>) := next := 1u64 / x.\nseed<u64> |0> step\n",
        &[("seed", U64(0))],
        U64(0),
    );
}
#[test]
fn nested_carriers_and_invocation_captures_are_current_visit_values() {
    use NativeScalarValue::U64;
    let source = "step(x<u64>) => (next<u64>) := next := x + delta.\nouter(x<u64>) => (next<u64>) := next := x |3> step.\ndelta := amount<u64> + 0u64\nseed<u64> |2> outer\n";
    check(source, &[("seed", U64(4)), ("amount", U64(7))], U64(46));
    check(source, &[("seed", U64(1)), ("amount", U64(0))], U64(1));
    let (artifacts, _) = artifacts(source);
    let plan =
        lower_scalar_iteration_artifact(&artifacts[0], &mech_stdlib::source_catalog()).unwrap();
    assert_eq!(plan.iterations().len(), 2);
    assert_eq!(plan.iterations()[1].parent(), Some(0));
    assert_ne!(
        plan.iterations()[0].carrier(),
        plan.iterations()[1].carrier()
    );
}
#[test]
fn matches_in_loop_bodies_preserve_lazy_errors_and_all_checked_nodes() {
    use NativeScalarValue::{Bool, U64};
    let source = "step(x<u64>) => (next<u64>) := next := (choose<bool>? | true => x + 1u64 | false => 1u64 / divisor<u64>).\nseed<u64> |3> step\n";
    check(
        source,
        &[
            ("seed", U64(9)),
            ("choose", Bool(true)),
            ("divisor", U64(0)),
        ],
        U64(12),
    );
    let (artifacts, _) = artifacts(source);
    for artifact in artifacts {
        let args = inputs(
            &artifact,
            &[
                ("seed", U64(9)),
                ("choose", Bool(false)),
                ("divisor", U64(0)),
            ],
        );
        assert!(reference(&artifact, &args).is_err());
        let plan =
            lower_scalar_iteration_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let jit = JitIterationArtifact::compile(&plan).unwrap();
        let mut output = vec![U64(99); plan.outputs().len()];
        let before = output.clone();
        let error = jit
            .evaluate_into(&args, &mut output, NativeExecutionLimits::default(), None)
            .unwrap_err();
        assert!(matches!(
            error,
            NativeIterationExecutionError::Arithmetic { .. }
        ));
        assert_eq!(output, before);
    }
    let source = "step(x<u64>) => (next<u64>) := unused := 1u64 / divisor<u64>\nnext := x + 1u64.\nseed<u64> |2> step\n";
    let (artifacts, _) = artifacts(source);
    for artifact in artifacts {
        let args = inputs(&artifact, &[("seed", U64(4)), ("divisor", U64(0))]);
        assert!(reference(&artifact, &args).is_err());
        let plan =
            lower_scalar_iteration_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert!(plan.blocks().iter().any(|b| matches!(
            b.action(),
            NativeIterationAction::Scalar(NativeMatchAction::Primitive {
                op: mech_engine::native_artifact::NativeScalarOp::U64Div,
                ..
            })
        )));
        let jit = JitIterationArtifact::compile(&plan).unwrap();
        let mut output = vec![U64(88); plan.outputs().len()];
        assert!(matches!(
            jit.evaluate_into(&args, &mut output, NativeExecutionLimits::default(), None),
            Err(NativeIterationExecutionError::Arithmetic { .. })
        ));
        assert!(output.iter().all(|v| *v == U64(88)));
    }
}
#[test]
fn max_count_is_finitely_denied_and_storage_is_independent_of_count() {
    use NativeScalarValue::U64;
    let source = |n: u64| {
        format!("step(x<u64>) => (next<u64>) := next := x + 0u64.\nseed<u64> |{n}> step\n")
    };
    let (small, _) = artifacts(&source(1));
    let (large, _) = artifacts(&source(u64::MAX));
    let small_plan =
        lower_scalar_iteration_artifact(&small[0], &mech_stdlib::source_catalog()).unwrap();
    for artifact in large {
        let args = inputs(&artifact, &[("seed", U64(5))]);
        let plan =
            lower_scalar_iteration_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(plan.values().len(), small_plan.values().len());
        assert_eq!(plan.blocks().len(), small_plan.blocks().len());
        let jit = JitIterationArtifact::compile(&plan).unwrap();
        let mut output = vec![U64(77); plan.outputs().len()];
        assert!(matches!(
            jit.evaluate_into(
                &args,
                &mut output,
                NativeExecutionLimits {
                    max_work: 20_000,
                    ..NativeExecutionLimits::default()
                },
                None
            ),
            Err(NativeIterationExecutionError::Work {
                origin: Some(_),
                ..
            })
        ));
        assert!(output.iter().all(|v| *v == U64(77)));
    }
}
#[test]
fn finite_work_memory_stack_and_completion_denials_are_atomic_and_recover() {
    use NativeScalarValue::U64;
    let (artifacts, last) =
        artifacts("step(x<u64>) => (next<u64>) := next := x + 1u64.\nseed<u64> |25> step\n");
    for artifact in artifacts {
        let args = inputs(&artifact, &[("seed", U64(5))]);
        let plan =
            lower_scalar_iteration_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let jit = JitIterationArtifact::compile(&plan).unwrap();
        let mut output = vec![U64(77); plan.outputs().len()];
        let report = jit
            .evaluate_into(&args, &mut output, NativeExecutionLimits::default(), None)
            .unwrap();
        assert_eq!(output[last], U64(30));
        let object = emit_iteration_object(&plan).unwrap();
        for limits in [
            NativeExecutionLimits {
                max_work: 0,
                ..NativeExecutionLimits::default()
            },
            NativeExecutionLimits {
                max_work: report.work - 1,
                ..NativeExecutionLimits::default()
            },
            NativeExecutionLimits {
                max_transport_bytes: 0,
                ..NativeExecutionLimits::default()
            },
            NativeExecutionLimits {
                max_native_stack_bytes: object.descriptor().stack_bytes() - 1,
                ..NativeExecutionLimits::default()
            },
        ] {
            output.fill(U64(77));
            let before = output.clone();
            assert!(jit.evaluate_into(&args, &mut output, limits, None).is_err());
            assert_eq!(output, before);
        }
        let budget = ManagedMemoryBudget::new(0);
        output.fill(U64(77));
        assert!(matches!(
            jit.evaluate_into(
                &args,
                &mut output,
                NativeExecutionLimits::default(),
                Some(&budget)
            ),
            Err(NativeIterationExecutionError::Base(
                NativeExecutionError::Memory { .. }
            ))
        ));
        assert!(output.iter().all(|v| *v == U64(77)));
        jit.evaluate_into(
            &args,
            &mut output,
            NativeExecutionLimits {
                max_work: report.work,
                ..NativeExecutionLimits::default()
            },
            None,
        )
        .unwrap();
        assert_eq!(output[last], U64(30));
    }
}
