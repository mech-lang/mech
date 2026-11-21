//! Execute the compressor's actual short-block predicate with finite admission.
use mech_core::{
    ManagedMemoryBudget, ValueDataDraft,
    snapshot::{SnapshotValidationContext, ValueDraft},
};
use mech_engine::CanonicalSourceFrontend;
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeExecutionLimits, NativeProgramExecutionLimits,
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

#[test]
fn short_literal_proof_observes_current_input_original_and_decoded() {
    // Use the maintained definitions, not a second implementation of them.
    let byte = include_str!("../../../hosts/gpu/tests/fixtures/native/deflate-matchfinder.mec");
    let byte = &byte[..byte.find("prefix-readable(input").unwrap()];
    let controller =
        include_str!("../../../hosts/gpu/tests/fixtures/native/deflate-compress-controller.mec");
    let start = controller.find("optimal-short-triple(input").unwrap();
    let end = controller
        .find("optimal-short-write-literals(bytes")
        .unwrap();
    let source = format!(
        "{byte}\n{}\noptimal-short-forced-literals(input<[u8]>)\n",
        &controller[start..end]
    );
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x53484f52), Revision(1), source.as_str()).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let syntax = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let catalog = mech_stdlib::source_catalog();
    let original = CanonicalSourceFrontend
        .compile_document_with_catalog(&syntax, catalog.clone())
        .unwrap()
        .compile_artifact()
        .unwrap();
    let encoded = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
    let mut cases = Vec::new();
    for len in 0..=10 {
        for bits in 0..1usize << len {
            cases.push(
                (0..len)
                    .map(|at| b'a' + ((bits >> at) & 1) as u8)
                    .collect::<Vec<_>>(),
            );
        }
    }
    for mask in 0..256 {
        cases.push(
            (0..8)
                .map(|at| at as u8 + if mask & (1 << at) != 0 { 144 } else { 0 })
                .collect(),
        );
    }
    for (form, artifact) in [("original", original), ("decoded", decoded)] {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        assert_eq!(artifact.inputs().len(), 1);
        // The established finite native qualification profile, shared with
        // canonical_artifact_program. This is not the timing entry's policy.
        let mut library = mech_core::TargetMemoryProfile::current_resident_cpu()
            .unwrap()
            .limits;
        library.max_compute_work = Some(134_217_728);
        library.max_comparison_work = Some(1_000_000);
        library.max_temporary_bytes = Some(128 * 1024 * 1024);
        let limits = NativeProgramExecutionLimits {
            execution: NativeExecutionLimits {
                max_work: 1_000_000_000,
                ..NativeExecutionLimits::default()
            },
            library,
        };
        for bytes in &cases {
            let expected = bytes.len() <= 8
                && (0..bytes.len().saturating_sub(2)).all(|at| {
                    !(at + 1..bytes.len().saturating_sub(2))
                        .any(|other| bytes[at..at + 3] == bytes[other..other + 3])
                });
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let schema = artifact.inputs()[0].schema;
            let shape = mech_core::shape_for_resolved_extents(
                artifact.schemas().get(schema).unwrap(),
                &[1, bytes.len() as u64],
            )
            .unwrap();
            let value = ValueDraft {
                schema,
                shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
                data: ValueDataDraft::Matrix(
                    bytes.iter().copied().map(ValueDataDraft::U8).collect(),
                ),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut reservation = memory
                .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                .unwrap();
            let arguments = vec![value.into_memory_budget(&mut reservation).unwrap()];
            drop(reservation);
            let aliases = arguments.clone();
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&arguments, &mut output, limits, &memory)
                .unwrap();
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                ValueDataDraft::Bool(expected),
                "{form}: {bytes:?}"
            );
            assert!(arguments[0].shares_frozen_storage(&aliases[0]));
            drop((arguments, aliases, output));
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
        println!(
            "{form}: {} short-block predicate visits, finite admission, unchanged aliases, zero retained bytes/nodes",
            cases.len()
        );
    }
}
