//! Exact registration selection must not change the canonical program.
use super::*;
use mech_core::{FunctionCatalog, FunctionCatalogBuilder, ResidentKernelFactoryEntry};
use mech_engine::linked_factories::NativeFactoryLinkageProfile;
use mech_gpu::native_artifact::{emit_program_object, generate_linked_hosted_program_application};
use sha2::{Digest, Sha256};

fn selected(artifact: &mech_engine::ProgramArtifact, full: &FunctionCatalog) -> FunctionCatalog {
    let profile = NativeFactoryLinkageProfile::derive(artifact, full);
    assert_eq!(profile.operations(), artifact.kernel_operation_references());
    let factories = profile.factories().unwrap_or_else(|| {
        panic!(
            "missing maintained linkage: {:?}",
            profile.missing_operations()
        )
    });
    let mut builder = FunctionCatalogBuilder::new();
    for factory in factories {
        factory.install(&mut builder).unwrap();
    }
    let catalog = builder.build().unwrap();
    assert_eq!(catalog.resident_factory_count(), profile.operations().len());
    assert!(catalog.resident_factory_count() < full.resident_factory_count());
    for (key, entry) in catalog.resident_factories() {
        let original = full
            .resident_factory(&key.module_path, &key.operation_name)
            .unwrap();
        assert_eq!(
            entry.implementation_memory(),
            original.implementation_memory()
        );
        assert!(std::ptr::fn_addr_eq(entry.factory(), original.factory()));
    }
    catalog
}

#[test]
fn native_image_factory_closure_restores_every_template_and_rejects_missing_witnesses() {
    use std::collections::BTreeSet;
    fn image_operations(
        value: &serde_json::Value,
        operations: &mut BTreeSet<mech_engine::OperationReference>,
    ) {
        if let Some(key) = value.get("selected_key") {
            operations.insert(mech_engine::OperationReference {
                module_path: key["module_path"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|part| part.as_str().unwrap().to_owned())
                    .collect(),
                operation_name: key["operation_name"].as_str().unwrap().to_owned(),
            });
        }
        match value {
            serde_json::Value::Array(values) => {
                for value in values {
                    image_operations(value, operations);
                }
            }
            serde_json::Value::Object(fields) => {
                for value in fields.values() {
                    image_operations(value, operations);
                }
            }
            _ => {}
        }
    }
    for source in [FULL, ECHO, CLOSED_FAIL, CLOSED_LIVE, FAIL] {
        let (variants, full) = artifacts(source);
        for artifact in variants {
            // Compilation still checks the full original catalog, before any
            // packaging reduction. No code or source requirement is replaced.
            let object = emit_program_object(&artifact, &full).unwrap();
            let descriptor = object.descriptor();
            let bytes = descriptor.encode_build_owned_image().unwrap();
            let profile =
                NativeFactoryLinkageProfile::derive_native_image(descriptor.plan(), &full);
            let mut expected = BTreeSet::new();
            image_operations(&serde_json::from_slice(&bytes).unwrap(), &mut expected);
            assert_eq!(
                profile.operations(),
                expected.into_iter().collect::<Vec<_>>()
            );
            let mut builder = FunctionCatalogBuilder::new();
            for factory in profile.factories().unwrap() {
                factory.install(&mut builder).unwrap();
            }
            let catalog = builder.build().unwrap();
            assert_eq!(catalog.resident_factory_count(), profile.operations().len());
            // SAFETY: unchanged image and catalog selected from this same checked
            // Object; restoration is inspected only, never executed by this test.
            let restored = unsafe {
                mech_gpu::native_artifact::NativeProgramDescriptor::decode_build_owned_image(
                    &bytes, &catalog,
                )
            }
            .unwrap();
            assert_eq!(restored.encode_build_owned_image().unwrap(), bytes);
            if let Some(operation) = profile.operations().first() {
                let entry = catalog
                    .resident_factory(&operation.module_path, &operation.operation_name)
                    .unwrap();
                // Even an identical public factory replacement cannot forge the
                // maintained owner's private witness used by the image loader.
                let forged = replaced_catalog(
                    &catalog,
                    operation,
                    entry.clone().with_factory(entry.factory()),
                );
                let bad =
                    NativeFactoryLinkageProfile::derive_native_image(descriptor.plan(), &forged);
                assert!(bad.factories().is_none());
                assert_eq!(bad.missing_operations(), &[operation.clone()]);
                // SAFETY: rejection probe only; no metadata escapes to execution.
                assert!(unsafe {
                    mech_gpu::native_artifact::NativeProgramDescriptor::decode_build_owned_image(&bytes, &forged)
                }.is_err());
            }
        }
    }
}

