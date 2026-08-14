#[cfg(all(feature = "run", feature = "cli_host"))]
fn temp_root(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "mech-cli-host-{name}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
}

#[cfg(all(feature = "run", feature = "cli_host"))]
fn write_resident_source(root: &std::path::Path) -> std::path::PathBuf {
    let source_path = root.join("resident.mec");
    std::fs::write(&source_path, "answer := 424242\nanswer\n").unwrap();
    source_path
}

#[cfg(all(feature = "run", feature = "cli_host"))]
fn assert_success_contains(output: std::process::Output, expected: &str) {
    assert!(
        output.status.success(),
        "mech command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(expected),
        "expected stdout to contain {expected:?}, got:\n{}",
        stdout,
    );
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_file_execution_runs_resident_source() {
    let root = temp_root("file");
    let source_path = write_resident_source(&root);

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg(&source_path)
        .current_dir(&root)
        .output()
        .unwrap();

    assert_success_contains(output, "424242");
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_subcommand_runs_resident_source() {
    let root = temp_root("run-subcommand");
    let source_path = write_resident_source(&root);

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("run")
        .arg(&source_path)
        .current_dir(&root)
        .output()
        .unwrap();

    assert_success_contains(output, "424242");
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_uses_config_run_paths() {
    let root = temp_root("config-run");
    std::fs::write(
        root.join("cli_host.mec"),
        r#"~state := 0.0
state += 424242.0
output := state
"#,
    )
    .unwrap();
    std::fs::write(
        root.join("mech.mcfg"),
        r#"config := {
  run: {
    paths: ["cli_host.mec"]
  }
}
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("run")
        .current_dir(&root)
        .output()
        .unwrap();

    assert_success_contains(output, "424242");
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_project_directory_uses_config_run_paths() {
    let root = temp_root("project-run");
    let project = root.join("project");
    std::fs::create_dir_all(&project).unwrap();
    write_resident_source(&project);
    std::fs::write(
        project.join("mech.mcfg"),
        r#"config := {
  run: {
    paths: ["resident.mec"]
  }
}
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("project")
        .current_dir(&root)
        .output()
        .unwrap();

    assert_success_contains(output, "424242");
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_without_inputs_and_without_config_errors() {
    let root = temp_root("run-no-inputs");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("run")
        .arg("--no-config")
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(!output.status.success(), "expected mech run to fail");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("no run inputs supplied"),
        "expected clean no-input error, got:\n{}",
        combined,
    );
}

#[cfg(all(feature = "run", feature = "cli_host"))]
fn combined_output(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_directory_ignores_non_mech_assets() {
    let root = temp_root("run-dir-ignore-assets");

    std::fs::write(root.join("main.mec"), "x := 41 + 1\n").unwrap();
    std::fs::write(root.join("app.js"), "console.log('not mech');\n").unwrap();
    std::fs::write(root.join("data.csv"), "a,b\n1,2\n").unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("run")
        .arg(".")
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "directory run should ignore ordinary assets:\n{}",
        combined_output(&output)
    );

    assert!(
        combined_output(&output).contains("42"),
        "expected Mech source result, got:\n{}",
        combined_output(&output)
    );
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_file_resolves_relative_sibling_import() {
    let root = temp_root("run-file-sibling-import");
    std::fs::write(root.join("dep.mec"), "value := 41\n<+ value\n").unwrap();
    std::fs::write(
        root.join("main.mec"),
        "+> ./dep.mec\nanswer := dep/value + 1\nanswer\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("run")
        .arg("main.mec")
        .current_dir(&root)
        .output()
        .unwrap();

    assert_success_contains(output, "42");
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_file_resolves_parent_relative_import() {
    let root = temp_root("run-file-parent-import");
    std::fs::create_dir_all(root.join("app")).unwrap();
    std::fs::create_dir_all(root.join("shared")).unwrap();
    std::fs::write(root.join("shared/dep.mec"), "value := 41\n<+ value\n").unwrap();
    std::fs::write(
        root.join("app/main.mec"),
        "+> ../shared/dep.mec\nanswer := dep/value + 1\nanswer\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("run")
        .arg("app/main.mec")
        .current_dir(&root)
        .output()
        .unwrap();

    assert_success_contains(output, "42");
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_file_resolves_fs_uri_import() {
    let root = temp_root("run-file-fs-import");
    std::fs::create_dir_all(root.join("lib")).unwrap();
    std::fs::write(root.join("lib/dep.mec"), "value := 41\n<+ value\n").unwrap();
    std::fs::write(
        root.join("main.mec"),
        "+> fs://lib/dep.mec\nanswer := dep/value + 1\nanswer\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("run")
        .arg("main.mec")
        .current_dir(&root)
        .output()
        .unwrap();

    assert_success_contains(output, "42");
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_file_missing_import_reports_dependency() {
    let root = temp_root("run-file-missing-import");
    std::fs::write(root.join("main.mec"), "+> ./missing.mec\nanswer := 1\n").unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("run")
        .arg("main.mec")
        .current_dir(&root)
        .output()
        .unwrap();

    let combined = assert_failure_contains(output, "RuntimeModuleDependencyMissing");
    assert!(
        combined.contains("./missing.mec"),
        "missing specifier should appear in output:\n{combined}"
    );
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_file_dependency_denied_by_filesystem_capability() {
    let root = temp_root("run-file-dependency-denied");
    std::fs::write(root.join("dep.mec"), "value := 41\n<+ value\n").unwrap();
    std::fs::write(
        root.join("main.mec"),
        "+> ./dep.mec\nanswer := dep/value + 1\nanswer\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("run")
        .arg("--no-default-capabilities")
        .arg("--allow-read")
        .arg("main.mec")
        .arg("main.mec")
        .current_dir(&root)
        .output()
        .unwrap();

    let combined = assert_failure_contains(output, "Capability");
    assert!(
        combined.contains("resolve") || combined.contains("import"),
        "expected filesystem resolve/import capability denial, got:\n{combined}"
    );
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_rejects_non_resident_text_root() {
    let root = temp_root("run-explicit-js");
    let source = root.join("script.js");
    std::fs::write(&source, "x := 21 + 21\n").unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("run")
        .arg(&source)
        .current_dir(&root)
        .output()
        .unwrap();

    let combined = assert_failure_contains(output, "Unsupported source extension");
    assert!(
        combined.contains("expected one of: mec, 🤖, mecb"),
        "expected the executable-root extension failure, got:\n{combined}"
    );
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_single_quoted_formula_with_slash_is_inline_source() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("run")
        .arg("1 / 2")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "single quoted inline formula with slash should execute as source, not be read as a path:
{}",
        combined_output(&output)
    );
}

#[cfg(all(feature = "run", feature = "cli_host"))]
fn assert_failure_contains(output: std::process::Output, expected: &str) -> String {
    assert!(!output.status.success(), "expected mech command to fail");
    let combined = combined_output(&output);
    assert!(
        combined.contains(expected),
        "expected output to contain {expected:?}, got:\n{combined}"
    );
    combined
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_inline_source_preserves_define_token() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("run")
        .arg("x")
        .arg(":=")
        .arg("1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "inline source with := should not have := filtered out:\n{}",
        combined_output(&output)
    );
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_inline_source_preserves_colon_prefixed_token() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("run")
        .arg(":running")
        .output()
        .unwrap();
    let combined = combined_output(&output);
    assert!(
        !combined.contains("unknown CLI capability profile"),
        "colon-prefixed source token must not be treated as capability profile:\n{combined}"
    );
    assert!(
        !combined.contains("No source files, project paths, or inline code were provided"),
        "colon-prefixed source token must not be dropped from run inputs:\n{combined}"
    );
}
