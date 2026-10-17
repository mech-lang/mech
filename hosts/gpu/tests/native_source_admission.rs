//! The public source runner must fail closed before selecting legacy I/O.
#![cfg(feature = "native-build")]

use std::{fs, process::Command};

#[test]
fn activated_sources_cannot_select_legacy_runner_frontends() {
    let root = std::env::temp_dir().join(format!(
        "mech-native-source-admission-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    for (index, declaration) in ["+> *", "+> @out := cli/stdout"].iter().enumerate() {
        let text = format!("{declaration}\nio/print(\"legacy bypass\\n\")\n");
        let parsed = mech_engine::native_sequence::ParsedSource::parse(&text).unwrap();
        assert!(parsed.has_activated_ports());
        assert!(
            mech_gpu::sequence::compile_parsed(&parsed, 4096).is_err(),
            "the regression must leave sequence admission"
        );
        if *declaration == "+> *" {
            assert!(
                mech_engine::native_sequence::control::compile_parsed(
                    &parsed,
                    mech_stdlib::native_control_catalog()
                )
                .is_ok(),
                "the regression must otherwise reach legacy control I/O"
            );
        }
        let source = root.join(format!("activated-{index}.mec"));
        fs::write(&source, text).unwrap();
        for mode in ["jit", "resident"] {
            let output = Command::new(env!("CARGO_BIN_EXE_mech-native-source"))
                .arg(&source)
                .arg(mode)
                .output()
                .unwrap();
            assert!(!output.status.success(), "{declaration}: {mode}");
            assert!(output.stdout.is_empty(), "{declaration}: {mode}");
            let error = String::from_utf8(output.stderr).unwrap();
            assert!(
                error.contains("activated native source admission"),
                "{error}"
            );
            assert!(
                error.contains("legacy frontend fallback is forbidden"),
                "{error}"
            );
        }
    }

    // Ordinary sources still use the existing control runner in both modes.
    let source = root.join("ordinary.mec");
    fs::write(&source, "io/print(\"ordinary\\n\")\n").unwrap();
    for mode in ["jit", "resident"] {
        let output = Command::new(env!("CARGO_BIN_EXE_mech-native-source"))
            .arg(&source)
            .arg(mode)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{mode}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"ordinary\n", "{mode}");
    }
    fs::remove_dir_all(root).unwrap();
}