#[test]
fn complete_kernel_closure_selected_factories_preserve_original_and_decoded_objects() {
    // Includes closed activation failures, nested FSM/control bodies and
    // operations that will not execute on the successful input below.
    for source in [ECHO, FULL, CLOSED_LIVE, CLOSED_FAIL, FAIL] {
        let (variants, full) = artifacts(source);
        for artifact in variants {
            let catalog = selected(&artifact, &full);
            let original = emit_program_object(&artifact, &full).unwrap();
            let linked = emit_program_object(&artifact, &catalog).unwrap();
            assert_eq!(original.bytes(), linked.bytes());
            assert_eq!(original.identity().bytes(), linked.identity().bytes());
            assert_eq!(original.metadata(), linked.metadata());
        }
    }
}

fn replaced_catalog(
    full: &FunctionCatalog,
    operation: &mech_engine::OperationReference,
    replacement: ResidentKernelFactoryEntry,
) -> FunctionCatalog {
    let mut builder = FunctionCatalogBuilder::new();
    for (key, entry) in full.resident_factories() {
        let replace = key.module_path.as_ref() == operation.module_path.as_ref()
            && key.operation_name == operation.operation_name;
        builder
            .insert_resident_factory_entry(if replace {
                replacement.clone()
            } else {
                entry.clone()
            })
            .unwrap();
    }
    builder.build().unwrap()
}

#[test]
fn public_factory_replacements_cannot_reuse_private_linkage_evidence() {
    let (variants, full) = artifacts(FULL);
    for artifact in variants {
        for operation in artifact.kernel_operation_references() {
            let entry = full
                .resident_factory(&operation.module_path, &operation.operation_name)
                .unwrap();
            // Same key/cost/function replacements ALSO invalidate evidence.
            // A public descriptive attachment cannot recreate the owner seal.
            for replacement in [
                entry.clone().with_key(entry.key().clone()),
                entry
                    .clone()
                    .with_implementation_memory(entry.implementation_memory()),
                entry.clone().with_factory(entry.factory()),
                entry
                    .clone()
                    .with_lowering_witness(Arc::new(operation.canonical_name())),
            ] {
                let catalog = replaced_catalog(&full, &operation, replacement);
                let profile = NativeFactoryLinkageProfile::derive(&artifact, &catalog);
                assert!(profile.factories().is_none());
                assert_eq!(profile.operations(), artifact.kernel_operation_references());
                assert_eq!(profile.missing_operations(), &[operation.clone()]);
            }
        }
    }
}

#[test]
fn selected_factory_program_uses_same_secured_round_and_accounts() {
    let (variants, full) = artifacts(FULL);
    for artifact in variants {
        let catalog = selected(&artifact, &full);
        let original = JitProgramArtifact::compile(&artifact, &full).unwrap();
        let linked = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let mut reports = Vec::new();
        for jit in [&original, &linked] {
            let (mut runtime, trace) = runtime(GGTA_INPUT, true);
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            reports.push(execute(&mut runtime, &artifact, jit, limits(), &memory).unwrap());
            assert_eq!(trace.lock().unwrap().output, GGTA_OUTPUT);
            assert_eq!(memory.used_bytes(), 0);
            no_resident(&runtime);
            runtime.shutdown().unwrap();

            let (mut denied, trace) = super::runtime(GGTA_INPUT, false);
            assert!(execute(&mut denied, &artifact, jit, limits(), &memory).is_err());
            quiet(&trace);
            assert_eq!(memory.used_bytes(), 0);
            no_resident(&denied);
            denied.shutdown().unwrap();
        }
        assert_eq!(reports[0].work, reports[1].work);
        assert_eq!(reports[0].comparison_work, reports[1].comparison_work);
        assert_eq!(reports[0].visits, reports[1].visits);
        assert_eq!(reports[0].transport_bytes, reports[1].transport_bytes);
    }
}

