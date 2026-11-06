use super::*;
use crate::native_artifact::{NativeProgramDescriptor, emit_program_object};
use mech_engine::CanonicalSourceFrontend;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn artifacts(source: &str) -> [mech_engine::ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x494d4750494e), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let artifact = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    [artifact, decoded]
}

#[test]
fn object_image_binding_rejects_changed_bytes_identity_and_every_binding_byte() {
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(include_str!(
        "../../../../tests/fixtures/native/numeric-parent-child.mec"
    )) {
        let object = emit_program_object(&artifact, &catalog).unwrap();
        let image = object.descriptor().encode_build_owned_image().unwrap();
        let identity = object.identity().bytes();
        let binding = object.image_binding().bytes();
        // SAFETY: all metadata and symbols here come from the genuine checked
        // object. No entry or native-code execution authority is manufactured.
        assert!(unsafe { verify(&image, identity, binding) }.is_ok());
        for byte in 0..binding.len() {
            let mut changed = *binding;
            changed[byte] ^= 1;
            assert!(
                unsafe { verify(&image, identity, &changed) }.is_err(),
                "binding byte {byte}"
            );
        }
        for byte in 0..identity.len() {
            let mut changed = *identity;
            changed[byte] ^= 1;
            assert!(
                unsafe { verify(&image, &changed, binding) }.is_err(),
                "identity byte {byte}"
            );
        }
        for byte in [0, 1, image.len() / 2, image.len() - 1] {
            let mut changed = image.clone();
            changed[byte] ^= 1;
            assert!(
                unsafe { verify(&changed, identity, binding) }.is_err(),
                "image byte {byte}"
            );
        }
        let mut extended = image.clone();
        extended.push(b' ');
        assert!(unsafe { verify(&extended, identity, binding) }.is_err());
        assert!(unsafe { verify(&image[..image.len() - 1], identity, binding) }.is_err());
        let other = emit_program_object(&artifacts("input<u64> + 1u64\n")[0], &catalog).unwrap();
        assert!(
            unsafe {
                verify(
                    &image,
                    other.identity().bytes(),
                    other.image_binding().bytes(),
                )
            }
            .is_err()
        );
    }
}

#[test]
fn object_bound_and_independent_images_preserve_all_prepared_metadata() {
    let catalog = mech_stdlib::source_catalog();
    for source in [
        "input<u64> + 1u64\n",
        include_str!("../../../../tests/fixtures/native/numeric-parent-child.mec"),
        "+> map\n+> sequence\nwords := input<[string]>\nmap/histogram(words)\n",
    ] {
        for artifact in artifacts(source) {
            let object = emit_program_object(&artifact, &catalog).unwrap();
            let image = object.descriptor().encode_build_owned_image().unwrap();
            let independent =
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }
                    .unwrap();
            for bytes in [image.clone(), super::super::packing::pack(&image).unwrap()] {
                let restored = unsafe {
                    NativeProgramDescriptor::decode_object_bound_image(
                        &bytes,
                        &catalog,
                        object.identity().bytes(),
                        object.image_binding().bytes(),
                    )
                }
                .unwrap();
                assert_eq!(restored.encode_build_owned_image().unwrap(), image);
                assert_eq!(independent.encode_build_owned_image().unwrap(), image);
                assert_eq!(restored.typed_ir_sha256(), independent.typed_ir_sha256());
            }
            // Proof bytes retain the independent loader's existing rejection,
            // while the bound path rejects any changed proof before decoding.
            let mut changed: serde_json::Value = serde_json::from_slice(&image).unwrap();
            if let Some(region) = changed["descriptor"]["closed_tails"]["regions"]
                .as_array_mut()
                .and_then(|regions| {
                    regions
                        .iter_mut()
                        .find(|region| region["numeric_tree"].is_object())
                })
            {
                let word = &mut region["numeric_tree"]["nodes"][0]["compute"];
                *word = (word.as_u64().unwrap() + 1).into();
                let bytes = serde_json::to_vec(&changed).unwrap();
                assert!(
                    unsafe { NativeProgramDescriptor::decode_build_owned_image(&bytes, &catalog) }
                        .is_err()
                );
                assert!(
                    unsafe {
                        NativeProgramDescriptor::decode_object_bound_image(
                            &bytes,
                            &catalog,
                            object.identity().bytes(),
                            object.image_binding().bytes(),
                        )
                    }
                    .is_err()
                );
            }
        }
    }
}

