use super::{NativeCompilationReport, NativeCompilerConfiguration};
use crate::native_artifact::emit_compact_closed_object_with_configuration_for_diagnostic as compile;
use mech_core::FunctionCatalog;
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn input() -> (ProgramArtifact, std::sync::Arc<FunctionCatalog>) {
    let source = "value := word<u64>\ndelta := delta<u64>\nsum := value + delta\nsum\n";
    compile_source(source, DocumentId(173))
}

fn compile_source(
    source: &str,
    document: DocumentId,
) -> (ProgramArtifact, std::sync::Arc<FunctionCatalog>) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(document, Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let catalog = mech_stdlib::source_catalog();
    let artifact = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            catalog.clone(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    (artifact, catalog)
}

#[test]
fn report_and_compile_workers_preserve_object_and_descriptor() {
    let (artifact, catalog) = input();
    for observe in [false, true] {
        let base = NativeCompilerConfiguration::capture([]);
        let expected = compile(&artifact, &catalog, &base, observe, None).unwrap();
        for workers in ["0", "1", "2", "4"] {
            let config = NativeCompilerConfiguration::capture([(
                "MECH_COMPACT_COMPILE_THREADS".into(),
                workers.into(),
            )]);
            let mut report = NativeCompilationReport::default();
            let actual = compile(&artifact, &catalog, &config, observe, Some(&mut report)).unwrap();
            assert_eq!(actual, expected);
            let json = report.json(&actual.1);
            assert_eq!(json["schema"], "mech.native-compilation-report.v2");
            assert_eq!(json["configuration"]["raw_setting_count"], 1);
            assert_eq!(
                json["configuration"]["resolved_profiles"],
                serde_json::json!([])
            );
            for field in ["raw_settings_sha256", "resolved_profiles_sha256", "sha256"] {
                assert_eq!(json["configuration"][field].as_str().unwrap().len(), 64);
            }
            assert_eq!(json["phase_observer"], observe);
            assert_eq!(json["compile_workers"], workers.parse::<usize>().unwrap());
            assert_eq!(json["object_sizes"]["object_file_bytes"], actual.0.len());
            assert!(
                json["object_sizes"]["function_body_bytes"]
                    .as_u64()
                    .unwrap()
                    > 0
            );
            assert_eq!(json["object_attempts"][0]["status"], "emitted");
            assert!(json["stages"].as_array().unwrap().len() >= 6);
        }
    }
}

#[cfg(target_arch = "aarch64")]
#[test]
fn distinct_alignments_are_isolated_in_alternating_and_concurrent_compiles() {
    let (artifact, catalog) = input();
    let configuration = |alignment: &str| {
        NativeCompilerConfiguration::capture([
            (
                "MECH_COMPACT_FUNCTION_ALIGNMENT_LOG2".into(),
                alignment.into(),
            ),
            ("MECH_COMPACT_COMPILE_THREADS".into(), "2".into()),
        ])
    };
    let a = configuration("4");
    let b = configuration("7");
    let reference_a = compile(&artifact, &catalog, &a, false, None).unwrap();
    let reference_b = compile(&artifact, &catalog, &b, false, None).unwrap();
    assert_ne!(reference_a.0, reference_b.0);
    assert_eq!(
        compile(&artifact, &catalog, &a, false, None).unwrap(),
        reference_a
    );
    assert_eq!(
        compile(&artifact, &catalog, &b, false, None).unwrap(),
        reference_b
    );
    let barrier = std::sync::Barrier::new(2);
    std::thread::scope(|scope| {
        let run = |config: &NativeCompilerConfiguration| {
            let (artifact, catalog) = input();
            barrier.wait();
            compile(&artifact, &catalog, config, false, None).unwrap()
        };
        let first = scope.spawn(move || run(&a));
        let second = scope.spawn(move || run(&b));
        assert_eq!(first.join().unwrap(), reference_a);
        assert_eq!(second.join().unwrap(), reference_b);
    });
    let malformed = configuration("invalid");
    assert!(
        compile(&artifact, &catalog, &malformed, false, None)
            .unwrap_err()
            .to_string()
            .contains("function alignment")
    );
    assert!(compile(&artifact, &catalog, &malformed, true, None).is_ok());
}

#[cfg(target_arch = "aarch64")]
#[test]
fn inactive_profiles_and_early_errors_finish_input_identity() {
    use crate::native_artifact::program::{
        compact::profile_inputs::SelectedProfiles, compiler::emit_compact_with_selected_profiles,
    };

    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<NativeCompilerConfiguration>();
    let (artifact, catalog) = input();
    let mut report = NativeCompilationReport::default();
    let selected = SelectedProfiles::with_reader(|path| panic!("unselected path: {path:?}"));
    let make = |alignment: &str| {
        NativeCompilerConfiguration::capture([
            (
                "MECH_COMPACT_ROOT_TAIL_PROFILE".into(),
                "missing root tail".into(),
            ),
            (
                "MECH_COMPACT_PHASE_BLOCK_LAYOUT".into(),
                "unused@missing layout".into(),
            ),
            (
                "MECH_COMPACT_FUNCTION_ALIGNMENT_LOG2".into(),
                alignment.into(),
            ),
        ])
    };
    let run = |input: NativeCompilerConfiguration, report: &mut NativeCompilationReport| {
        emit_compact_with_selected_profiles(
            &artifact,
            &catalog,
            false,
            &input.0,
            &selected,
            Some(report),
        )
    };
    let (_, metadata) = run(make("4").clone(), &mut report).unwrap();
    let first = report.json(&metadata);
    assert_eq!(first["configuration"]["raw_setting_count"], 3);
    assert!(
        first["configuration"]["resolved_profiles"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let error = run(make("invalid"), &mut report).unwrap_err();
    assert!(error.to_string().contains("function alignment"));
    let failed = report.json(&serde_json::Value::Null);
    assert_ne!(
        failed["configuration"]["raw_settings_sha256"],
        first["configuration"]["raw_settings_sha256"]
    );
    assert_eq!(
        failed["configuration"]["resolved_profiles_sha256"],
        first["configuration"]["resolved_profiles_sha256"]
    );
    assert_ne!(
        failed["configuration"]["sha256"],
        first["configuration"]["sha256"]
    );
    assert!(failed["object_attempts"].as_array().unwrap().is_empty());
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
mod qualified {
    use super::*;
    use crate::native_artifact::{
        identity::hex,
        program::{
            compact::profile_inputs::SelectedProfiles,
            compiler::emit_compact_with_selected_profiles,
        },
    };
    use serde_json::Value;
    use sha2::{Digest, Sha256};
    use std::{
        cell::RefCell,
        collections::{BTreeMap, BTreeSet},
        fs, io,
        path::{Path, PathBuf},
        rc::Rc,
    };

    const LAYOUT: &str = "MECH_COMPACT_PHASE_BLOCK_LAYOUT";
    const WEIGHTS: &str = "MECH_COMPACT_PHASE_REGISTER_WEIGHTS";
    const ROOT_TAIL: &str = "MECH_COMPACT_ROOT_TAIL_PROFILE";
    const VECTOR: &str = "mech_compact_numeric_475_vector";
    type Settings = BTreeMap<String, String>;

    fn digest(bytes: &[u8]) -> String {
        hex(&Sha256::digest(bytes))
    }

    fn checked(path: &Path, hash: &Value) -> Vec<u8> {
        let bytes = fs::read(path).unwrap_or_else(|error| panic!("{path:?}: {error}"));
        assert_eq!(digest(&bytes), hash.as_str().unwrap(), "{path:?}");
        bytes
    }

    fn paths(value: &str, key: &str, mut path: impl FnMut(&str) -> String) -> String {
        if key == ROOT_TAIL {
            return path(value);
        }
        value
            .split(';')
            .map(|entry| {
                let (symbol, relative) = entry.split_once('@').unwrap();
                format!("{symbol}@{}", path(relative))
            })
            .collect::<Vec<_>>()
            .join(";")
    }

    fn expand(
        value: &str,
        key: &str,
        root: &Path,
        hashes: &Value,
        used: &mut BTreeSet<String>,
    ) -> String {
        paths(value, key, |name| {
            let resolved = root.join(name);
            checked(&resolved, &hashes[name]);
            used.insert(name.to_owned());
            resolved.to_str().unwrap().to_owned()
        })
    }

    fn recipe() -> (
        ProgramArtifact,
        std::sync::Arc<FunctionCatalog>,
        Settings,
        Value,
    ) {
        let recipe =
            PathBuf::from(std::env::var_os("NATIVE_COMPILER_QUALIFICATION_RECIPE").unwrap());
        let bench = PathBuf::from(std::env::var_os("NATIVE_COMPILER_BENCH_ROOT").unwrap());
        assert!(recipe.is_absolute() && bench.is_absolute());
        let candidate: Value = serde_json::from_slice(&fs::read(recipe).unwrap()).unwrap();
        let base_dir = bench.join("experiments/mech/mechdeflate-81-v1");
        let base: Value = serde_json::from_slice(&checked(
            &base_dir.join("recipe.json"),
            &candidate["base_recipe_sha256"],
        ))
        .unwrap();
        let mut settings: Settings = serde_json::from_value(base["flags"].clone()).unwrap();
        let mut used = BTreeSet::new();
        for key in [LAYOUT, WEIGHTS] {
            let expanded = expand(
                &settings[key],
                key,
                &base_dir,
                &base["profile_inputs_sha256"],
                &mut used,
            );
            settings.insert(key.to_owned(), expanded);
        }
        assert_eq!(
            used,
            base["profile_inputs_sha256"]
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect()
        );
        let packet = bench.join("experiments/mech/mechdeflate-cranelift-v1");
        let role = &candidate["roles"][candidate["candidate_actor"].as_str().unwrap()];
        used.clear();
        for (key, value) in role["settings_delta"].as_object().unwrap() {
            if value.is_null() {
                assert!(settings.remove(key).is_some());
            } else {
                let value = value.as_str().unwrap();
                let value = if [LAYOUT, WEIGHTS, ROOT_TAIL].contains(&key.as_str()) {
                    expand(
                        value,
                        key,
                        &packet,
                        &role["profile_inputs_sha256"],
                        &mut used,
                    )
                } else {
                    value.to_owned()
                };
                settings.insert(key.clone(), value);
            }
        }
        assert_eq!(
            used,
            role["profile_inputs_sha256"]
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect()
        );
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/native/compact-compiler-regression.mec");
        assert_eq!(
            base["codec_source"],
            "hosts/gpu/tests/fixtures/native/compact-compiler-regression.mec"
        );
        let source = String::from_utf8(checked(&source, &base["codec_source_sha256"])).unwrap();
        let (artifact, catalog) = compile_source(&source, DocumentId(0x50524f47));
        (artifact, catalog, settings, role["artifacts"].clone())
    }

    fn copy_profiles(settings: &Settings, folder: &Path) -> Settings {
        let mut inputs = settings.clone();
        let mut copies = BTreeMap::new();
        for key in [LAYOUT, WEIGHTS, ROOT_TAIL] {
            if let Some(value) = settings.get(key) {
                let expanded = paths(value, key, |original| {
                    let next = copies.len();
                    copies
                        .entry(original.to_owned())
                        .or_insert_with(|| {
                            let dest = folder.join(format!("profile-{next}.json"));
                            fs::copy(original, &dest).unwrap();
                            dest.to_str().unwrap().to_owned()
                        })
                        .clone()
                });
                inputs.insert(key.to_owned(), expanded);
            }
        }
        inputs
    }

    fn capture(settings: &Settings) -> NativeCompilerConfiguration {
        NativeCompilerConfiguration::capture(
            settings
                .iter()
                .map(|(key, value)| (key.into(), value.into())),
        )
    }

    fn profile_path(settings: &Settings, key: &str, symbol: &str) -> PathBuf {
        PathBuf::from(
            settings[key]
                .split(';')
                .find_map(|entry| {
                    let (name, relative) = entry.split_once('@').unwrap();
                    (name == symbol).then_some(relative)
                })
                .unwrap(),
        )
    }

    pub(super) fn run() {
        let (artifact, catalog, settings, expected) = recipe();
        let bytecode = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        assert_eq!(digest(&bytecode), expected["program.mecb"]);
        assert!(settings.contains_key("MECH_COMPACT_DIRECT_ENTRY_LITERALS"));
        let check_artifacts = |mode: &str, actual: &(Vec<u8>, Value)| {
            assert_eq!(digest(&actual.0), expected["program.o"]);
            let expected_descriptor = if mode == "0" {
                "3c88e31701d637597d456d4717b210bd9b19cc5b92b9846eb3bd5ab3414f6ea3"
            } else {
                expected["metadata.json"].as_str().unwrap()
            };
            assert_eq!(
                digest(&serde_json::to_vec(&actual.1).unwrap()),
                expected_descriptor
            );
        };
        for threads in ["0", "1", "2", "4"] {
            let mut inputs = settings.clone();
            inputs.insert("MECH_COMPACT_COMPILE_THREADS".into(), threads.into());
            let baseline = compile(&artifact, &catalog, &capture(&inputs), false, None).unwrap();
            check_artifacts(threads, &baseline);
            for mutation in ["baseline", "replace", "delete"] {
                let folder = tempfile::tempdir().unwrap();
                let local = copy_profiles(&inputs, folder.path());
                let vector = profile_path(&local, LAYOUT, VECTOR);
                let root = PathBuf::from(&local[ROOT_TAIL]);
                let ledger = Rc::new(RefCell::new(BTreeMap::new()));
                let reads = ledger.clone();
                let selected = SelectedProfiles::with_reader(move |path| {
                    *reads.borrow_mut().entry(path.to_owned()).or_insert(0) += 1;
                    let bytes = fs::read(path)?;
                    match mutation {
                        "replace" => fs::write(path, b"replaced after selected read")?,
                        "delete" => fs::remove_file(path)?,
                        _ => (),
                    }
                    Ok(bytes)
                });
                let mut report = NativeCompilationReport::default();
                let output = emit_compact_with_selected_profiles(
                    &artifact,
                    &catalog,
                    false,
                    &capture(&local).0,
                    &selected,
                    Some(&mut report),
                )
                .unwrap();
                assert_eq!(output, baseline, "{threads}/{mutation}");
                assert_eq!(ledger.borrow().get(&vector), Some(&1));
                assert_eq!(ledger.borrow().get(&root), Some(&1));
                assert!(ledger.borrow().values().all(|count| *count == 1));
                let json = report.json(&output.1);
                let profiles = json["configuration"]["resolved_profiles"]
                    .as_array()
                    .unwrap();
                assert_eq!(profiles.len(), ledger.borrow().len());
                assert!(profiles.iter().all(|row| row["sha256"].as_str().is_some()));
                assert_eq!(json["compile_workers"], threads.parse::<usize>().unwrap());
            }
        }

        let folder = tempfile::tempdir().unwrap();
        let stale = copy_profiles(&settings, folder.path());
        let weights = profile_path(&stale, WEIGHTS, VECTOR);
        let mut stale_profile: Value =
            serde_json::from_slice(&fs::read(&weights).unwrap()).unwrap();
        stale_profile["expected_vcode"] = "obsolete VCode".into();
        fs::write(&weights, serde_json::to_vec(&stale_profile).unwrap()).unwrap();
        let error = compile(&artifact, &catalog, &capture(&stale), false, None).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("register frequency profile does not match lowered code"),
            "{error}"
        );

        let folder = tempfile::tempdir().unwrap();
        let local = copy_profiles(&settings, folder.path());
        let missing = profile_path(&local, LAYOUT, VECTOR);
        let bytes = fs::read(&missing).unwrap();
        fs::remove_file(&missing).unwrap();
        let ledger = Rc::new(RefCell::new(Vec::new()));
        let reads = ledger.clone();
        let selected_path = missing.clone();
        let selected = SelectedProfiles::with_reader(move |path| {
            reads.borrow_mut().push(path.to_owned());
            if path == selected_path {
                let error = fs::read(path).unwrap_err();
                assert_eq!(error.kind(), io::ErrorKind::NotFound);
                fs::write(path, &bytes).unwrap();
                Err(error)
            } else {
                fs::read(path)
            }
        });
        let mut report = NativeCompilationReport::default();
        let result = emit_compact_with_selected_profiles(
            &artifact,
            &catalog,
            false,
            &capture(&local).0,
            &selected,
            Some(&mut report),
        );
        assert!(result.is_err());
        assert!(missing.is_file());
        assert_eq!(
            ledger
                .borrow()
                .iter()
                .filter(|path| **path == missing)
                .count(),
            1
        );
        let report = report.json(&Value::Null);
        let attempts = report["object_attempts"].as_array().unwrap();
        assert_eq!(attempts.len(), 2);
        assert_eq!(attempts[0]["status"], "failed");
        assert_eq!(attempts[1]["status"], "failed");
        assert_eq!(attempts[0]["error"], attempts[1]["error"]);
        assert_eq!(attempts[0]["direct_entry_literals"], true);
        assert_eq!(attempts[1]["direct_entry_literals"], false);
        let failure = report["configuration"]["resolved_profiles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["path"] == missing.to_str().unwrap())
            .unwrap();
        assert_eq!(failure["error_kind"], "NotFound");
        assert!(failure["os_error"].as_i64().is_some());
        assert!(failure.get("sha256").is_none());
        let next = compile(&artifact, &catalog, &capture(&local), false, None).unwrap();
        check_artifacts(&settings["MECH_COMPACT_COMPILE_THREADS"], &next);
    }
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
#[ignore = "requires the pinned benchmark checkout and Apple arm64 qualification host"]
fn qualified_encoder_profile_resolution_and_retry() {
    qualified::run();
}
