use super::*;

fn options(path: &Path) -> BuildOptions {
    BuildOptions {
        paths: vec![path.to_string_lossy().into_owned()],
        emit: BuildEmit::Native,
        cranelift: true,
        native_compile_limits: None,
        native_scalar: false,
        native_loop_fma: false,
        name: Some("canonical-cli-test".into()),
        output_path: None,
        target: None,
        profile: BuildProfile::Release,
        config_path: None,
        no_config: true,
        workspace_root: Some(PathBuf::from(env!("CARGO_MANIFEST_DIR"))),
        keep_project: false,
        offline: true,
        debug: false,
        trace: false,
        rounds_per_step: 10_000,
    }
}
#[test]
fn canonical_options_reject_unsupported_modes_before_reading_inputs() {
    for mode in 0..7 {
        let mut options = options(Path::new("nonexistent.mec"));
        match mode {
            0 => options.emit = BuildEmit::Plan,
            1 => options.target = Some("aarch64-apple-darwin".into()),
            2 => options.native_scalar = true,
            3 => options.native_loop_fma = true,
            4 => options.config_path = Some("nonexistent.mcfg".into()),
            5 => options.debug = true,
            _ => options.trace = true,
        }
        assert!(validate_options(&options).is_err());
    }
}
#[test]
fn retained_source_bundle_survives_deletion_and_bytecode_is_explicitly_unpaired() {
    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("main.mec");
    let dependency = temporary.path().join("dep.mec");
    let text = "+> ./dep.mec\n+> @unused := cli/stdin\nignored := @unused/text\nanswer := dep/value\nanswer\n";
    let dep = "value := \"original unicode é\"\n<+ value\n";
    fs::write(&source, text).unwrap();
    fs::write(&dependency, dep).unwrap();
    let opts = options(&source);
    let (resident, _) = prepare_build_input(&opts, "canonical-cli-test", 0, false).unwrap();
    let (prepared, _) = prepare_build_input(&opts, "canonical-cli-test", 0, true).unwrap();
    assert_eq!(resident.bytecode, prepared.bytecode);
    let (artifact, sources) = match &prepared.origin {
        PreparedBuildOrigin::CanonicalSource {
            _artifact,
            _sources,
            ..
        } => (_artifact, _sources),
        _ => panic!("source compilation lost its original owners"),
    };
    let plan = mech_runtime::CanonicalNativeResourcePlan::select(artifact, 16).unwrap();
    assert_eq!(
        plan.requirements().len(),
        1,
        "unused observation results do not erase original requirements"
    );
    assert_eq!(plan.observations().len(), 1);
    assert!(plan.effects().is_empty());
    let bytes = provenance(artifact, Some(sources)).unwrap();
    let bundle: Json = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(bundle["source_pairing"], "retained-canonical-compilation");
    assert_eq!(bundle["roots"].as_array().unwrap().len(), 1);
    assert_eq!(bundle["dependencies"].as_array().unwrap().len(), 1);
    let root_bytes: Vec<u8> =
        serde_json::from_value(bundle["roots"][0]["source_utf8_bytes"].clone()).unwrap();
    let dep_bytes: Vec<u8> =
        serde_json::from_value(bundle["dependencies"][0]["source_utf8_bytes"].clone()).unwrap();
    assert_eq!(root_bytes, text.as_bytes());
    assert_eq!(dep_bytes, dep.as_bytes());
    fs::write(&dependency, "invalid {{{").unwrap();
    fs::remove_file(&dependency).unwrap();
    fs::remove_file(&source).unwrap();
    assert_eq!(provenance(artifact, Some(sources)).unwrap(), bytes);
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"));
    let paired = generate(
        &prepared,
        workspace,
        &temporary.path().join("paired"),
        "paired",
        NativeCompileLimits::default(),
    )
    .unwrap();
    assert_eq!(
        fs::read(paired.root().join("application.mec")).unwrap(),
        bytes
    );
    assert_eq!(
        fs::read(paired.root().join("program.mecb")).unwrap(),
        prepared.bytecode
    );
    let bytecode = temporary.path().join("original.mecb");
    fs::write(&bytecode, &prepared.bytecode).unwrap();
    let (unpaired, _) = prepare_build_input(&options(&bytecode), "unpaired", 1, true).unwrap();
    assert!(matches!(
        unpaired.origin,
        PreparedBuildOrigin::SourceUnpairedBytecode
    ));
    let decoded = generate(
        &unpaired,
        workspace,
        &temporary.path().join("decoded"),
        "decoded",
        NativeCompileLimits::default(),
    )
    .unwrap();
    let bundle: Json =
        serde_json::from_slice(&fs::read(decoded.root().join("application.mec")).unwrap()).unwrap();
    assert_eq!(bundle["source_pairing"], "source-unpaired-bytecode");
    assert_eq!(bundle["roots"], json!([]));
    assert_eq!(bundle["dependencies"], json!([]));
    assert_eq!(
        paired.metadata()["program_revision"],
        decoded.metadata()["program_revision"]
    );
    assert_eq!(
        fs::read(paired.root().join("object-identity.bin")).unwrap(),
        fs::read(decoded.root().join("object-identity.bin")).unwrap()
    );
    let invalid = PreparedBuildInput::source_unpaired_bytecode(b"not an artifact".to_vec());
    let destination = temporary.path().join("invalid");
    assert!(
        generate(
            &invalid,
            workspace,
            &destination,
            "invalid",
            NativeCompileLimits::default()
        )
        .is_err()
    );
    assert!(
        !destination.exists(),
        "invalid bytecode must not enter another frontend"
    );
}
#[test]
fn discovered_runtime_configuration_is_not_native_launch_authority() {
    let temporary = tempfile::tempdir().unwrap();
    let config = temporary.path().join("mech.mcfg");
    fs::write(
        &config,
        "config := {runtime: {name: \"not-a-launch-policy\"}}\n",
    )
    .unwrap();
    let loaded = crate::load_mech_config_path(config, None).unwrap();
    assert!(validate_prepared_build_config(Some(&loaded), "native", true).is_err());
    assert!(validate_prepared_build_config(Some(&loaded), "resident", false).is_ok());
}