#[test]
fn native_metadata_exports_preserve_the_exact_descriptor_and_object_binding() {
    let catalog = mech_stdlib::source_catalog();
    for source in [
        "input<u64> + 1u64\n",
        include_str!("../../../../tests/fixtures/native/numeric-parent-child.mec"),
        "+> map\n+> sequence\nwords := input<[string]>\nmap/histogram(words)\n",
    ] {
        for artifact in artifacts(source) {
            let object = emit_program_object(&artifact, &catalog).unwrap();
            let json = object.descriptor().encode_build_owned_image().unwrap();
            let bytes = object.native_metadata();
            let binding = object.native_metadata_binding().bytes();
            assert!(bytes.starts_with(mech_engine::native_artifact::NATIVE_METADATA_IMAGE_MAGIC));
            // SAFETY: genuine data, identity and binding from the same checked
            // Object. No foreign entry or fabricated source proof is executed.
            let bound = unsafe {
                NativeProgramDescriptor::decode_object_bound_image(
                    bytes,
                    &catalog,
                    object.identity().bytes(),
                    binding,
                )
            }
            .unwrap();
            let independent =
                unsafe { NativeProgramDescriptor::decode_build_owned_image(bytes, &catalog) }
                    .unwrap();
            assert_eq!(bound.encode_build_owned_image().unwrap(), json);
            assert_eq!(independent.encode_build_owned_image().unwrap(), json);
            assert!(
                unsafe {
                    verify(
                        bytes,
                        object.identity().bytes(),
                        object.image_binding().bytes(),
                    )
                }
                .is_err()
            );
            assert!(unsafe { verify(&json, object.identity().bytes(), binding) }.is_err());
            for byte in 0..binding.len() {
                let mut changed = *binding;
                changed[byte] ^= 1;
                assert!(unsafe { verify(bytes, object.identity().bytes(), &changed) }.is_err());
            }
            for byte in [0, 7, bytes.len() / 2, bytes.len() - 1] {
                let mut changed = bytes.to_vec();
                changed[byte] ^= 1;
                assert!(
                    unsafe {
                        NativeProgramDescriptor::decode_object_bound_image(
                            &changed,
                            &catalog,
                            object.identity().bytes(),
                            binding,
                        )
                    }
                    .is_err()
                );
            }
            assert!(
                unsafe {
                    verify(
                        &bytes[..bytes.len() - 1],
                        object.identity().bytes(),
                        binding,
                    )
                }
                .is_err()
            );
            let mut changed = bytes.to_vec();
            changed.push(0);
            assert!(unsafe { verify(&changed, object.identity().bytes(), binding) }.is_err());
        }
    }
}

#[test]
fn borrowed_frame_metadata_genuine_object_rows_keep_runtime_lanes_and_exact_public_images() {
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(include_str!(
        "../../../../tests/fixtures/native/numeric-parent-child.mec"
    )) {
        let object = emit_program_object(&artifact, &catalog).unwrap();
        let expected = object.descriptor().encode_build_owned_image().unwrap();
        let bytes = Box::leak(object.native_metadata().to_vec().into_boxed_slice());
        // SAFETY: exact immutable metadata copy, identity and binding from
        // this genuine checked Object. No entry is invoked by this reader test.
        let verified = unsafe {
            verify(
                bytes,
                object.identity().bytes(),
                object.native_metadata_binding().bytes(),
            )
        }
        .unwrap();
        let restored = crate::native_artifact::program::image::decode::decode_compiled_for_test(
            &verified, &catalog,
        )
        .unwrap();
        let frames = restored
            .metadata()
            .closed_tails
            .regions
            .iter()
            .filter_map(|r| r.numeric.as_ref())
            .flat_map(|p| p.frames.iter())
            .collect::<Vec<_>>();
        assert!(!frames.is_empty());
        assert!(
            frames
                .iter()
                .all(|f| f.compiler_data_is_borrowed_and_unmaterialized())
        );
        for frame in &frames {
            std::hint::black_box((
                frame.value,
                frame.maximum_arity,
                frame.scalar_nodes(),
                &frame.arrays,
                &frame.array_bindings,
            ));
        }
        assert!(
            frames
                .iter()
                .all(|f| f.compiler_data_is_borrowed_and_unmaterialized())
        );
        assert_eq!(restored.encode_build_owned_image().unwrap(), expected);
    }
}

