//! Selected-factory negatives for a real generic String/map projection.
//! These prepare metadata only: no foreign entry, callback or leaf is executed.
use crate::native_artifact::program::image::*;
use crate::native_artifact::{JitProgramArtifact, emit_program_object};
use mech_engine::ProgramArtifact;
use mech_engine::native_artifact::{
    NativeArtifactLoweringError, NativeCollectionPrimitive, NativeIterationAction,
    prepare_native_program_library_call,
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::atomic::{AtomicUsize, Ordering};

const SOURCE: &str = "+> text\n+> map\ntext/join([(entries<{string:string}>){piece} | piece <- source<string>], separator<string>)\n";

fn artifacts() -> [ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x534d464143), Revision(1), SOURCE).unwrap(),
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

static FOREIGN_BINDS: AtomicUsize = AtomicUsize::new(0);
fn foreign(
    _: &mech_core::ResidentKernelBindRequest<'_>,
) -> Result<mech_core::BoundResidentKernel, mech_core::ResidentKernelBindError> {
    FOREIGN_BINDS.fetch_add(1, Ordering::SeqCst);
    Err(mech_core::ResidentKernelBindError::UnsupportedLayout)
}

#[test]
fn string_map_projection_rejects_same_pointer_and_foreign_factories_before_binding() {
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts() {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let object = emit_program_object(&artifact, &catalog).unwrap();
        let descriptor = object.descriptor();
        let projections = descriptor
            .metadata()
            .plan
            .collections()
            .iter()
            .enumerate()
            .filter_map(|(id, collection)| {
                collection
                    .string_element_map_projection()
                    .map(|projection| (id, projection))
            })
            .collect::<Vec<_>>();
        assert_eq!(
            projections.len(),
            1,
            "the actual generic source must select the projection"
        );
        let (id, projection) = projections[0];
        let site = projection.selection_site() as usize;
        let template = projection.selection_template();
        assert_eq!(
            template.primitive(),
            NativeCollectionPrimitive::MapKeyAccess
        );
        assert!(matches!(descriptor.metadata().plan.blocks()[site].action(),
            NativeIterationAction::ProjectStringMapEntry { collection } if *collection as usize == id));
        assert_eq!(
            descriptor.metadata().calls[site]
                .as_ref()
                .unwrap()
                .template(),
            template
        );
        assert_eq!(
            descriptor.typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );

        // Valid original-catalog controls isolate substitution rejection from
        // missing unrelated registrations or an unsupported source fixture.
        prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
        crate::native_artifact::program::prepare::PreparedNativeProgram::prepare(
            &artifact,
            &catalog,
            descriptor.metadata().plan.clone(),
        )
        .map(|prepared| {
            NativeProgramDescriptor::from_prepared(
                prepared,
                crate::native_artifact::program::image::NativeCodeFootprint {
                    stack_bytes: descriptor.stack_bytes(),
                    code_bytes: descriptor.code_bytes(),
                },
            )
        })
        .unwrap();
        let linked = crate::native_artifact::program::linked::prepare_descriptor(
            &artifact,
            &catalog,
            object.identity().bytes(),
        )
        .unwrap();
        assert_eq!(linked.typed_ir_sha256(), descriptor.typed_ir_sha256());

        let key = template.selected_key();
        let original = catalog
            .resident_factory(&key.module_path, &key.operation_name)
            .unwrap();
        for (label, altered, same_pointer) in [
            (
                "same-factory-pointer",
                original.clone().with_factory(original.factory()),
                true,
            ),
            (
                "same-operation-key",
                original.clone().with_key(original.key().clone()),
                true,
            ),
            (
                "same-memory-class",
                original
                    .clone()
                    .with_implementation_memory(original.implementation_memory()),
                true,
            ),
            (
                "foreign-binder",
                original.clone().with_factory(foreign),
                false,
            ),
        ] {
            assert_eq!(altered.key(), original.key(), "{label}");
            assert_eq!(
                altered.implementation_memory(),
                original.implementation_memory(),
                "{label}"
            );
            if same_pointer {
                assert!(
                    std::ptr::fn_addr_eq(altered.factory(), original.factory()),
                    "{label}"
                );
            }
            // These public replacements deliberately strip the private witness.
            // Keep EVERY other actual resident entry, not just the chosen key.
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
            assert_eq!(
                replaced.resident_factories().count(),
                catalog.resident_factories().count()
            );
            assert!(
                matches!(
                    prepare_native_program_library_call(&artifact, template, &replaced),
                    Err(NativeArtifactLoweringError::MissingSemanticWitness { node })
                        if node == template.origin().node()
                ),
                "{label}: direct selected preparation accepted an uncertified factory"
            );
            assert!(
                crate::native_artifact::program::prepare::PreparedNativeProgram::prepare(
                    &artifact,
                    &replaced,
                    descriptor.metadata().plan.clone()
                )
                .map(|prepared| NativeProgramDescriptor::from_prepared(
                    prepared,
                    crate::native_artifact::program::image::NativeCodeFootprint {
                        stack_bytes: descriptor.stack_bytes(),
                        code_bytes: descriptor.code_bytes()
                    }
                ))
                .is_err(),
                "{label}: descriptor accepted an uncertified factory"
            );
            assert!(
                JitProgramArtifact::compile(&artifact, &replaced).is_err(),
                "{label}: JIT accepted"
            );
            assert!(
                emit_program_object(&artifact, &replaced).is_err(),
                "{label}: Object accepted"
            );
            assert!(
                crate::native_artifact::program::linked::prepare_descriptor(
                    &artifact,
                    &replaced,
                    object.identity().bytes(),
                )
                .is_err(),
                "{label}: linked metadata reconstruction accepted"
            );
            assert_eq!(
                FOREIGN_BINDS.load(Ordering::SeqCst),
                0,
                "{label}: binder was entered"
            );
        }

        // Rejection must neither mutate the original catalog nor invalidate its
        // same-artifact Object metadata. This is not linked code execution.
        JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        emit_program_object(&artifact, &catalog).unwrap();
        crate::native_artifact::program::linked::prepare_descriptor(
            &artifact,
            &catalog,
            object.identity().bytes(),
        )
        .unwrap();
    }
    assert_eq!(FOREIGN_BINDS.load(Ordering::SeqCst), 0);
}