#[test]
fn exact_linked_name_grammar_rejects_before_compilation_without_sanitizing() {
    let too_long = "a".repeat(65);
    for name in [
        "",
        "_leading",
        "has_underscore",
        "1leading",
        "é",
        too_long.as_str(),
    ] {
        assert!(validate_linked_application_binary_name(name).is_err());
        let mut opts = options(Path::new("does-not-exist.mec"));
        opts.name = Some(name.to_owned());
        let error = match run(opts) {
            Err(error) => error,
            Ok(_) => panic!("invalid canonical binary name was accepted"),
        };
        assert!(
            error
                .display_message()
                .contains("canonical linked binary names")
        );
    }
    let longest = "a".repeat(64);
    for name in ["a", "A1-z", longest.as_str()] {
        assert!(validate_linked_application_binary_name(name).is_ok());
    }
}
#[test]
fn ordinary_turn_inputs_are_rejected_without_another_execution_route() {
    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("input.mec");
    fs::write(&source, "answer := input<u64> + 1u64\nanswer\n").unwrap();
    let (prepared, _) = prepare_build_input(&options(&source), "input-test", 0, true).unwrap();
    let destination = temporary.path().join("app");
    let error = match generate(
        &prepared,
        Path::new(env!("CARGO_MANIFEST_DIR")),
        &destination,
        "input-test",
        NativeCompileLimits::default(),
    ) {
        Err(error) => error,
        Ok(_) => panic!("ordinary input unexpectedly acquired a native resource binding"),
    };
    assert!(
        error
            .display_message()
            .contains("bind its application inputs through resources")
    );
    assert!(!destination.exists());
}

#[test]
fn explicit_compile_policy_is_strict_and_separate_from_launch_authority() {
    assert_eq!(
        compile_limits(None).unwrap(),
        NativeCompileLimits::default()
    );
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("compile.json");
    let limits = NativeCompileLimits {
        max_nodes: 12345,
        max_definition_table_bytes: 128 * 1024 * 1024,
        ..Default::default()
    };
    fs::write(&path, serde_json::to_vec(&limits).unwrap()).unwrap();
    assert_eq!(compile_limits(Some(&path)).unwrap(), limits);
    for invalid in [
        r#"{"max_nodes":1}"#,
        r#"{"runtime_grants":[]}"#,
        r#"{"max_nodes":-1}"#,
    ] {
        fs::write(&path, invalid).unwrap();
        assert!(compile_limits(Some(&path)).is_err());
    }
}