#[test]
fn frozen_static_layouts_match_independent_derivation_and_reject_each_inventory_mutation() {
    use serde::Deserialize;
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(include_str!(
        "../../../../tests/fixtures/native/numeric-parent-child.mec"
    )) {
        let object = emit_program_object(&artifact, &catalog).unwrap();
        let bytes = object.native_metadata();
        let mut reader =
            mech_engine::native_artifact::BuildOwnedImageReader::from_image(bytes).unwrap();
        let image = serde_json::Value::deserialize(&mut reader).unwrap();
        reader.end().unwrap();
        assert_eq!(image["version"], 8);
        let independent =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(bytes, &catalog) }.unwrap();
        let bound = unsafe {
            NativeProgramDescriptor::decode_object_bound_image(
                bytes,
                &catalog,
                object.identity().bytes(),
                object.native_metadata_binding().bytes(),
            )
        }
        .unwrap();
        let original = object.descriptor().metadata();
        for restored in [&independent, &bound] {
            let restored = restored.metadata();
            assert_eq!(
                restored.unsigned_buffer_calls,
                original.unsigned_buffer_calls
            );
            assert_eq!(restored.call_routes, original.call_routes);
            assert_eq!(restored.call_cache, original.call_cache);
            assert_eq!(restored.frame_retirements, original.frame_retirements);
            assert_eq!(restored.cleanup_effects, original.cleanup_effects);
            assert_eq!(restored.compositions, original.compositions);
            assert_eq!(
                restored.uniform_word_collections,
                original.uniform_word_collections
            );
        }
        // Public V2 descriptors keep their existing field inventory. V3 native
        // images continue to use independent derivation when no cache is present.
        let public = object.descriptor().encode_build_owned_image().unwrap();
        assert!(
            !serde_json::from_slice::<serde_json::Value>(&public).unwrap()["descriptor"]
                .as_object()
                .unwrap()
                .contains_key("static_layouts")
        );
        struct LegacyPlan<'a>(&'a mech_engine::native_artifact::NativeProgramArtifactPlan);
        impl serde::Serialize for LegacyPlan<'_> {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                self.0.serialize_build_owned_table_plan(serializer)
            }
        }
        for version in [3, 4] {
            let mut legacy = image.clone();
            legacy["version"] = version.into();
            legacy["descriptor"]
                .as_object_mut()
                .unwrap()
                .remove("schema_templates");
            legacy["descriptor"]
                .as_object_mut()
                .unwrap()
                .remove("numeric_maps");
            // Reproduce the actual legacy representation, including full
            // selected templates, rather than relabeling a shared-table plan.
            legacy["descriptor"]["plan"] =
                serde_json::to_value(LegacyPlan(object.descriptor().plan())).unwrap();
            if let Some(layouts) = legacy["descriptor"]["static_layouts"].as_object_mut() {
                layouts.remove("uniform_word_collections");
            }
            if version == 3 {
                legacy["descriptor"]
                    .as_object_mut()
                    .unwrap()
                    .remove("static_layouts");
            }
            let legacy = mech_engine::native_artifact::encode_native_metadata_image(
                &serde_json::to_vec(&legacy).unwrap(),
            )
            .unwrap();
            let legacy =
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&legacy, &catalog) }
                    .unwrap();
            assert_eq!(legacy.encode_build_owned_image().unwrap(), public);
        }

        for fault in 0..8 {
            let mut bad = image.clone();
            let frozen = &mut bad["descriptor"]["static_layouts"];
            match fault {
                0 => {
                    let call = frozen["unsigned_buffer_calls"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|call| call.is_object())
                        .expect("fixture has a buffer call");
                    call["site"] = u32::MAX.into();
                }
                1 => frozen["call_routes"][0] = "StringMatrixElement".into(),
                2 => {
                    let repeated = frozen["call_cache"]["repeated"].as_array_mut().unwrap();
                    repeated[0] = (!repeated[0].as_bool().unwrap()).into();
                }
                3 => {
                    let recipe = frozen["frame_retirements"]["recipes"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|recipe| recipe.is_object())
                        .expect("fixture has a private frame");
                    recipe["work"] = (recipe["work"].as_u64().unwrap() + 1).into();
                }
                4 => {
                    frozen["cleanup_effects"]["boundaries"] = serde_json::json!([{
                        "total": 1, "charges": 1, "omitted_work": 0, "has_frames": false, "effects": []
                    }])
                }
                5 => {
                    frozen["compositions"]["words"] =
                        (frozen["compositions"]["words"].as_u64().unwrap() + 1).into()
                }
                6 => {
                    bad["descriptor"]
                        .as_object_mut()
                        .unwrap()
                        .remove("static_layouts");
                }
                _ => bad["version"] = 3.into(),
            }
            let bytes = mech_engine::native_artifact::encode_native_metadata_image(
                &serde_json::to_vec(&bad).unwrap(),
            )
            .unwrap();
            // SAFETY: confined negative reader tests; changed proofs never reach
            // native execution and no replacement Object binding is fabricated.
            assert!(
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&bytes, &catalog) }
                    .is_err(),
                "independent fault {fault}"
            );
            assert!(
                unsafe {
                    NativeProgramDescriptor::decode_object_bound_image(
                        &bytes,
                        &catalog,
                        object.identity().bytes(),
                        object.native_metadata_binding().bytes(),
                    )
                }
                .is_err(),
                "bound fault {fault}"
            );
        }
    }
}

#[test]
fn native_table_rows_reject_foreign_types_and_mismatched_binding_arity() {
    use serde::Deserialize;
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts("+> sequence\nwords := input<[string]>\nsequence/length(words)\n") {
        let object = emit_program_object(&artifact, &catalog).unwrap();
        let mut reader = mech_engine::native_artifact::BuildOwnedImageReader::from_image(
            object.native_metadata(),
        )
        .unwrap();
        let image = serde_json::Value::deserialize(&mut reader).unwrap();
        reader.end().unwrap();
        assert_eq!(image["version"], 8);
        let rows = image["descriptor"]["plan"]["values"].as_array().unwrap();
        assert!(
            rows.iter()
                .all(|r| r.as_array().is_some_and(|r| r.len() == 2))
        );
        let scalar = rows.iter().position(|r| r[1] == 5).unwrap();
        let call = image["descriptor"]["calls"]
            .as_array()
            .unwrap()
            .iter()
            .position(|r| !r.is_null())
            .unwrap();
        assert_eq!(
            image["descriptor"]["calls"][call].as_array().unwrap().len(),
            12
        );
        for fault in 0..3 {
            let mut bad = image.clone();
            match fault {
                0 => bad["descriptor"]["plan"]["values"][scalar][0] = u32::MAX.into(),
                1 => bad["descriptor"]["plan"]["values"][scalar][1] = 1.into(),
                _ => bad["descriptor"]["calls"][call][3] = u64::MAX.into(),
            }
            let json = serde_json::to_vec(&bad).unwrap();
            let native = mech_engine::native_artifact::encode_native_metadata_image(&json).unwrap();
            // SAFETY: confined negative wire test. No changed descriptor or
            // fabricated proof can escape or reach any native executable.
            assert!(
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&native, &catalog) }
                    .is_err()
            );
            assert!(
                unsafe {
                    NativeProgramDescriptor::decode_object_bound_image(
                        &native,
                        &catalog,
                        object.identity().bytes(),
                        object.native_metadata_binding().bytes(),
                    )
                }
                .is_err()
            );
        }
    }
}

