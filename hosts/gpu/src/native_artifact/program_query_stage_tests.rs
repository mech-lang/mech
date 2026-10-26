//! Constructor-only association negatives; no generated callback is replaced.
use super::*;
use crate::native_artifact::{JitProgramArtifact, emit_program_object};
use mech_core::FunctionCatalog;
use mech_engine::native_artifact::{
    NativeArtifactLoweringError, NativeCollectionResult, NativeHistogramQueryStepKind as Step,
};
use mech_engine::{ProgramArtifact, native_artifact::prepare_native_program_library_call};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::atomic::{AtomicUsize, Ordering};

const SOURCE: &str = "+> map\nlookup(items<[string]>, query<string>) => (out<u64>) :=\n  words := [part | part <- items]\n  counts := map/histogram(words)\n  keys<{string}> := counts_keys\n  count := (query ∈ keys) ? | true => counts{query} | false => 0u64.\n  out := count.\nfirst := lookup(source<[string]>, query<string>)\nsecond := lookup(source, otherquery<string>)\nfirst + second\n";
fn artifacts() -> [ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x5153544147), Revision(1), SOURCE).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let original = mech_engine::CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}
fn calls(
    artifact: &ProgramArtifact,
    descriptor: &NativeProgramDescriptor,
    catalog: &FunctionCatalog,
) -> Vec<Option<NativePreparedLibraryCall>> {
    descriptor
        .metadata()
        .calls
        .iter()
        .map(|call| {
            call.as_ref().map(|call| {
                prepare_native_program_library_call(artifact, call.template(), catalog).unwrap()
            })
        })
        .collect()
}
#[test]
fn query_stage_constructor_and_link_rederive_exact_associations_and_reject_corruption() {
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts() {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let descriptor = jit.descriptor();
        let sinks = descriptor
            .metadata()
            .plan
            .collections()
            .iter()
            .enumerate()
            .filter_map(|(id, c)| match c.result() {
                NativeCollectionResult::GuardedHistogramQuery(sink) => Some((id as u32, sink)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            sinks.len(),
            2,
            "both actual root lookup bodies must select fused queries"
        );
        let (id, sink) = sinks[0];
        validate_query_stages(&descriptor.metadata().plan, &descriptor.metadata().calls).unwrap();
        for &(collection, query) in &sinks {
            for (ordinal, step) in query.steps().iter().enumerate() {
                descriptor.query_stage(collection, ordinal as u32).unwrap();
                assert_eq!(
                    descriptor.metadata().calls[step.site() as usize]
                        .as_ref()
                        .map(|c| c.template()),
                    step.template()
                );
            }
        }
        assert!(descriptor.query_stage(u32::MAX, 0).is_err());
        assert!(descriptor.query_stage(id, u32::MAX).is_err());
        let key = sink
            .steps()
            .iter()
            .find(|s| s.kind() == Step::Keys)
            .unwrap()
            .site() as usize;
        let membership = sink
            .steps()
            .iter()
            .find(|s| s.kind() == Step::Membership)
            .unwrap()
            .site() as usize;
        let cleanup = sink
            .steps()
            .iter()
            .find(|s| s.kind() == Step::Cleanup)
            .unwrap()
            .site() as usize;
        let mut missing = calls(&artifact, descriptor, &catalog);
        missing[key] = None;
        assert!(validate_query_stages(&descriptor.metadata().plan, &missing).is_err());
        let mut swapped = calls(&artifact, descriptor, &catalog);
        swapped.swap(key, membership);
        assert!(validate_query_stages(&descriptor.metadata().plan, &swapped).is_err());
        // Presence mismatch is separate from the same-kind foreign-origin case.
        let mut cleanup_presence = calls(&artifact, descriptor, &catalog);
        cleanup_presence[cleanup] = Some(
            prepare_native_program_library_call(
                &artifact,
                descriptor.metadata().calls[key]
                    .as_ref()
                    .unwrap()
                    .template(),
                &catalog,
            )
            .unwrap(),
        );
        assert!(validate_query_stages(&descriptor.metadata().plan, &cleanup_presence).is_err());
        let original_keys = sink
            .steps()
            .iter()
            .find(|s| s.kind() == Step::Keys)
            .unwrap()
            .template()
            .unwrap();
        let foreign_keys = sinks[1]
            .1
            .steps()
            .iter()
            .find(|s| s.kind() == Step::Keys)
            .unwrap()
            .template()
            .unwrap();
        assert_ne!(original_keys.origin(), foreign_keys.origin());
        assert_eq!(
            original_keys.program_revision(),
            foreign_keys.program_revision()
        );
        assert_eq!(original_keys.primitive(), foreign_keys.primitive());
        assert_eq!(original_keys.selected_key(), foreign_keys.selected_key());
        assert_eq!(original_keys.inputs(), foreign_keys.inputs());
        assert_eq!(original_keys.output(), foreign_keys.output());
        assert_eq!(original_keys.contract(), foreign_keys.contract());
        assert_eq!(
            original_keys.implementation_memory(),
            foreign_keys.implementation_memory()
        );
        // This is the OTHER actual selected Keys call in the SAME artifact,
        // not a mutated template, renamed operation or fabricated call origin.
        let mut wrong_origin = calls(&artifact, descriptor, &catalog);
        wrong_origin[key] =
            Some(prepare_native_program_library_call(&artifact, foreign_keys, &catalog).unwrap());
        assert!(validate_query_stages(&descriptor.metadata().plan, &wrong_origin).is_err());
        assert!(
            validate_query_stages(
                &descriptor.metadata().plan,
                &descriptor.metadata().calls[..descriptor.metadata().calls.len() - 1]
            )
            .is_err()
        );

        let object = emit_program_object(&artifact, &catalog).unwrap();
        let linked = crate::native_artifact::program_linked::prepare_descriptor(
            &artifact,
            &catalog,
            object.identity().bytes(),
        )
        .unwrap();
        validate_query_stages(&linked.metadata().plan, &linked.metadata().calls).unwrap();
        for &(collection, query) in &sinks {
            for ordinal in 0..query.steps().len() {
                linked.query_stage(collection, ordinal as u32).unwrap();
            }
        }
        assert!(crate::native_artifact::NATIVE_PROGRAM_WORK_POLICY_VERSION >= 17);
        let mut old = *object.identity().bytes();
        old[32..40].copy_from_slice(&16u64.to_le_bytes());
        assert!(
            crate::native_artifact::program_linked::prepare_descriptor(&artifact, &catalog, &old)
                .is_err()
        );
    }
}

static BINDS: AtomicUsize = AtomicUsize::new(0);
fn foreign(
    _: &mech_core::ResidentKernelBindRequest<'_>,
) -> Result<mech_core::BoundResidentKernel, mech_core::ResidentKernelBindError> {
    BINDS.fetch_add(1, Ordering::SeqCst);
    Err(mech_core::ResidentKernelBindError::UnsupportedLayout)
}
#[test]
fn query_stage_preparation_rejects_factory_key_and_memory_replacements_before_binding() {
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts() {
        let object = emit_program_object(&artifact, &catalog).unwrap();
        let descriptor = object.descriptor();
        for step in descriptor
            .metadata()
            .plan
            .collections()
            .iter()
            .filter_map(|c| match c.result() {
                NativeCollectionResult::GuardedHistogramQuery(sink) => Some(sink),
                _ => None,
            })
            .flat_map(|sink| sink.steps())
            .filter_map(|step| step.template())
        {
            let key = step.selected_key();
            let original = catalog
                .resident_factory(&key.module_path, &key.operation_name)
                .unwrap();
            for altered in [
                original.clone().with_factory(original.factory()),
                original.clone().with_key(original.key().clone()),
                original
                    .clone()
                    .with_implementation_memory(original.implementation_memory()),
                original.clone().with_factory(foreign),
            ] {
                // Keep ALL other real registrations, so an unrelated missing
                // factory cannot masquerade as rejection of this selected site.
                let mut builder = mech_core::FunctionCatalogBuilder::new();
                for (candidate, entry) in catalog.resident_factories() {
                    builder
                        .insert_resident_factory_entry(if candidate == key {
                            altered.clone()
                        } else {
                            entry.clone()
                        })
                        .unwrap();
                }
                let replaced = builder.build().unwrap();
                assert!(matches!(
                    prepare_native_program_library_call(&artifact, step, &replaced),
                    Err(NativeArtifactLoweringError::MissingSemanticWitness { .. })
                ));
                assert!(
                    crate::native_artifact::program::prepare::PreparedNativeProgram::prepare(
                        &artifact,
                        &replaced,
                        descriptor.metadata().plan.clone()
                    )
                    .map(|prepared| NativeProgramDescriptor::from_prepared(
                        prepared,
                        crate::native_artifact::program_descriptor::NativeCodeFootprint {
                            stack_bytes: descriptor.stack_bytes(),
                            code_bytes: descriptor.code_bytes()
                        }
                    ))
                    .is_err()
                );
                assert!(JitProgramArtifact::compile(&artifact, &replaced).is_err());
                assert!(emit_program_object(&artifact, &replaced).is_err());
                assert!(
                    crate::native_artifact::program_linked::prepare_descriptor(
                        &artifact,
                        &replaced,
                        object.identity().bytes(),
                    )
                    .is_err()
                );
            }
        }
    }
    assert_eq!(BINDS.load(Ordering::SeqCst), 0);
}
