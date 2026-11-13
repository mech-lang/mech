//! End-to-end qualification of one linked executable under two launch policies.
#![cfg(all(feature = "native-build", feature = "native-io"))]

use mech_gpu::{native_build, sequence::NativeIoGrant};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};

fn digest(path: &Path) -> String {
    Sha256::digest(fs::read(path).unwrap())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn assert_standalone_resolution(
    name: &str,
    text: &str,
    kind: native_build::ApplicationKind,
    expected_feature: &str,
) {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let root = tempfile::tempdir().unwrap();
    let root_path = root.path().canonicalize().unwrap();
    assert!(!root_path.starts_with(&workspace), "{root_path:?}");
    let source = root_path.join("input.mec");
    let application = root_path.join("generated");
    fs::write(&source, text).unwrap();
    assert_eq!(
        native_build::emit_application(&source, &application, &workspace, name, false, false)
            .unwrap(),
        kind
    );

    // Run from the generated workspace so Cargo uses its root configuration.
    let host = cranelift_native::builder().unwrap().triple().to_string();
    let output = Command::new(env!("CARGO"))
        .current_dir(&application)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--offline",
            "--filter-platform",
            &host,
        ])
        .env_remove("CARGO_BUILD_TARGET")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{name}: Cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        Path::new(metadata["workspace_root"].as_str().unwrap())
            .canonicalize()
            .unwrap(),
        application
    );

    let source_manifest = fs::read_to_string(workspace.join("Cargo.toml"))
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    let generated_manifest = fs::read_to_string(application.join("Cargo.toml"))
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    let source_patches = source_manifest["patch"]["crates-io"]
        .as_table_like()
        .unwrap();
    let generated_patches = generated_manifest["patch"]["crates-io"]
        .as_table_like()
        .unwrap();
    for (name, entry) in source_patches.iter() {
        let relative = entry["path"].as_str().unwrap();
        let generated = generated_patches.get(name).unwrap()["path"]
            .as_str()
            .unwrap();
        assert_eq!(
            Path::new(generated),
            workspace.join(relative).canonicalize().unwrap(),
            "{name}"
        );
    }
    for name in ["mech-core", "mech-engine", "mech-syntax", "mech-stdlib"] {
        assert!(generated_patches.get(name).is_some(), "{name}");
    }
    if kind == native_build::ApplicationKind::Sequence {
        for name in ["mech-runtime", "mech-terminal"] {
            assert!(generated_patches.get(name).is_some(), "{name}");
        }
    }

    let packages = metadata["packages"].as_array().unwrap();
    let nodes = metadata["resolve"]["nodes"].as_array().unwrap();
    for (name, path, feature) in [
        (
            "cranelift-codegen",
            "vendor/cranelift-codegen",
            Some("experimental-cmp-reuse"),
        ),
        (
            "cranelift-codegen-meta",
            "vendor/cranelift-codegen-meta",
            None,
        ),
        ("cranelift-frontend", "vendor/cranelift-frontend", None),
        ("regalloc2", "vendor/regalloc2", None),
        ("mech-gpu", "hosts/gpu", Some(expected_feature)),
    ] {
        let matches = packages
            .iter()
            .filter(|package| package["name"] == name)
            .collect::<Vec<_>>();
        let [package] = matches.as_slice() else {
            panic!("{name}: expected one resolved package, got {matches:?}");
        };
        assert!(package["source"].is_null(), "{name}");
        assert_eq!(
            Path::new(package["manifest_path"].as_str().unwrap())
                .canonicalize()
                .unwrap(),
            workspace
                .join(path)
                .join("Cargo.toml")
                .canonicalize()
                .unwrap(),
            "{name}"
        );
        let node = nodes
            .iter()
            .find(|node| node["id"] == package["id"])
            .unwrap();
        if let Some(feature) = feature {
            assert!(
                node["features"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|selected| selected == feature),
                "{name}: {feature} was not selected"
            );
        }
    }
}

#[test]
fn generated_sequence_resolves_workspace_backends() {
    assert_standalone_resolution(
        "native_sequence_resolution",
        "io/write(\"ok\")\n",
        native_build::ApplicationKind::Sequence,
        "sequence-cli",
    );
}

#[test]
fn generated_activated_sequence_resolves_workspace_backends() {
    assert_standalone_resolution(
        "native_activated_resolution",
        "+> @out := cli/stdout\n@out/line <- \"ok\"\n",
        native_build::ApplicationKind::Sequence,
        "native-io",
    );
}

#[test]
fn generated_control_resolves_workspace_backends() {
    assert_standalone_resolution(
        "native_control_resolution",
        include_str!("fixtures/native/fannkuch-idiomatic.mec"),
        native_build::ApplicationKind::Control,
        "sequence-cli",
    );
}