#[test]
fn native_u32_arithmetic_images_preserve_selected_checked_operations() {
    use mech_engine::native_artifact::{
        NativeIterationAction as Action, NativeMatchAction as Scalar, NativeScalarOp as Op,
    };
    let catalog = mech_stdlib::source_catalog();
    for (operator, expected) in [("*", Op::U32Mul), ("/", Op::U32Div), ("%", Op::U32Rem)] {
        for artifact in artifacts(&format!("left<u32> {operator} right<u32>\n")) {
            let object = emit_program_object(&artifact, &catalog).unwrap();
            assert!(object.descriptor().plan().blocks().iter().any(|block| matches!(block.action(), Action::Scalar(Scalar::Primitive { op, .. }) if *op == expected)));
            let image = object.native_metadata();
            // SAFETY: exact bytes and symbols from this checked Object. No
            // decoded metadata is executed through a fabricated native entry.
            let independent =
                unsafe { NativeProgramDescriptor::decode_build_owned_image(image, &catalog) }
                    .unwrap();
            let bound = unsafe {
                NativeProgramDescriptor::decode_object_bound_image(
                    image,
                    &catalog,
                    object.identity().bytes(),
                    object.native_metadata_binding().bytes(),
                )
            }
            .unwrap();
            for restored in [&independent, &bound] {
                assert_eq!(
                    restored.encode_build_owned_image().unwrap(),
                    object.descriptor().encode_build_owned_image().unwrap()
                );
            }
        }
    }
}

#[test]
fn frozen_uniform_collections_preserve_fields_and_reject_changed_proofs() {
    use serde::Deserialize;
    let catalog = mech_stdlib::source_catalog();
    for width in [8, 16, 32, 64] {
        for filtered in [false, true] {
            let source = if filtered {
                format!("[word<u{width}> | i <- 0u64..=count<u64>, i < count]\n")
            } else {
                format!("[word<u{width}> | i <- 0u64..count<u64>]\n")
            };
            for artifact in artifacts(&source) {
                let object = emit_program_object(&artifact, &catalog).unwrap();
                let expected = &object.descriptor().metadata().uniform_word_collections;
                assert_eq!(expected.len(), 1);
                assert_eq!(expected[0].filtered(), filtered);
                let bytes = object.native_metadata();
                let mut reader =
                    mech_engine::native_artifact::BuildOwnedImageReader::from_image(bytes).unwrap();
                let image = serde_json::Value::deserialize(&mut reader).unwrap();
                reader.end().unwrap();
                assert_eq!(image["version"], 8);
                assert_eq!(
                    image["descriptor"]["static_layouts"]["uniform_word_collections"]
                        .as_array()
                        .unwrap()
                        .len(),
                    1
                );
                // SAFETY: genuine immutable metadata from this checked Object.
                // Neither loader authorizes a fabricated native entry here.
                let independent =
                    unsafe { NativeProgramDescriptor::decode_build_owned_image(bytes, &catalog) }
                        .unwrap();
                let bound = unsafe {
                    NativeProgramDescriptor::decode_object_bound_image(
                        bytes,
                        &catalog,
                        object.identity().bytes(),
                        object.native_metadata_binding().bytes(),
                    )
                }
                .unwrap();
                for restored in [&independent, &bound] {
                    assert_eq!(&restored.metadata().uniform_word_collections, expected);
                    assert_eq!(
                        restored.encode_build_owned_image().unwrap(),
                        object.descriptor().encode_build_owned_image().unwrap()
                    );
                }
                // Genuine V5 representation lacks this inventory and rederives
                // it. Relabeling V6 while retaining its extra field must fail.
                let mut legacy = image.clone();
                legacy["version"] = 5.into();
                legacy["descriptor"]
                    .as_object_mut()
                    .unwrap()
                    .remove("schema_templates");
                legacy["descriptor"]
                    .as_object_mut()
                    .unwrap()
                    .remove("numeric_maps");
                legacy["descriptor"]["static_layouts"]
                    .as_object_mut()
                    .unwrap()
                    .remove("uniform_word_collections");
                let legacy = mech_engine::native_artifact::encode_native_metadata_image(
                    &serde_json::to_vec(&legacy).unwrap(),
                )
                .unwrap();
                let restored =
                    unsafe { NativeProgramDescriptor::decode_build_owned_image(&legacy, &catalog) }
                        .unwrap();
                assert_eq!(&restored.metadata().uniform_word_collections, expected);
                for fault in 0..13 {
                    let mut changed = image.clone();
                    let layouts = &mut changed["descriptor"]["static_layouts"];
                    let region = &mut layouts["uniform_word_collections"][0];
                    match fault {
                        0 => region["collection"] = u32::MAX.into(),
                        1 => region["iteration"] = u32::MAX.into(),
                        2 => region["header"] = u32::MAX.into(),
                        3 => region["word"] = u32::MAX.into(),
                        4 => region["start"] = u32::MAX.into(),
                        5 => region["limit"] = u32::MAX.into(),
                        6 => region["inclusive"] = (!region["inclusive"].as_bool().unwrap()).into(),
                        7 => region["filtered"] = (!filtered).into(),
                        8 => region["kind"] = "F64".into(),
                        9 => layouts["uniform_word_collections"] = serde_json::json!([]),
                        10 => {
                            layouts
                                .as_object_mut()
                                .unwrap()
                                .remove("uniform_word_collections");
                        }
                        11 => layouts["uniform_word_collections"] = serde_json::Value::Null,
                        _ => changed["version"] = 5.into(),
                    }
                    let native = mech_engine::native_artifact::encode_native_metadata_image(
                        &serde_json::to_vec(&changed).unwrap(),
                    )
                    .unwrap();
                    // SAFETY: confined negative wire cases. Changed proofs
                    // never reach native execution or acquire Object authority.
                    assert!(
                        unsafe {
                            NativeProgramDescriptor::decode_build_owned_image(&native, &catalog)
                        }
                        .is_err(),
                        "independent fault {fault}"
                    );
                    assert!(
                        unsafe {
                            NativeProgramDescriptor::decode_object_bound_image(
                                &native,
                                &catalog,
                                object.identity().bytes(),
                                object.native_metadata_binding().bytes(),
                            )
                        }
                        .is_err(),
                        "bound fault {fault}"
                    );
                }
            }
        }
    }
}

