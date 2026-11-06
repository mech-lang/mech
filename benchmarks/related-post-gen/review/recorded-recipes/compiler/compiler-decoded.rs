use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_gpu::native_artifact::{NativeProgramLimitMode, emit_program_object_with_limits};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::path::Path;

fn compile(text: &str) -> ProgramArtifact {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(71), Revision(1), text).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "source must parse without recovery: {:?}",
        parsed.diagnostics
    );
    CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap()
}
fn run() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 2, "compiler SOURCE BUILD_DIRECTORY");
    let root = Path::new(&args[1]);
    std::fs::create_dir_all(root).unwrap();
    let source = std::fs::read_to_string(&args[0]).unwrap();
    let original = compile(&source);
    let artifact = mech_engine::decode_program_artifact_bytecode_v1(&mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
    let object = emit_program_object_with_limits(
        &artifact,
        &mech_stdlib::source_catalog(),
        NativeProgramLimitMode::NoLimits.compilation(),
    )
    .unwrap();
    std::fs::write(root.join("source.mec"), source).unwrap();
    std::fs::write(
        root.join("artifact.mecb"),
        mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    std::fs::write(root.join("object.o"), object.bytes()).unwrap();
    std::fs::write(
        root.join("metadata.json"),
        serde_json::to_vec_pretty(&object.metadata()).unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join("descriptor.json"),
        object.descriptor().encode_build_owned_image().unwrap(),
    )
    .unwrap();
    let plan = object.descriptor().plan();
    let profile = mech_engine::linked_factories::NativeFactoryLinkageProfile::derive_native_image(plan, &mech_stdlib::source_catalog());
    let factories = profile.factories().expect("every native-image factory must have its maintained private linkage witness");
    let installers = factories.iter().map(|factory| format!("    {}.install(&mut builder).map_err(|e| format!(\"{{e:?}}\"))?;",factory.rust_path())).collect::<Vec<_>>().join("\n");
    let code = format!("fn linked_catalog() -> Result<mech_core::FunctionCatalog,String> {{\n    let mut builder = mech_core::FunctionCatalogBuilder::new();\n{installers}\n    builder.build().map_err(|e| format!(\"{{e:?}}\"))\n}}\n");
    std::fs::write(root.join("catalog.rs"),code).unwrap();
    std::fs::write(root.join("catalog-profile.json"),serde_json::to_vec_pretty(&serde_json::json!({"basis":profile.selection_basis(),"factories":factories.iter().map(|f|f.rust_path()).collect::<Vec<_>>(),"operations":profile.operations().iter().map(|o|o.canonical_name()).collect::<Vec<_>>(),"missing":profile.missing_operations().iter().map(|o|o.canonical_name()).collect::<Vec<_>>()})).unwrap()).unwrap();

    std::fs::write(root.join("map-facts.txt"),format!("owners={:#?}\nwrites={:#?}",plan.map_owner_regions(),plan.map_write_uses())).unwrap();
    let (frames, owners) = plan.facts().buffer_frame_owner_layouts().unwrap();
    let actions = plan.blocks().iter().enumerate().filter_map(|(site, _)| {
        frames.action(site).map(|action| serde_json::json!({"site":site,"operation":format!("{:?}",action.operation())}))
    }).collect::<Vec<_>>();
    let retained_maps = owners.regions().iter().map(|region| serde_json::json!({
        "iteration":region.iteration(),"reads":region.reads(),"writes":region.writes(),
        "exit":region.tail().exit(),"result":region.tail().result().get()
    })).collect::<Vec<_>>();
    std::fs::write(root.join("frame-summary.json"), serde_json::to_vec_pretty(&serde_json::json!({"scalar_fields":frames.scalar_fields(),"managed_fields":frames.managed_fields(),"actions":actions,"retained_map_regions":retained_maps,"uniform_unsigned_collections":plan.facts().uniform_unsigned_collections()})).unwrap()).unwrap();
    let metadata = serde_json::to_value(object.metadata()).unwrap();
    let identity = metadata["identity_symbol"].as_str().unwrap();
    let binding = metadata["image_binding_symbol"].as_str().unwrap();
    std::fs::write(root.join("image-binding.bin"),object.image_binding().bytes()).unwrap();
    std::fs::write(root.join("native-metadata.bin"), object.native_metadata()).unwrap();
    std::fs::write(root.join("native-metadata-binding.bin"), object.native_metadata_binding().bytes()).unwrap();
    let native_symbol = object.native_metadata_symbol();
    let native_binding = object.native_metadata_binding().symbol();
    let native_bytes = object.native_metadata().len();
    let module_symbol = object.compiled_module_symbol();
    std::fs::write(root.join("linked.rs"), format!("unsafe extern \"C\" {{\n #[link_name=\"{identity}\"] static LINKED_IDENTITY:[u8;mech_gpu::native_artifact::NATIVE_PROGRAM_LINK_IDENTITY_BYTES];\n #[link_name=\"{binding}\"] static LINKED_IMAGE_BINDING:[u8;mech_gpu::native_artifact::NATIVE_PROGRAM_IMAGE_BINDING_BYTES];\n #[link_name=\"{native_symbol}\"] static LINKED_NATIVE_METADATA:[u8;{native_bytes}];\n #[link_name=\"{native_binding}\"] static LINKED_NATIVE_METADATA_BINDING:[u8;mech_gpu::native_artifact::NATIVE_PROGRAM_IMAGE_BINDING_BYTES];\n #[link_name=\"mech_canonical_program_entry_v1\"] fn linked_entry(frame:*mut std::ffi::c_void)->u64;\n #[link_name=\"{module_symbol}\"] static LINKED_MODULE:mech_gpu::native_artifact::NativeProgramModule;\n}}\n")).unwrap();
    println!(
        "{}",
        serde_json::json!({"code_bytes":object.descriptor().code_bytes(), "stack_bytes":object.descriptor().stack_bytes(), "inputs":artifact.inputs().iter().map(|p| mech_engine::decode_source_input_name(&p.name)).collect::<Vec<_>>(), "outputs":artifact.outputs().len()})
    );
}
fn main() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}
