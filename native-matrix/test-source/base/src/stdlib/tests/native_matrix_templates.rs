#![cfg(all(feature = "full_compiler", feature = "native-artifact"))]
use mech_core::{
    BoundResidentKernel, FunctionCatalog, FunctionCatalogBuilder, ImplementationMemoryClass,
    ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelFactoryEntry, SchemaBody,
    ShapeRule,
};
use mech_engine::native_artifact::{
    NativeMatrixPrimitive, NativeMatrixTemplateError, matrix_operation_templates,
};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::Arc;

fn compile(source: &str) -> ProgramArtifact {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4d4154524958), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"))
        .compile_artifact()
        .unwrap()
}

fn artifacts(source: &str) -> [ProgramArtifact; 2] {
    let original = compile(source);
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    [original, decoded]
}

fn forbidden_factory(
    _: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    panic!("symbolic template certification must not invoke a binder or executor")
}

fn with_entry(entry: ResidentKernelFactoryEntry) -> FunctionCatalog {
    let mut builder = FunctionCatalogBuilder::new();
    builder.insert_resident_factory_entry(entry).unwrap();
    builder.build().unwrap()
}

#[test]
fn dynamic_original_and_decoded_transpose_product_keep_symbolic_schemas_and_source_order() {
    for artifact in artifacts("left<[f64]>' ** right<[f64]>\n") {
        let templates =
            matrix_operation_templates(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(templates.len(), 2);
        assert_eq!(
            templates
                .iter()
                .map(|template| template.primitive())
                .collect::<Vec<_>>(),
            [
                NativeMatrixPrimitive::TransposeF64,
                NativeMatrixPrimitive::MatmulF64
            ]
        );
        assert_eq!(
            templates
                .iter()
                .map(|template| template.node())
                .collect::<Vec<_>>(),
            artifact
                .nodes()
                .iter()
                .map(|node| node.node)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            templates[0].shape_rule(),
            ShapeRule::TransposeOf { input: 0 }
        );
        assert_eq!(
            templates[1].shape_rule(),
            ShapeRule::MatrixProduct { lhs: 0, rhs: 1 }
        );
        for template in &templates {
            assert_eq!(template.program_revision(), artifact.revision());
            assert!(!template.output().dimension_parameters().is_empty());
            for schema in template
                .inputs()
                .iter()
                .chain(std::iter::once(template.output()))
            {
                assert!(
                    matches!(schema.body(), SchemaBody::Matrix { element, dimensions }
                    if element.as_ref() == &SchemaBody::FloatingPoint(mech_core::FloatWidth::W64) && dimensions.len() == 2)
                );
            }
            assert_eq!(template.sources().len(), template.inputs().len());
        }
    }
}

#[test]
fn unchanged_genuine_entry_and_catalog_delegation_preserve_authority_without_binding() {
    let maintained = mech_stdlib::source_catalog();
    for (source, item, primitive) in [
        (
            "input<[f64]>'\n",
            "transpose",
            NativeMatrixPrimitive::TransposeF64,
        ),
        (
            "left<[f64]> ** right<[f64]>\n",
            "matmul",
            NativeMatrixPrimitive::MatmulF64,
        ),
    ] {
        let entry = maintained
            .resident_factory(&["matrix".into()], item)
            .unwrap()
            .clone();
        let catalog = with_entry(entry);
        let cloned = FunctionCatalogBuilder::from_catalog(&catalog)
            .build()
            .unwrap();
        for artifact in artifacts(source) {
            let templates = matrix_operation_templates(&artifact, &cloned).unwrap();
            assert_eq!(templates.len(), 1);
            assert_eq!(templates[0].primitive(), primitive);
            assert_eq!(templates[0].selected_key().operation_name, item);
        }
    }
}

#[test]
fn same_name_public_marker_and_all_changed_delegates_are_not_certified() {
    let maintained = mech_stdlib::source_catalog();
    let genuine = maintained
        .resident_factory(&["matrix".into()], "matmul")
        .unwrap();
    let arbitrary = ResidentKernelFactoryEntry::new(
        genuine.key().clone(),
        genuine.implementation_memory(),
        forbidden_factory,
    )
    .with_lowering_witness(Arc::new(NativeMatrixPrimitive::MatmulF64));
    for entry in [
        arbitrary,
        genuine.clone().with_factory(forbidden_factory),
        genuine.clone().with_factory(genuine.factory()),
        genuine.clone().with_key(genuine.key().clone()),
        genuine
            .clone()
            .with_implementation_memory(genuine.implementation_memory()),
        genuine
            .clone()
            .with_implementation_memory(ImplementationMemoryClass::CanonicalFinalize),
    ] {
        let catalog = with_entry(entry);
        for artifact in artifacts("left<[f64]> ** right<[f64]>\n") {
            let node = artifact.nodes()[0].node;
            assert_eq!(
                matrix_operation_templates(&artifact, &catalog).unwrap_err(),
                NativeMatrixTemplateError::MissingImplementationCertificate { node }
            );
        }
    }
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_resident_factory(
            ["matrix"],
            "matmul",
            genuine.implementation_memory(),
            forbidden_factory,
        )
        .unwrap();
    let catalog = builder.build().unwrap();
    let artifact = compile("left<[f64]> ** right<[f64]>\n");
    assert!(matches!(
        matrix_operation_templates(&artifact, &catalog),
        Err(NativeMatrixTemplateError::MissingImplementationCertificate { .. })
    ));
}

#[test]
fn non_f64_nested_collections_control_and_scalar_paths_are_rejected_without_fallback() {
    for source in [
        "left<[u64]> ** right<[u64]>\n",
        "left<f64> + right<f64>\n",
        "text<string>\n",
        "[value | value <- input<[f64]>]\n",
        "flag<bool> ? | true => left<[f64]:1,2> | false => right<[f64]:1,2>\n",
    ] {
        for artifact in artifacts(source) {
            assert!(
                matrix_operation_templates(&artifact, &mech_stdlib::source_catalog()).is_err(),
                "{source}"
            );
        }
    }
}