#[test]
fn shared_template_definitions_preserve_sites_and_reject_foreign_rows_and_factories() {
    use serde::Deserialize;
    let catalog = mech_stdlib::source_catalog();
    fn headers(value: &serde_json::Value, path: &str, found: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(fields) => {
                if value["leaf"].is_u64()
                    && value["sources"]
                        .as_array()
                        .is_some_and(|sources| !sources.is_empty())
                {
                    found.push(path.to_owned());
                    return;
                }
                for (key, value) in fields {
                    let key = key.replace('~', "~0").replace('/', "~1");
                    headers(value, &format!("{path}/{key}"), found);
                }
            }
            serde_json::Value::Array(values) => {
                for (index, value) in values.iter().enumerate() {
                    headers(value, &format!("{path}/{index}"), found);
                }
            }
            _ => {}
        }
    }
    let mut observed_sharing = false;
    for source in [
        "+> sequence\nleft := sequence/length(first<[string]>)\nright := sequence/length(second<[string]>)\nleft + right\n",
        include_str!("../../../../tests/fixtures/native/numeric-parent-child.mec"),
        include_str!("../../../../tests/fixtures/native/window-histogram.mec"),
        "+> sequence\n[at + 1u64 | at <- 0u64..=limit<u64>]\n",
        "+> text\n+> map\nentries := {\"a\": \"A\", \"b\": \"B\"}\ntext/join([entries{ch} | ch <- seed<string>], \"\")\n",
    ] {
        for artifact in artifacts(source) {
            let object = emit_program_object(&artifact, &catalog).unwrap();
            let expected = object.descriptor().encode_build_owned_image().unwrap();
            let bytes = object.native_metadata();
            let mut reader =
                mech_engine::native_artifact::BuildOwnedImageReader::from_image(bytes).unwrap();
            let image = serde_json::Value::deserialize(&mut reader).unwrap();
            reader.end().unwrap();
            assert_eq!(image["version"], 8);
            let definitions = image["descriptor"]["plan"]["template_definitions"]
                .as_array()
                .unwrap();
            assert!(!definitions.is_empty());
            let library_sites = object
                .descriptor()
                .plan()
                .blocks()
                .iter()
                .filter(|block| {
                    matches!(
                        block.action(),
                        mech_engine::native_artifact::NativeIterationAction::LibraryCall { .. }
                    )
                })
                .count();
            observed_sharing |= library_sites > definitions.len();
            // SAFETY: exact genuine checked Object metadata; both loaders must
            // reconstruct every original source header and the full semantic
            // descriptor. Neither assertion executes a fabricated native entry.
            let independent =
                unsafe { NativeProgramDescriptor::decode_build_owned_image(bytes, &catalog) }
                    .unwrap();
            let bound = unsafe {
                NativeProgramDescriptor::decode_object_bound_image(
                    bytes,
                    &catalog,
                    object.identity().bytes(),
                    object.native_metadata_binding().bytes(),
                )
            }
            .unwrap();
            assert_eq!(independent.encode_build_owned_image().unwrap(), expected);
            assert_eq!(bound.encode_build_owned_image().unwrap(), expected);
            let mut sites = Vec::new();
            headers(&image["descriptor"]["plan"], "/descriptor/plan", &mut sites);
            let path = sites
                .first()
                .expect("fixture has no nonempty template header");
            let definition = image.pointer(path).unwrap()["leaf"].as_u64().unwrap() as usize;
            for fault in 0..8 {
                let mut changed = image.clone();
                let plan = &mut changed["descriptor"]["plan"];
                match fault {
                    0 => changed.pointer_mut(path).unwrap()["leaf"] = u32::MAX.into(),
                    1 => changed.pointer_mut(path).unwrap()["sources"] = serde_json::json!([]),
                    2 => {
                        plan["template_definitions"][definition]["selected_key"]["operation_name"] =
                            "unlinked-template-definition".into()
                    }
                    3 => {
                        plan["template_definitions"][definition]["primitive"] =
                            "ForeignPrimitive".into()
                    }
                    4 => {
                        plan["template_definitions"][definition]["implementation_memory"] =
                            "ForeignMemoryClass".into()
                    }
                    5 => plan["template_definitions"][definition]["inputs"] = serde_json::json!([]),
                    6 => {
                        plan.as_object_mut().unwrap().remove("template_definitions");
                    }
                    _ => changed["version"] = 4.into(),
                }
                let json = serde_json::to_vec(&changed).unwrap();
                let native =
                    mech_engine::native_artifact::encode_native_metadata_image(&json).unwrap();
                // SAFETY: confined negative wire tests; altered metadata can
                // neither escape these checks nor authorize any native entry.
                assert!(
                    unsafe { NativeProgramDescriptor::decode_build_owned_image(&native, &catalog) }
                        .is_err(),
                    "independent fault {fault}"
                );
                assert!(
                    unsafe {
                        NativeProgramDescriptor::decode_object_bound_image(
                            &native,
                            &catalog,
                            object.identity().bytes(),
                            object.native_metadata_binding().bytes(),
                        )
                    }
                    .is_err(),
                    "bound fault {fault}"
                );
            }
        }
    }
    assert!(
        observed_sharing,
        "fixture did not exercise repeated definitions"
    );
}

