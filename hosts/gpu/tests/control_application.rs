//! Linked qualification of the native Control object and launcher.
#![cfg(feature = "native-build")]

use mech_gpu::native_build;
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};

const SOURCE: &str = include_str!("fixtures/native/fannkuch-idiomatic.mec");

fn digest(path: &Path) -> String {
    Sha256::digest(fs::read(path).unwrap())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
#[ignore = "links a generated Cargo application; run explicitly with MECH_CONTROL_INLINE=0"]
fn one_linked_control_executable_preserves_signed_machine_outputs() {
    assert_eq!(
        std::env::var("MECH_CONTROL_INLINE").as_deref(),
        Ok("0"),
        "launch this qualification with MECH_CONTROL_INLINE=0 so object emission tests structural callbacks without ordinary inlining"
    );
    let retained = std::env::var_os("MECH_CONTROL_QUALIFICATION_DIRECTORY");
    let root = retained
        .clone()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!(
                "mech-native-control-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
        });
    fs::create_dir(&root).unwrap();
    let source = root.join("application.mec");
    fs::write(&source, SOURCE).unwrap();
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let application = root.join("generated");
    let name = "native_control_qualification";
    let kind = native_build::emit_application(&source, &application, &workspace, name, true, false)
        .unwrap();
    assert_eq!(kind, native_build::ApplicationKind::Control);
    assert_eq!(
        fs::read(application.join("source.mec")).unwrap(),
        SOURCE.as_bytes()
    );
    let object = application.join("mech-control.o");
    assert!(fs::metadata(&object).unwrap().len() > 0);
    let descriptor = application.join("control.json");
    let descriptor_value: serde_json::Value =
        serde_json::from_slice(&fs::read(&descriptor).unwrap()).unwrap();
    assert_eq!(
        descriptor_value["output_signed"],
        serde_json::json!([true, false, false])
    );
    let executable = root.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    native_build::build_application(kind, &application, &executable, name, false, true).unwrap();
    let binary_sha = digest(&executable);
    let mut attempts = Vec::new();
    for (n, expected) in [
        (2, "-1\nPfannkuchen(2) = 1\n"),
        (3, "2\nPfannkuchen(3) = 2\n"),
        (4, "4\nPfannkuchen(4) = 4\n"),
        (5, "11\nPfannkuchen(5) = 7\n"),
        (0, ""),
    ] {
        let output = Command::new(&executable)
            .arg(n.to_string())
            .env("RAYON_NUM_THREADS", "2")
            .output()
            .unwrap();
        let stdout = root.join(format!("n{n}.stdout"));
        let stderr = root.join(format!("n{n}.stderr"));
        fs::write(&stdout, &output.stdout).unwrap();
        fs::write(&stderr, &output.stderr).unwrap();
        if n == 0 {
            assert!(!output.status.success(), "n=0 must fail its source bounds");
            assert!(output.stdout.is_empty());
            assert!(!output.stderr.is_empty());
        } else {
            assert!(
                output.status.success(),
                "n={n}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(output.stdout, expected.as_bytes(), "n={n}");
            assert!(output.stderr.is_empty(), "n={n}");
        }
        assert_eq!(
            digest(&executable),
            binary_sha,
            "executable changed after n={n}"
        );
        attempts.push(serde_json::json!({
            "arguments": [n.to_string()], "exit_code": output.status.code(),
            "stdout_sha256": digest(&stdout), "stderr_sha256": digest(&stderr),
            "stdout_validation": "exact-complete-bytes",
            "stderr_validation": if n == 0 { "nonempty" } else { "empty" }
        }));
    }
    let receipt = serde_json::json!({
        "schema": "native-control-application-qualification-v1",
        "application_kind": "control", "build_count": 1,
        "same_executable_for_all_attempts": true,
        "emission_environment": {"MECH_CONTROL_INLINE": "0"},
        "execution_environment": {"RAYON_NUM_THREADS": "2"},
        "source_fixture": "hosts/gpu/tests/fixtures/native/fannkuch-idiomatic.mec",
        "source_sha256": digest(&source),
        "object_sha256": digest(&object),
        "descriptor_sha256": digest(&descriptor),
        "binary_sha256": binary_sha,
        "attempts": attempts, "benchmark_measurements": 0
    });
    fs::write(
        root.join("qualification.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    if retained.is_some() {
        println!("native control qualification: {}", root.display());
    } else {
        fs::remove_dir_all(root).unwrap();
    }
}
