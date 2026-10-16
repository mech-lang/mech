//! Ordinary source to private typed matrix plans; no native execution claim.
#![cfg(all(feature = "full_compiler", feature = "native-artifact"))]

use mech_core::{ExecutionTarget, ResolvedValueDescriptor};
use mech_engine::{
    CanonicalSourceFrontend, ProgramArtifact, decode_source_input_name,
    native_artifact::{NativeMatrixPrimitive, lower_matrix_artifact},
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::collections::BTreeMap;

fn artifacts(source: &str, interactive: bool) -> [ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4d4154504c414e), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let product = if interactive {
        CanonicalSourceFrontend
            .compile_interactive_document_with_catalog(&document, mech_stdlib::source_catalog())
    } else {
        CanonicalSourceFrontend
            .compile_document_with_catalog(&document, mech_stdlib::source_catalog())
    }
    .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    let original = product.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    assert_eq!(
        bytes,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    [original, decoded]
}

#[test]
fn ordinary_transpose_product_instantiates_actual_descriptors_for_each_current_geometry() {
    for artifact in artifacts("left<[f64]>' ** right<[f64]>\n", false) {
        let plan = lower_matrix_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(plan.program_revision(), artifact.revision());
        assert_eq!(
            plan.calls()
                .iter()
                .map(|call| call.template().primitive())
                .collect::<Vec<_>>(),
            [
                NativeMatrixPrimitive::TransposeF64,
                NativeMatrixPrimitive::MatmulF64
            ]
        );
        for (left, right, output) in [
            ([2, 3], [2, 4], [3, 4]),
            ([0, 2], [0, 4], [2, 4]),
            ([3, 0], [3, 4], [0, 4]),
            ([2, 3], [2, 0], [3, 0]),
        ] {
            let mut values = BTreeMap::new();
            for input in plan.inputs() {
                let declaration = &artifact.inputs()[input.input().get() as usize];
                assert_eq!(input.slot(), declaration.slot);
                assert_eq!(input.name(), declaration.name);
                let name = decode_source_input_name(input.name()).unwrap();
                let extents = match name.as_str() {
                    "left" => left,
                    "right" => right,
                    _ => panic!("unexpected real input {name}"),
                };
                let schema = plan.values()[input.value().get() as usize].schema();
                let shape = mech_core::shape_for_resolved_extents(schema, &extents).unwrap();
                values.insert(
                    input.value(),
                    ResolvedValueDescriptor::from_schema(schema.clone(), shape).unwrap(),
                );
            }
            for (ordinal, call) in plan.calls().iter().enumerate() {
                assert_eq!(call.template().node(), artifact.nodes()[ordinal].node);
                let inputs = call
                    .operands()
                    .iter()
                    .map(|input| values[input].clone())
                    .collect::<Vec<_>>();
                let memory = plan.instantiate_call(ordinal, &inputs).unwrap();
                assert_eq!(memory.bound_call.target(), ExecutionTarget::Native);
                assert_eq!(memory.inputs.len(), inputs.len());
                for (planned, actual) in memory.inputs.iter().zip(&inputs) {
                    assert_eq!(&planned.descriptor, actual);
                }
                values.insert(call.output(), memory.outputs[0].descriptor.clone());
            }
            assert_eq!(plan.outputs().len(), artifact.outputs().len());
            assert_eq!(
                values[&plan.outputs()[0].value()]
                    .current_extents()
                    .unwrap()
                    .as_ref(),
                &output
            );
        }
    }
}

#[test]
fn actual_interactive_alias_identity_and_shared_producer_are_preserved() {
    for artifact in artifacts(
        "transposed := input<[f64]>'\nalias := transposed\nalias\n",
        true,
    ) {
        let plan = lower_matrix_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(plan.calls().len(), 1);
        assert_eq!(plan.outputs().len(), artifact.outputs().len());
        assert!(
            plan.outputs().len() >= 2,
            "real named aliases must remain exported"
        );
        for (actual, declared) in plan.outputs().iter().zip(artifact.outputs()) {
            assert_eq!(actual.output(), declared.output);
            assert_eq!(actual.slot(), declared.source);
            assert_eq!(actual.name(), declared.name);
            assert_eq!(
                actual.interactive_binding(),
                declared.interactive_binding.as_ref()
            );
            assert_eq!(actual.value(), plan.calls()[0].output());
            let ty = &plan.values()[actual.value().get() as usize];
            assert_eq!(ty.schema_id(), declared.schema);
            assert_eq!(
                ty.schema_key(),
                artifact.schemas().entry(declared.schema).unwrap().key()
            );
        }
        assert!(
            plan.outputs()
                .iter()
                .any(|output| output.interactive_binding().is_some())
        );
    }
}

#[test]
fn an_unused_checked_matrix_call_is_not_removed_from_the_native_plan() {
    for artifact in artifacts(
        "unused := left<[f64]> ** right<[f64]>\ninput<[f64]>'\n",
        false,
    ) {
        let plan = lower_matrix_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(artifact.nodes().len(), 2);
        assert_eq!(plan.calls().len(), 2);
        assert_eq!(
            plan.calls()[0].template().primitive(),
            NativeMatrixPrimitive::MatmulF64
        );
        assert_eq!(
            plan.calls()[1].template().primitive(),
            NativeMatrixPrimitive::TransposeF64
        );
        assert_eq!(
            plan.calls()
                .iter()
                .map(|call| call.template().node())
                .collect::<Vec<_>>(),
            artifact
                .nodes()
                .iter()
                .map(|node| node.node)
                .collect::<Vec<_>>()
        );
    }
}