#[test]
fn object_bound_calls_preserve_independent_selections_and_reject_template_tampering() {
    let catalog = mech_stdlib::source_catalog();
    for source in [
        "+> sequence\n[at + 1u64 | at <- 0u64..=limit<u64>]\n",
        include_str!("../../../../tests/fixtures/native/window-histogram.mec"),
        "+> text\n+> map\nentries := {\"a\": \"A\", \"b\": \"B\"}\ntext/join([entries{ch} | ch <- seed<string>], \"\")\n",
        include_str!("../../../../tests/fixtures/native/numeric-parent-child.mec"),
    ] {
        for artifact in artifacts(source) {
            let object = emit_program_object(&artifact, &catalog).unwrap();
            let image = object.descriptor().encode_build_owned_image().unwrap();
            // SAFETY: both paths use the same genuine checked Object metadata;
            // no native entry or execution authority is manufactured here.
            let independent =
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }
                    .unwrap();
            let bound = unsafe {
                NativeProgramDescriptor::decode_object_bound_image(
                    &image,
                    &catalog,
                    object.identity().bytes(),
                    object.image_binding().bytes(),
                )
            }
            .unwrap();
            assert_eq!(bound.encode_build_owned_image().unwrap(), image);
            assert_eq!(independent.encode_build_owned_image().unwrap(), image);
            let plan = bound.plan();
            assert!(plan.prepared_library_template(plan.blocks().len()).is_err());
            for (site, (a, b)) in independent
                .metadata()
                .calls
                .iter()
                .zip(bound.metadata().calls.iter())
                .enumerate()
            {
                assert_eq!(
                    a.as_ref().map(|c| c.template()),
                    b.as_ref().map(|c| c.template())
                );
                assert_eq!(
                    b.as_ref().map(|c| c.template()),
                    plan.prepared_library_template(site).unwrap()
                );
            }
            let mut changed: serde_json::Value = serde_json::from_slice(&image).unwrap();
            let call = changed["descriptor"]["calls"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|call| !call.is_null())
                .expect("fixture must exercise a prepared call");
            call["template"]["origin"]["node"] = u32::MAX.into();
            let changed = serde_json::to_vec(&changed).unwrap();
            assert!(
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&changed, &catalog) }
                    .is_err()
            );
            assert!(
                unsafe {
                    NativeProgramDescriptor::decode_object_bound_image(
                        &changed,
                        &catalog,
                        object.identity().bytes(),
                        object.image_binding().bytes(),
                    )
                }
                .is_err()
            );
        }
    }
}

