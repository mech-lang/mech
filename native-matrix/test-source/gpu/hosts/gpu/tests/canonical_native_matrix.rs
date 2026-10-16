#![cfg(feature = "canonical-native")]
//! Actual canonical source/resident/JIT differentials. Object assertions alone
//! are non-execution evidence; linked execution has its own ignored qualifier.
use mech_core::{
    ManagedMemoryBudget, ReactiveInstanceId, Value, ValueData, ValueDataDraft, ValueDraft,
    snapshot::{F64Bits, SequenceView, SnapshotValidationContext},
};
use mech_engine::{
    CanonicalSourceFrontend, ProgramArtifact, decode_source_input_name,
    native_artifact::{NativeMatrixArtifactPlan, lower_matrix_artifact},
    resident::{ActivationFacts, CapturedValueInput, activate},
};
use mech_gpu::native_artifact::matrix::{
    JitMatrixArtifact, NativeMatrixExecutionError as Error, NativeMatrixExecutionLimits as Limits,
    NativeMatrixExecutionPhase as Phase, NativeMatrixValue, emit_matrix_object,
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn artifacts(source: &str, interactive: bool) -> [ProgramArtifact; 2] {
    let parse = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4d41544e4154), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parse.is_strictly_clean(),
        "{source}: {:?}",
        parse.diagnostics
    );
    let doc = DocumentSyntax::cast(parse.syntax()).unwrap();
    let product = if interactive {
        CanonicalSourceFrontend
            .compile_interactive_document_with_catalog(&doc, mech_stdlib::source_catalog())
    } else {
        CanonicalSourceFrontend.compile_document_with_catalog(&doc, mech_stdlib::source_catalog())
    }
    .unwrap();
    let artifact = product.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(artifact.revision(), decoded.revision());
    assert_eq!(
        bytes,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    [artifact, decoded]
}
fn inputs(artifact: &ProgramArtifact, named: &[(&str, [u64; 2], Vec<u64>)]) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = decode_source_input_name(&input.name).unwrap();
            let (_, extents, bits) = named.iter().find(|(key, _, _)| *key == name).unwrap();
            let schema = artifact.schemas().get(input.schema).unwrap();
            let shape = mech_core::shape_for_resolved_extents(schema, extents).unwrap();
            ValueDraft {
                schema: input.schema,
                shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
                data: ValueDataDraft::Matrix(
                    bits.iter()
                        .map(|bits| ValueDataDraft::F64(F64Bits::from_bits(*bits)))
                        .collect(),
                ),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap()
        })
        .collect()
}
fn reference(
    artifact: &ProgramArtifact,
    inputs: &[Value],
) -> Result<Vec<([u64; 2], Vec<u64>)>, String> {
    let mut instance = activate(
        ReactiveInstanceId::new(0x4d41544e4154, 0),
        artifact,
        &mech_stdlib::source_catalog(),
        &ActivationFacts::default(),
    )
    .map_err(|e| format!("{e:?}"))?;
    let captured = instance
        .plan
        .inputs
        .iter()
        .map(|input| {
            let ordinal = artifact
                .inputs()
                .iter()
                .position(|declared| declared.slot == input.artifact_slot)
                .unwrap();
            CapturedValueInput {
                slot: input.slot,
                value: &inputs[ordinal],
            }
        })
        .collect::<Vec<_>>();
    instance
        .prepare_turn_values(&captured)
        .and_then(|p| p.publish())
        .map_err(|e| format!("{e:?}"))?;
    (0..artifact.outputs().len())
        .map(|index| {
            let value = instance.copied_output(index).unwrap();
            let owner = value.schemas().unwrap();
            let schema = value.validate_against(&owner).unwrap();
            let mech_core::SchemaBody::Matrix { dimensions, .. } = schema.body() else {
                panic!("matrix result required")
            };
            let extents = [
                value.shape().resolve_dimension(&dimensions[0]).unwrap(),
                value.shape().resolve_dimension(&dimensions[1]).unwrap(),
            ];
            let ValueData::Matrix(matrix) = value.data() else {
                panic!("matrix payload required")
            };
            let bits = match matrix.elements() {
                SequenceView::F64(xs) => xs.iter().map(|v| v.bits()).collect(),
                SequenceView::Values(xs) => xs
                    .iter()
                    .map(|v| match v {
                        ValueData::F64(v) => v.bits(),
                        _ => panic!("F64 lane required"),
                    })
                    .collect(),
                other => {
                    assert_eq!(other.len(), 0);
                    Vec::new()
                }
            };
            Ok((extents, bits))
        })
        .collect()
}
fn actual(outputs: &[Option<NativeMatrixValue>]) -> Vec<([u64; 2], Vec<u64>)> {
    outputs
        .iter()
        .map(|output| {
            let output = output.as_ref().unwrap();
            let [rows, cols] = output.extents();
            let mut bits = Vec::new();
            for ordinal in 0..rows.checked_mul(cols).unwrap() {
                let row = ordinal / cols;
                let col = ordinal % cols;
                bits.push(output.column_major_values()[(row + col * rows) as usize].to_bits());
            }
            ([rows, cols], bits)
        })
        .collect()
}
fn check(artifact: &ProgramArtifact, args: &[Value]) {
    let expected = reference(artifact, args).unwrap();
    let plan = lower_matrix_artifact(artifact, &mech_stdlib::source_catalog()).unwrap();
    let jit = JitMatrixArtifact::compile(&plan).unwrap();
    let budget = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let mut output = vec![None; plan.outputs().len()];
    let report = jit
        .evaluate_into(args, &mut output, Limits::default(), &budget)
        .unwrap();
    assert_eq!(actual(&output), expected);
    assert_eq!(report.completed_nodes, artifact.nodes().len());
    assert_eq!(
        jit.descriptor().plan().program_revision(),
        artifact.revision()
    );
    let retained = budget.used_bytes();
    assert!(retained > 0);
    drop(output);
    assert_eq!(
        budget.used_bytes(),
        0,
        "all private/result ownership must release"
    );
}
#[test]
fn transpose_preserves_rectangular_geometry_and_every_f64_transport_bit() {
    let bits = vec![
        0,
        1,
        1u64 << 63,
        0x7ff0000000000000,
        0xfff0000000000000,
        0x7ff8000000001234,
    ];
    for artifact in artifacts("input<[f64]>'\n", false) {
        for (shape, bits) in [
            ([2, 3], bits.clone()),
            ([0, 3], vec![]),
            ([3, 0], vec![]),
            ([1_000_000_000, 0], vec![]),
            ([1, 1], vec![0x7ff0000000000001]),
        ] {
            check(&artifact, &inputs(&artifact, &[("input", shape, bits)]));
        }
    }
}
#[test]
fn matrix_product_matches_actual_maintained_resident_association_and_empty_inner_bits() {
    for artifact in artifacts("left<[f64]> ** right<[f64]>\n", false) {
        for (left, right, a, b) in [
            (
                [2, 3],
                [3, 2],
                vec![1.0, 2.0, 3.0, -0.0, 4.0, 5.0],
                vec![6.0, 7.0, 8.0, 9.0, 10.0, 11.0],
            ),
            ([2, 0], [0, 3], vec![], vec![]),
            ([0, 2], [2, 3], vec![], vec![0.0; 6]),
            ([2, 3], [3, 0], vec![0.0; 6], vec![]),
        ] {
            let values = inputs(
                &artifact,
                &[
                    ("left", left, a.into_iter().map(f64::to_bits).collect()),
                    ("right", right, b.into_iter().map(f64::to_bits).collect()),
                ],
            );
            if left[1] == 0 && left[0] > 0 && right[1] > 0 {
                assert!(
                    reference(&artifact, &values).unwrap()[0]
                        .1
                        .iter()
                        .all(|b| *b == (-0.0f64).to_bits())
                );
            }
            check(&artifact, &values);
        }
        check(
            &artifact,
            &inputs(
                &artifact,
                &[
                    ("left", [1, 2], vec![0x7ff8000000001234, 1]),
                    ("right", [2, 1], vec![1.0f64.to_bits(), (-0.0f64).to_bits()]),
                ],
            ),
        );
    }
}
#[test]
fn chained_transpose_product_and_interactive_aliases_use_real_typed_results() {
    for artifact in artifacts("left<[f64]>' ** right<[f64]>\n", false) {
        check(
            &artifact,
            &inputs(
                &artifact,
                &[
                    (
                        "left",
                        [2, 3],
                        (1..=6).map(|n| (n as f64).to_bits()).collect(),
                    ),
                    (
                        "right",
                        [2, 2],
                        vec![1.0f64.to_bits(), 0, 0, 1.0f64.to_bits()],
                    ),
                ],
            ),
        );
    }
    for artifact in artifacts(
        "transposed := input<[f64]>'\nalias := transposed\nalias\n",
        true,
    ) {
        let args = inputs(
            &artifact,
            &[(
                "input",
                [2, 3],
                (1..=6).map(|n| (n as f64).to_bits()).collect(),
            )],
        );
        check(&artifact, &args);
        let plan = lower_matrix_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let jit = JitMatrixArtifact::compile(&plan).unwrap();
        let budget = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let mut outputs = vec![None; plan.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, Limits::default(), &budget)
            .unwrap();
        let first = outputs[0].as_ref().unwrap().column_major_values().as_ptr();
        assert!(
            outputs
                .iter()
                .all(|value| value.as_ref().unwrap().column_major_values().as_ptr() == first)
        );
        drop(outputs);
        assert_eq!(budget.used_bytes(), 0);
    }
}
fn last(plan: &NativeMatrixArtifactPlan) -> usize {
    plan.outputs().len()
}
#[test]
fn setup_completion_and_real_memory_denials_preserve_old_outputs_and_recover() {
    for artifact in artifacts("input<[f64]>'\n", false) {
        let args = inputs(
            &artifact,
            &[(
                "input",
                [2, 3],
                (1..=6).map(|n| (n as f64).to_bits()).collect(),
            )],
        );
        let plan = lower_matrix_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let jit = JitMatrixArtifact::compile(&plan).unwrap();
        let budget = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let mut outputs = vec![None; last(&plan)];
        let report = jit
            .evaluate_into(&args, &mut outputs, Limits::default(), &budget)
            .unwrap();
        let before = actual(&outputs);
        let pointer = outputs[0].as_ref().unwrap().column_major_values().as_ptr();
        let live = budget.used_bytes();
        let mut setup = Limits::default();
        setup.max_work = plan.setup_demand().work - 1;
        assert!(matches!(
            jit.evaluate_into(&args, &mut outputs, setup, &budget),
            Err(Error::Work {
                phase: Phase::Setup,
                ..
            })
        ));
        assert_eq!(actual(&outputs), before);
        assert_eq!(budget.used_bytes(), live);
        let mut completion = Limits::default();
        completion.max_work = report.work - 1;
        assert!(matches!(
            jit.evaluate_into(&args, &mut outputs, completion, &budget),
            Err(Error::Work {
                phase: Phase::Completion,
                ..
            })
        ));
        assert_eq!(
            outputs[0].as_ref().unwrap().column_major_values().as_ptr(),
            pointer
        );
        assert_eq!(budget.used_bytes(), live);
        let tiny = ManagedMemoryBudget::new(1);
        assert!(matches!(
            jit.evaluate_into(&args, &mut outputs, Limits::default(), &tiny),
            Err(Error::Memory(_))
        ));
        assert_eq!(tiny.used_bytes(), 0);
        assert_eq!(actual(&outputs), before);
        jit.evaluate_into(&args, &mut outputs, Limits::default(), &budget)
            .unwrap();
        assert_eq!(actual(&outputs), before);
        drop(outputs);
        assert_eq!(budget.used_bytes(), 0);
    }
}
#[test]
fn ordered_node_admission_precedes_later_shape_error_and_unused_checked_calls_are_retained() {
    for artifact in artifacts(
        "unused := left<[f64]> ** right<[f64]>\ninput<[f64]>'\n",
        false,
    ) {
        let plan = lower_matrix_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let jit = JitMatrixArtifact::compile(&plan).unwrap();
        let values = inputs(
            &artifact,
            &[
                ("left", [2, 3], vec![0; 6]),
                ("right", [4, 2], vec![0; 8]),
                ("input", [1, 1], vec![1.0f64.to_bits()]),
            ],
        );
        let mut out = vec![None; plan.outputs().len()];
        let budget = ManagedMemoryBudget::new(64 * 1024 * 1024);
        assert!(
            matches!(jit.evaluate_into(&values,&mut out,Limits::default(),&budget),Err(Error::Planning{node})if node==plan.calls()[0].template().node())
        );
        assert!(out.iter().all(Option::is_none));
        assert_eq!(budget.used_bytes(), 0);
    }
    for artifact in artifacts("left<[f64]>' ** right<[f64]>\n", false) {
        let plan = lower_matrix_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let jit = JitMatrixArtifact::compile(&plan).unwrap();
        let values = inputs(
            &artifact,
            &[("left", [2, 3], vec![0; 6]), ("right", [4, 2], vec![0; 8])],
        );
        let mut out = vec![None; plan.outputs().len()];
        let budget = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let mut tiny = Limits::default();
        tiny.max_work = plan.setup_demand().work + 256 + plan.calls()[0].planning_demand().work - 1;
        assert!(
            matches!(jit.evaluate_into(&values,&mut out,tiny,&budget),Err(Error::Work{phase:Phase::Node(node),..})if node==plan.calls()[0].template().node())
        );
        assert_eq!(budget.used_bytes(), 0);
        assert!(
            matches!(jit.evaluate_into(&values,&mut out,Limits::default(),&budget),Err(Error::Planning{node})if node==plan.calls()[1].template().node())
        );
        assert_eq!(budget.used_bytes(), 0);
    }
}
#[test]
fn object_emission_identity_is_deterministic_and_explicitly_not_linked_execution() {
    for artifact in artifacts("left<[f64]>' ** right<[f64]>\n", false) {
        let plan = lower_matrix_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let first = emit_matrix_object(&plan).unwrap();
        let second = emit_matrix_object(&plan).unwrap();
        assert_eq!(first.bytes(), second.bytes());
        assert_eq!(first.identity().bytes(), second.identity().bytes());
        assert_eq!(first.metadata()["linked"], false);
        assert_eq!(first.metadata()["executed"], false);
        assert_eq!(first.metadata()["timing_samples"], 0);
        assert_eq!(
            first.descriptor().plan().program_revision(),
            artifact.revision()
        );
    }
}