#[test]
fn rejected_declarations_cannot_select_legacy_application_frontends() {
    let root = std::env::temp_dir().join(format!(
        "mech-rejected-native-declarations-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    for (index, declaration) in ["+> *", "+> @out :="].iter().enumerate() {
        let source = root.join(format!("{index}.mec"));
        let text = format!("{declaration}\nio/print(\"legacy bypass\\n\")\n");
        if *declaration == "+> *" {
            assert!(
                mech_engine::native_sequence::control::compile(
                    &text,
                    mech_stdlib::native_control_catalog()
                )
                .is_ok(),
                "the regression must otherwise be accepted by the legacy control frontend"
            );
        }
        fs::write(&source, &text).unwrap();
        if let Ok(parsed) = mech_engine::native_sequence::ParsedSource::parse(&text) {
            assert!(parsed.has_activated_ports(), "{text}");
        }
        let directory = root.join(format!("rejected-{index}"));
        let error = native_build::emit_application(
            &source,
            &directory,
            &workspace,
            "rejected_declaration",
            false,
            false,
        )
        .unwrap_err();
        assert!(
            error.contains("activated native source admission")
                || error.contains("canonical source diagnostics"),
            "{error}"
        );
        assert!(!directory.join("Cargo.toml").exists());
        assert!(!directory.join("src/main.rs").exists());
        assert!(!directory.join("mech-control.o").exists());
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "links an isolated generated Cargo application; run explicitly for native I/O qualification"]
fn one_linked_executable_runs_secure_and_trusted_without_rebuilding() {
    let retained = std::env::var_os("MECH_NATIVE_QUALIFICATION_DIRECTORY");
    let root = retained
        .clone()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!(
                "mech-native-host-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
        });
    fs::create_dir(&root).unwrap();
    let source = root.join("application.mec");
    fs::write(&source, "+> @args := cli/args\n+> @out := cli/stdout\nstep(x<u64>) => <u64> := x + 3\nn := number/parse-u64(@args/1)\nr := n |19> step\n@out/line <- \"result: \" + number/format(r, 0) + \" \" + @args/2\n").unwrap();
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let application = root.join("generated");
    let kind = native_build::emit_application(
        &source,
        &application,
        &workspace,
        "native_host_qualification",
        true,
        false,
    )
    .unwrap();
    assert_eq!(kind, native_build::ApplicationKind::Sequence);
    let executable = root.join(format!(
        "native_host_qualification{}",
        std::env::consts::EXE_SUFFIX
    ));
    native_build::build_application(
        kind,
        &application,
        &executable,
        "native_host_qualification",
        false,
        true,
    )
    .unwrap();
    let binary_sha = digest(&executable);
    let grants = vec![
        NativeIoGrant {
            target: "cli/args".into(),
            operations: vec!["read".into()],
            paths: vec!["1".into(), "2".into()],
        },
        NativeIoGrant {
            target: "cli/stdout".into(),
            operations: vec!["write".into()],
            paths: vec!["line".into()],
        },
    ];
    let grant_file = root.join("grants.json");
    fs::write(&grant_file, serde_json::to_vec_pretty(&grants).unwrap()).unwrap();
    let mut attempts = Vec::new();
    for mode in ["trusted", "secure", "default-denied", "invalid-policy"] {
        let mut command = Command::new(&executable);
        command
            .args(["2", "suffix"])
            .env_remove("MECH_NATIVE_SECURITY")
            .env_remove("MECH_NATIVE_GRANTS")
            .env("MECH_SEQUENCE_PARTITIONS", "1");
        match mode {
            "trusted" => {
                command.env("MECH_NATIVE_SECURITY", "trusted");
            }
            "secure" => {
                command
                    .env("MECH_NATIVE_SECURITY", "secure")
                    .env("MECH_NATIVE_GRANTS", &grant_file);
            }
            "invalid-policy" => {
                command.env("MECH_NATIVE_SECURITY", "invalid");
            }
            _ => {}
        }
        let output = command.output().unwrap();
        fs::write(root.join(format!("{mode}.stdout")), &output.stdout).unwrap();
        fs::write(root.join(format!("{mode}.stderr")), &output.stderr).unwrap();
        if matches!(mode, "secure" | "trusted") {
            assert!(
                output.status.success(),
                "{mode}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(output.stdout, b"result: 59 suffix\n");
            assert!(output.stderr.is_empty());
        } else {
            assert!(!output.status.success(), "{mode}");
            assert!(output.stdout.is_empty());
            assert!(!output.stderr.is_empty());
        }
        assert_eq!(
            digest(&executable),
            binary_sha,
            "executable changed between modes"
        );
        attempts.push(
            serde_json::json!({"mode": mode, "exit_code": output.status.code(),
            "stdout_sha256": digest(&root.join(format!("{mode}.stdout"))),
            "stderr_sha256": digest(&root.join(format!("{mode}.stderr")))}),
        );
    }
    let invalid = root.join("invalid.mec");
    fs::write(&invalid, "+> @out := cli/stdout\nio/write(\"bypass\")\n").unwrap();
    assert!(
        native_build::emit_application(
            &invalid,
            &root.join("rejected"),
            &workspace,
            "rejected_host_bypass",
            true,
            false
        )
        .unwrap_err()
        .contains("activated")
    );
    let receipt = serde_json::json!({
        "schema": "native-host-application-qualification-v1", "build_count": 1,
        "same_executable_for_all_attempts": true, "binary_sha256": binary_sha,
        "source_sha256": digest(&source),
        "object_sha256": digest(&application.join("mech-sequence.o")),
        "constants_sha256": digest(&application.join("constants.json")),
        "requirements_sha256": digest(&application.join("resource-requirements.json")),
        "attempts": attempts, "activated_legacy_fallback_rejected": true,
        "benchmark_measurements": 0
    });
    fs::write(
        root.join("qualification.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    if retained.is_some() {
        println!("native host qualification: {}", root.display());
    } else {
        fs::remove_dir_all(root).unwrap();
    }
}