#[test]
fn frozen_frames_preserve_metadata_and_independent_legacy_validation() {
    let catalog = mech_stdlib::source_catalog();
    for source in [
        include_str!("../../../../tests/fixtures/native/numeric-parent-named-managed.mec"),
        include_str!("../../../../tests/fixtures/native/private-tagged-map.mec"),
        include_str!("../../../../tests/fixtures/native/window-histogram.mec"),
    ] {
        for artifact in artifacts(source) {
            let object = emit_program_object(&artifact, &catalog).unwrap();
            let bytes = object.descriptor().encode_build_owned_image().unwrap();
            let image: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(image["version"], 2);
            assert!(image["descriptor"]["buffer_frames"].is_object());
            for bound in [false, true] {
                // SAFETY: complete metadata and symbols come from one genuine
                // checked Object. No entry authority is fabricated here.
                let restored = if bound {
                    unsafe {
                        NativeProgramDescriptor::decode_object_bound_image(
                            &bytes,
                            &catalog,
                            object.identity().bytes(),
                            object.image_binding().bytes(),
                        )
                    }
                } else {
                    unsafe { NativeProgramDescriptor::decode_build_owned_image(&bytes, &catalog) }
                }
                .unwrap();
                assert_eq!(restored.encode_build_owned_image().unwrap(), bytes);
            }
            let mut legacy = image.clone();
            legacy["version"] = 1.into();
            legacy["descriptor"]
                .as_object_mut()
                .unwrap()
                .remove("schema_templates");
            legacy["descriptor"]
                .as_object_mut()
                .unwrap()
                .remove("buffer_frames");
            // Build-side compatibility conversion only; no changed image is
            // paired with an entry or allowed to bypass Object provenance.
            let restored = unsafe {
                NativeProgramDescriptor::decode_build_owned_image(
                    &serde_json::to_vec(&legacy).unwrap(),
                    &catalog,
                )
            }
            .unwrap();
            assert_eq!(restored.encode_build_owned_image().unwrap(), bytes);
            for mode in 0..4 {
                let mut changed = image.clone();
                match mode {
                    0 => {
                        let old = changed["descriptor"]["buffer_frames"]["scalar_fields"]
                            .as_u64()
                            .unwrap();
                        changed["descriptor"]["buffer_frames"]["scalar_fields"] = (old + 1).into();
                    }
                    1 => {
                        changed["descriptor"]
                            .as_object_mut()
                            .unwrap()
                            .remove("buffer_frames");
                    }
                    2 => {
                        changed["descriptor"]["buffer_frames"] = serde_json::Value::Null;
                    }
                    _ => {
                        changed["version"] = 1.into();
                    }
                }
                let changed = serde_json::to_vec(&changed).unwrap();
                assert!(
                    unsafe {
                        NativeProgramDescriptor::decode_build_owned_image(&changed, &catalog)
                    }
                    .is_err(),
                    "case {mode}"
                );
                assert!(
                    unsafe {
                        NativeProgramDescriptor::decode_object_bound_image(
                            &changed,
                            &catalog,
                            object.identity().bytes(),
                            object.image_binding().bytes(),
                        )
                    }
                    .is_err(),
                    "case {mode}"
                );
            }
        }
    }
}
#[test]
fn native_numeric_map_inventory_matches_source_and_rejects_changed_proofs_and_unfunded_headers() {
    use serde::Deserialize;
    let text = "byte(x<u64>) => (out<u8>) :=\n  result<u8> := x\n  out := result.\n[byte(x) | x <- words<[u64]>]\n";
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(text) {
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &catalog,
            crate::native_artifact::NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let bytes = object.native_metadata();
        let mut reader =
            mech_engine::native_artifact::BuildOwnedImageReader::from_image(bytes).unwrap();
        let image = serde_json::Value::deserialize(&mut reader).unwrap();
        reader.end().unwrap();
        assert_eq!(image["version"], 8);
        let expected = &object.descriptor().metadata().numeric_maps;
        assert_eq!(expected.regions.len(), 1);
        for bound in [false, true] {
            let restored = if bound {
                unsafe {
                    NativeProgramDescriptor::decode_object_bound_image(
                        bytes,
                        &catalog,
                        object.identity().bytes(),
                        object.native_metadata_binding().bytes(),
                    )
                }
            } else {
                unsafe { NativeProgramDescriptor::decode_build_owned_image(bytes, &catalog) }
            }
            .unwrap();
            assert_eq!(&restored.metadata().numeric_maps, expected);
        }
        // A real V6 representation omits this inventory; its ordinary source
        // plan remains authoritative and must rederive the same facts.
        let mut legacy = image.clone();
        legacy["version"] = 6.into();
        legacy["descriptor"]
            .as_object_mut()
            .unwrap()
            .remove("schema_templates");
        legacy["descriptor"]
            .as_object_mut()
            .unwrap()
            .remove("numeric_maps");
        let legacy = mech_engine::native_artifact::encode_native_metadata_image(
            &serde_json::to_vec(&legacy).unwrap(),
        )
        .unwrap();
        assert_eq!(
            &unsafe { NativeProgramDescriptor::decode_build_owned_image(&legacy, &catalog) }
                .unwrap()
                .metadata()
                .numeric_maps,
            expected
        );
        for mutation in 0..16 {
            let mut changed = image.clone();
            let maps = &mut changed["descriptor"]["numeric_maps"];
            match mutation {
                0 => maps["regions"][0]["collection"] = 999.into(),
                1 => maps["regions"][0]["iteration"] = 999.into(),
                2 => maps["regions"][0]["header"] = 999.into(),
                3 => maps["regions"][0]["source"] = 999.into(),
                4 => maps["regions"][0]["result"] = 999.into(),
                5 => maps["regions"][0]["input_width"] = 3.into(),
                6 => maps["regions"][0]["output_width"] = 8.into(),
                7 => maps["regions"][0]["steps"] = serde_json::json!([]),
                8 => maps["regions"][0]["captures"] = serde_json::json!([999]),
                9 => maps["headers"] = serde_json::json!([]),
                10 => {
                    changed["descriptor"]
                        .as_object_mut()
                        .unwrap()
                        .remove("numeric_maps");
                }
                11 => changed["descriptor"]["numeric_maps"] = serde_json::Value::Null,
                12 => changed["version"] = 6.into(),
                13 => {
                    changed["descriptor"]["numeric_maps"] =
                        serde_json::json!({"regions":[],"headers":[]})
                }
                14 => {
                    changed["descriptor"]["visits"][expected.regions[0].header as usize]["work"] =
                        0.into()
                }
                15 => maps["regions"][0]["sites"] = serde_json::json!([]),
                _ => unreachable!(),
            }
            let changed = mech_engine::native_artifact::encode_native_metadata_image(
                &serde_json::to_vec(&changed).unwrap(),
            )
            .unwrap();
            assert!(
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&changed, &catalog) }
                    .is_err(),
                "independent mutation {mutation}"
            );
            assert!(
                unsafe {
                    NativeProgramDescriptor::decode_object_bound_image(
                        &changed,
                        &catalog,
                        object.identity().bytes(),
                        object.native_metadata_binding().bytes(),
                    )
                }
                .is_err(),
                "bound mutation {mutation}"
            );
        }
    }
}

