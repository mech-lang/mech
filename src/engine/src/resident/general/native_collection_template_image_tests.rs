use super::*;
use mech_core::{FunctionCatalogBuilder, ResidentKernelFactoryEntry};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn fixture() -> (
    ProgramArtifact,
    FunctionCatalog,
    NativeCollectionOperationTemplate,
) {
    let source = "lines := text/lines(input<string>)\nsequence/length(lines)\n";
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x494d4147), Revision(0), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean());
    let artifact = crate::CanonicalSourceFrontend
        .compile_document(&DocumentSyntax::cast(parsed.syntax()).unwrap())
        .unwrap()
        .compile_artifact()
        .unwrap();
    let mut builder = FunctionCatalogBuilder::new();
    crate::install_intrinsic_resident(&mut builder).unwrap();
    let catalog = builder.build().unwrap();
    let template = artifact
        .nodes()
        .iter()
        .find_map(|node| {
            native_collection_operation_template(&artifact, node.node, &catalog)
                .ok()
                .filter(|template| template.primitive() == NativeCollectionPrimitive::TextLines)
        })
        .unwrap();
    (artifact, catalog, template)
}

#[test]
fn build_owned_template_image_roundtrips_with_the_actual_factory() {
    let (artifact, catalog, template) = fixture();
    let image = serde_json::to_value(&template).unwrap();
    // SAFETY: unchanged encoding of this checked fixture's actual selected
    // template; returned metadata stays in the test and is never executed.
    let loaded = unsafe {
        NativeCollectionOperationTemplate::from_build_image(&image, &catalog, artifact.schemas())
    }
    .unwrap();
    assert_eq!(loaded, template);
    let outer = crate::native_artifact::NativeProgramLibraryTemplate::from_root(template.clone());
    let outer_image = serde_json::to_value(&outer).unwrap();
    // Keep the preexisting wire contract independently of its compact private
    // representation, including every redundant consistency-check field.
    let expected_outer = serde_json::json!({
        "origin": {"node": template.node(), "block": null, "local": null},
        "revision": template.program_revision(),
        "inputs": template.inputs(),
        "output": template.output(),
        "contract": template.contract(),
        "selected_key": template.selected_key(),
        "selected": {"Root": &template},
    });
    assert_eq!(outer_image, expected_outer);
    // SAFETY: same checked fixture; metadata is inspected only, never executed.
    let loaded = unsafe {
        crate::native_artifact::program_image_read::library::template(
            &outer_image,
            &catalog,
            artifact.schemas(),
        )
    }
    .unwrap();
    assert_eq!(loaded, outer);
    for key in ["inputs", "output", "contract", "selected_key", "revision"] {
        let mut missing = outer_image.clone();
        missing.as_object_mut().unwrap().remove(key);
        let mut extra = outer_image.clone();
        extra
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), true.into());
        let mut extra_tag = outer_image.clone();
        extra_tag["selected"]
            .as_object_mut()
            .unwrap()
            .insert("Lexical".into(), false.into());
        for changed in [&missing, &extra, &extra_tag] {
            // SAFETY: rejection probes stay confined to this test.
            assert!(
                unsafe {
                    crate::native_artifact::program_image_read::library::template(
                        changed,
                        &catalog,
                        artifact.schemas(),
                    )
                }
                .is_err()
            );
        }
    }
    for (pointer, replacement) in [
        (
            "/revision/0",
            serde_json::json!(outer_image["revision"][0].as_u64().unwrap() ^ 1),
        ),
        (
            "/selected_key/operation_name",
            serde_json::json!("invalid-image-operation"),
        ),
        ("/inputs/0/body", serde_json::json!("Bool")),
        ("/output/body", serde_json::json!("Bool")),
        (
            "/contract/Declared/inputs/0/schema",
            serde_json::json!(u32::MAX),
        ),
    ] {
        let mut changed = outer_image.clone();
        *changed.pointer_mut(pointer).unwrap() = replacement;
        // SAFETY: a rejection probe; no restored metadata reaches execution.
        assert!(
            unsafe {
                crate::native_artifact::program_image_read::library::template(
                    &changed,
                    &catalog,
                    artifact.schemas(),
                )
            }
            .is_err(),
            "{pointer}"
        );
    }
    let mut changed = outer_image.clone();
    changed["origin"].as_object_mut().unwrap().remove("block");
    // SAFETY: missing-field rejection probe only.
    assert!(
        unsafe {
            crate::native_artifact::program_image_read::library::template(
                &changed,
                &catalog,
                artifact.schemas(),
            )
        }
        .is_err()
    );
    let mut changed = outer_image.clone();
    changed
        .pointer_mut("/selected/Root/leaf/contract/Declared/inputs/0")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert("unexpected".into(), true.into());
    // SAFETY: nested unknown-field rejection probe only.
    assert!(
        unsafe {
            crate::native_artifact::program_image_read::library::template(
                &changed,
                &catalog,
                artifact.schemas(),
            )
        }
        .is_err()
    );
    let mut changed = outer_image;
    changed["origin"]["node"] = 999.into();
    // SAFETY: rejection probe; no returned proof escapes to execution.
    assert!(
        unsafe {
            crate::native_artifact::program_image_read::library::template(
                &changed,
                &catalog,
                artifact.schemas(),
            )
        }
        .is_err()
    );
    let decoded = crate::decode_program_artifact_bytecode_v1(
        &crate::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    let selected =
        native_collection_operation_template(&decoded, template.node(), &catalog).unwrap();
    assert_eq!(serde_json::to_value(selected).unwrap(), image);
}

#[test]
fn build_owned_template_image_rejects_missing_or_forged_factory_and_memory_class() {
    let (artifact, catalog, template) = fixture();
    let image = serde_json::to_value(&template).unwrap();
    let empty = FunctionCatalogBuilder::new().build().unwrap();
    let entry = catalog
        .resident_factory(
            &template.selected_key().module_path,
            &template.selected_key().operation_name,
        )
        .unwrap();
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_resident_factory_entry(ResidentKernelFactoryEntry::new(
            entry.key().clone(),
            entry.implementation_memory(),
            entry.factory(),
        ))
        .unwrap();
    let forged = builder.build().unwrap();
    for registry in [&empty, &forged] {
        // SAFETY: rejection probe only; no returned metadata can escape this test.
        assert!(
            unsafe {
                NativeCollectionOperationTemplate::from_build_image(
                    &image,
                    registry,
                    artifact.schemas(),
                )
            }
            .is_err()
        );
    }
    let mut changed = image;
    changed["leaf"]["implementation_memory"] = serde_json::json!("MatrixSolve");
    // SAFETY: rejection probe only, never an execution descriptor.
    assert!(
        unsafe {
            NativeCollectionOperationTemplate::from_build_image(
                &changed,
                &catalog,
                artifact.schemas(),
            )
        }
        .is_err()
    );
}