#[test]
fn generated_launcher_references_individual_factories_and_pins_profile() {
    let temporary = tempfile::tempdir().unwrap();
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let (variants, full) = artifacts(FULL);
    let mut pins = Vec::new();
    for (index, artifact) in variants.iter().enumerate() {
        let generated = generate_linked_hosted_program_application(
            &workspace,
            &temporary.path().join(format!("project-{index}")),
            "selected-canonical-factories",
            artifact,
            &full,
            FULL.as_bytes(),
        )
        .unwrap();
        let main = std::fs::read_to_string(generated.root().join("src/main.rs")).unwrap();
        assert!(!main.contains("install_intrinsic_resident("));
        let capture = main
            .find("with_prepared_canonical_native_resources_phased_text(")
            .unwrap();
        assert!(!main.contains("decode_program_artifact"));
        assert!(!main.contains("let artifact ="));
        assert!(
            main.find("LinkedProgramArtifact::from_build_owned_image(")
                .unwrap()
                < capture
        );
        assert!(
            main.find("CanonicalNativeResourceMetadata::decode_build_owned_image(")
                .unwrap()
                < capture
        );
        assert!(main.find("drop(catalog);").unwrap() < capture);
        assert!(main.contains("hex(program_revision.as_bytes())"));
        assert!(!main.contains("@INSTALL_CANONICAL_FACTORIES@"));
        assert!(!main.contains("\\\""));
        let object = emit_program_object(artifact, &full).unwrap();
        let profile =
            NativeFactoryLinkageProfile::derive_native_image(object.descriptor().plan(), &full);
        for factory in profile.factories().unwrap() {
            assert!(main.contains(&format!("{}.install(&mut builder)", factory.rust_path())));
        }
        let bytes = std::fs::read(generated.root().join("factory-profile.json")).unwrap();
        assert_eq!(
            generated.pins()["factory-profile.json"],
            Sha256::digest(&bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        let metadata: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(metadata["mode"], "artifact-selected");
        assert_eq!(metadata["selection_basis"], "native-image-libraries");
        assert_eq!(profile.operations().len(), 20);
        for unused in [
            "NUMERIC_MATH_ADD",
            "NUMERIC_MATH_SUB",
            "NUMERIC_MATH_MUL",
            "NUMERIC_MATH_DIV",
            "NUMERIC_COMPARE_GT",
            "NUMERIC_COMPARE_LT",
            "NUMERIC_COMPARE_LTE",
        ] {
            assert!(!main.contains(&format!("::{unused}.install(")), "{unused}");
        }
        let manifest = std::fs::read_to_string(generated.manifest_path())
            .unwrap()
            .parse::<toml_edit::DocumentMut>()
            .unwrap();
        assert!(manifest["dependencies"].get("mech-stdlib").is_none());
        let runtime_features = manifest["dependencies"]["mech-runtime"]["features"]
            .as_array()
            .unwrap();
        assert!(
            runtime_features
                .iter()
                .any(|feature| { feature.as_str() == Some("canonical-native-resources") })
        );
        assert!(
            !runtime_features
                .iter()
                .any(|feature| feature.as_str() == Some("resident-routing"))
        );
        let advertised = generated.metadata()["runtime_features"].as_array().unwrap();
        assert!(advertised.iter().any(|feature| {
            feature.as_str() == Some("mech-runtime/canonical-native-resources")
        }));
        assert!(
            !advertised
                .iter()
                .any(|feature| { feature.as_str() == Some("mech-runtime/resident-routing") })
        );
        assert_eq!(metadata["selective_conversions"], true);
        let core_features = manifest["dependencies"]["mech-core"]["features"]
            .as_array()
            .unwrap();
        assert!(
            core_features
                .iter()
                .any(|feature| feature.as_str() == Some("selective-convert"))
        );
        let value_features = metadata["required_core_value_features"].as_array().unwrap();
        for required in [
            "bool", "u64", "f64", "string", "map", "matrixd", "set", "tuple", "enum",
        ] {
            assert!(
                value_features
                    .iter()
                    .any(|value| value.as_str() == Some(required))
            );
        }
        for value in value_features {
            assert!(
                core_features
                    .iter()
                    .any(|feature| feature.as_str() == value.as_str())
            );
        }
        for unused in ["complex", "r64", "u128", "i128", "table"] {
            assert!(
                !value_features
                    .iter()
                    .any(|value| value.as_str() == Some(unused))
            );
        }
        assert!(
            !generated.metadata()["runtime_features"]
                .as_array()
                .unwrap()
                .iter()
                .any(|feature| feature.as_str() == Some("mech-stdlib/native-plan"))
        );
        assert_eq!(
            metadata["kernel_operations"],
            serde_json::json!(
                profile
                    .operations()
                    .iter()
                    .map(|operation| operation.canonical_name())
                    .collect::<Vec<_>>()
            )
        );
        pins.push(generated.pins().clone());
    }
    assert_eq!(pins[0], pins[1]);
}

#[test]
fn unrecognized_registration_retains_complete_catalog_without_skipping_obligations() {
    let temporary = tempfile::tempdir().unwrap();
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    // The scalar launcher still restores the source-kernel closure. Unlike a
    // compiled native image, it cannot drop this registration after lowering.
    let source = "value<u64> + 1u64\n";
    let (variants, full) = artifacts(source);
    for (index, artifact) in variants.iter().enumerate() {
        let operation = artifact
            .kernel_operation_references()
            .into_iter()
            .find(|operation| operation.canonical_name() == "math/add")
            .unwrap();
        let entry = full
            .resident_factory(&operation.module_path, &operation.operation_name)
            .unwrap();
        let catalog = replaced_catalog(
            &full,
            &operation,
            entry.clone().with_factory(entry.factory()),
        );
        // Its bound scalar semantic evidence remains valid, but its private
        // REGISTRATION evidence was deliberately invalidated by public API.
        assert_eq!(
            emit_program_object(artifact, &full).unwrap().bytes(),
            emit_program_object(artifact, &catalog).unwrap().bytes()
        );
        let generated = mech_gpu::native_artifact::generate_linked_application(
            &workspace,
            &temporary.path().join(format!("fallback-{index}")),
            "complete-canonical-catalog",
            artifact,
            &catalog,
            source.as_bytes(),
        )
        .unwrap();
        let main = std::fs::read_to_string(generated.root().join("src/main.rs")).unwrap();
        assert!(main.contains("mech_engine::install_intrinsic_resident(&mut builder)"));
        assert!(!main.contains("mech_engine::linked_factories::"));
        let profile: serde_json::Value = serde_json::from_slice(
            &std::fs::read(generated.root().join("factory-profile.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(profile["mode"], "complete-catalog");
        assert_eq!(profile["selection_basis"], "artifact-kernels");
        let manifest = std::fs::read_to_string(generated.manifest_path())
            .unwrap()
            .parse::<toml_edit::DocumentMut>()
            .unwrap();
        assert!(manifest["dependencies"].get("mech-stdlib").is_some());
        assert_eq!(profile["selective_conversions"], false);
        assert!(
            generated.metadata()["runtime_features"]
                .as_array()
                .unwrap()
                .iter()
                .any(|feature| feature.as_str() == Some("mech-stdlib/native-plan"))
        );
        assert_eq!(
            profile["missing_linkage_operations"],
            serde_json::json!(["math/add"])
        );
        assert_eq!(
            profile["kernel_operations"],
            serde_json::json!(
                artifact
                    .kernel_operation_references()
                    .iter()
                    .map(|operation| operation.canonical_name())
                    .collect::<Vec<_>>()
            )
        );
    }
}

#[test]
fn scalar_and_managed_launchers_enable_their_selected_factory_owner_features() {
    // The generator's source catalog has String equality evidence even when
    // this particular artifact is scalar numeric. A scalar runtime does not
    // otherwise enable the owning collection module; its declaration must
    // supply that requirement instead of producing an unresolved Rust path.
    let source = "value<u64> == 2u64\n";
    let (variants, catalog) = artifacts(source);
    let temporary = tempfile::tempdir().unwrap();
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    for (index, artifact) in variants.iter().enumerate() {
        for managed in [false, true] {
            let directory = temporary.path().join(format!("project-{index}-{managed}"));
            let generate = if managed {
                mech_gpu::native_artifact::generate_linked_program_application
            } else {
                mech_gpu::native_artifact::generate_linked_application
            };
            let generated = generate(
                &workspace,
                &directory,
                "selected-equality",
                artifact,
                &catalog,
                source.as_bytes(),
            )
            .unwrap();
            let manifest = std::fs::read_to_string(generated.manifest_path())
                .unwrap()
                .parse::<toml_edit::DocumentMut>()
                .unwrap();
            let release = &manifest["profile"]["release"];
            assert!(manifest["dependencies"].get("mech-stdlib").is_none());
            assert_eq!(release["strip"].as_str(), Some("symbols"));
            assert_eq!(release["opt-level"].as_integer(), Some(3));
            assert_eq!(release["lto"].as_str(), Some("thin"));
            assert!(release.get("panic").is_none());
            assert!(release.get("codegen-units").is_none());
            let features = manifest["dependencies"]["mech-engine"]["features"]
                .as_array()
                .unwrap();
            let profile = if managed {
                let object = emit_program_object(artifact, &catalog).unwrap();
                NativeFactoryLinkageProfile::derive_native_image(
                    object.descriptor().plan(),
                    &catalog,
                )
            } else {
                NativeFactoryLinkageProfile::derive(artifact, &catalog)
            };
            assert_eq!(
                profile
                    .factories()
                    .unwrap()
                    .iter()
                    .any(|factory| factory.required_engine_features().contains(&"pure_text")),
                !managed,
            );
            if managed {
                // This equality is a native scalar instruction, not a call to
                // the source catalog's String-capable equality registration.
                assert!(profile.operations().is_empty());
            }
            for factory in profile.factories().unwrap() {
                for requirement in factory.required_engine_features() {
                    assert!(
                        features
                            .iter()
                            .any(|value| value.as_str() == Some(requirement))
                    );
                }
            }
            let profile: serde_json::Value = serde_json::from_slice(
                &std::fs::read(directory.join("factory-profile.json")).unwrap(),
            )
            .unwrap();
            assert_eq!(
                profile["required_engine_features"],
                if managed {
                    serde_json::json!([])
                } else {
                    serde_json::json!(["pure_text"])
                }
            );
        }
    }
}