#[test]
fn frozen_schema_templates_preserve_public_metadata_and_reject_type_evidence_forgery() {
    use serde::Deserialize;
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(include_str!(
        "../../../../tests/fixtures/native/numeric-parent-child.mec"
    )) {
        let object = emit_program_object(&artifact, &catalog).unwrap();
        let mut reader = mech_engine::native_artifact::BuildOwnedImageReader::from_image(
            object.native_metadata(),
        )
        .unwrap();
        let image = serde_json::Value::deserialize(&mut reader).unwrap();
        reader.end().unwrap();
        assert_eq!(image["version"], 8);
        let templates = image["descriptor"]["schema_templates"].as_array().unwrap();
        assert_eq!(
            templates.len(),
            object.descriptor().metadata().schemas.len()
        );
        let at = templates
            .iter()
            .position(|t| t["evidence"].as_array().is_some_and(|e| !e.is_empty()))
            .unwrap();
        let expected = object.descriptor().encode_build_owned_image().unwrap();
        let independent = unsafe {
            NativeProgramDescriptor::decode_build_owned_image(object.native_metadata(), &catalog)
        }
        .unwrap();
        assert_eq!(independent.encode_build_owned_image().unwrap(), expected);
        let mut legacy = image.clone();
        legacy["version"] = 7.into();
        legacy["descriptor"]
            .as_object_mut()
            .unwrap()
            .remove("schema_templates");
        let legacy = mech_engine::native_artifact::encode_native_metadata_image(
            &serde_json::to_vec(&legacy).unwrap(),
        )
        .unwrap();
        let legacy =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&legacy, &catalog) }
                .unwrap();
        assert_eq!(legacy.encode_build_owned_image().unwrap(), expected);
        for fault in 0..9 {
            let mut changed = image.clone();
            let rows = &mut changed["descriptor"]["schema_templates"];
            match fault {
                0 => {
                    rows.as_array_mut().unwrap().pop();
                }
                1 => {
                    rows.as_array_mut().unwrap().push(serde_json::Value::Null);
                }
                2 => rows[at] = serde_json::Value::Null,
                3 => rows[at]["evidence"] = serde_json::json!([]),
                4 => {
                    let bits = rows[at]["evidence"][0][1].as_u64().unwrap();
                    rows[at]["evidence"][0][1] = (bits ^ 1).into();
                }
                5 => rows[at]["kind"] = serde_json::json!("Wildcard"),
                6 => rows[at]["extra"] = true.into(),
                7 => {
                    changed["descriptor"]
                        .as_object_mut()
                        .unwrap()
                        .remove("schema_templates");
                }
                _ => changed["version"] = 7.into(),
            }
            let changed = mech_engine::native_artifact::encode_native_metadata_image(
                &serde_json::to_vec(&changed).unwrap(),
            )
            .unwrap();
            // SAFETY: confined negative loader tests. No fabricated metadata
            // reaches an executable descriptor or invokes a native entry.
            assert!(
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&changed, &catalog) }
                    .is_err(),
                "independent fault {fault}"
            );
            assert!(
                unsafe {
                    NativeProgramDescriptor::decode_object_bound_image(
                        &changed,
                        &catalog,
                        object.identity().bytes(),
                        object.native_metadata_binding().bytes(),
                    )
                }
                .is_err(),
                "bound fault {fault}"
            );
        }
    }
}
