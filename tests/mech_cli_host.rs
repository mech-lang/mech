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
fn write_cli_host_source(root: &std::path::Path) -> std::path::PathBuf {
    let source_path = root.join("cli_host.mec");
    std::fs::write(
        &source_path,
        r#"+> @env := cli/env
+> @out := cli/stdout

@out/line <- @env/MECH_CLI_HOST_TEST
"done"
"#,
    )
    .unwrap();
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
fn mech_run_rejects_retired_interpreter_time_flag() {
    let root = temp_root("retired-time");
    let source_path = write_resident_source(&root);

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("run")
        .arg("--time")
        .arg(&source_path)
        .current_dir(&root)
        .output()
        .unwrap();

    let combined = assert_failure_contains(output, "unexpected argument '--time'");
    let removed_profile_banner = ["Cycle", " Time:"].concat();
    assert!(!combined.contains(&removed_profile_banner), "{combined}");
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_captures_cli_env_and_delivers_stdout_after_resident_commit() {
    let root = temp_root("resident-env-stdout");
    let source_path = write_cli_host_source(&root);

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("run")
        .arg(&source_path)
        .current_dir(&root)
        .env("MECH_CLI_HOST_TEST", "resident-cli-ok")
        .output()
        .unwrap();

    assert_success_contains(output, "resident-cli-ok");
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_delivers_stderr_after_resident_commit() {
    let root = temp_root("resident-stderr");
    let source_path = root.join("stderr.mec");
    std::fs::write(
        &source_path,
        "+> @err := cli/stderr\n@err/line <- \"resident-stderr-ok\"\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("run")
        .arg(&source_path)
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr resident program failed:\n{}",
        combined_output(&output)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("resident-stderr-ok"),
        "expected resident stderr output, got:\n{}",
        combined_output(&output)
    );
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn denied_env_read_rejects_resident_turn_before_stdout_delivery() {
    let root = temp_root("resident-deny-env");
    let source_path = root.join("deny-env.mec");
    std::fs::write(
        &source_path,
        r#"+> @out := cli/stdout
+> @env := cli/env

@out/line <- "must-not-write"
value := @env/HOME
"done"
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("run")
        .arg("--deny-default-capabilities")
        .arg("--capabilities")
        .arg(":cli/stdout")
        .arg("--allow-read")
        .arg(&source_path)
        .arg(&source_path)
        .current_dir(&root)
        .output()
        .unwrap();

    let combined = assert_failure_contains(output, "CapabilityDenied");
    assert!(
        !combined.lines().any(|line| line == "must-not-write"),
        "rejected resident program delivered stdout:\n{combined}"
    );
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn denied_stdout_blocks_resident_output() {
    let root = temp_root("resident-deny-stdout");
    let source_path = root.join("deny-stdout.mec");
    std::fs::write(
        &source_path,
        "+> @out := cli/stdout\n@out/line <- \"denied-output\"\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("run")
        .arg("--deny-default-capabilities")
        .arg("--allow-read")
        .arg(&source_path)
        .arg(&source_path)
        .current_dir(&root)
        .output()
        .unwrap();

    let combined = assert_failure_contains(output, "CapabilityDenied");
    assert!(
        !combined.lines().any(|line| line == "denied-output"),
        "denied resident output was delivered:\n{combined}"
    );
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn path_scoped_cli_env_grant_works() {
    let root = temp_root("resident-env-path");
    std::fs::write(
        root.join("path.mec"),
        "+> @env := cli/env\nvalue := @env/MECH_CLI_SCOPED_ENV\nvalue\n",
    )
    .unwrap();
    std::fs::write(
        root.join("mech.mcfg"),
        r#"config := { run: { grants: [{ target: "cli/env" operations: ["read"] paths: ["MECH_CLI_SCOPED_ENV"] }] } }"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("run")
        .arg("--deny-default-capabilities")
        .arg("path.mec")
        .current_dir(&root)
        .env("MECH_CLI_SCOPED_ENV", "scoped-env-ok")
        .output()
        .unwrap();

    assert_success_contains(output, "scoped-env-ok");
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn path_scoped_cli_stdout_grant_works() {
    let root = temp_root("resident-stdout-path");
    std::fs::write(
        root.join("line.mec"),
        "+> @out := cli/stdout\n@out/line <- \"scoped-stdout-ok\"\n",
    )
    .unwrap();
    std::fs::write(
        root.join("mech.mcfg"),
        r#"config := { run: { grants: [{ target: "cli/stdout" operations: ["write"] paths: ["line"] }] } }"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("run")
        .arg("--deny-default-capabilities")
        .arg("line.mec")
        .current_dir(&root)
        .output()
        .unwrap();

    assert_success_contains(output, "scoped-stdout-ok");
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn configured_cli_host_alias_runs_residently() {
    let root = temp_root("resident-cli-alias");
    std::fs::write(
        root.join("term.mec"),
        "+> @out := term/stdout\n@out/line <- \"resident-alias-ok\"\n",
    )
    .unwrap();
    std::fs::write(
        root.join("mech.mcfg"),
        r#"config := {
  hosts: [{ name: "term" provider: "cli" settings: {} }]
  run: { grants: [{ target: "term/stdout" operations: ["write"] paths: ["line"] }] }
}"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("run")
        .arg("--deny-default-capabilities")
        .arg("term.mec")
        .current_dir(&root)
        .output()
        .unwrap();

    assert_success_contains(output, "resident-alias-ok");
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn resident_cli_source_executes_exactly_once() {
    let root = temp_root("resident-cli-once");
    let source_path = root.join("once.mec");
    std::fs::write(
        &source_path,
        "+> @out := cli/stdout\n@out/line <- \"resident-once\"\n\"done\"\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("run")
        .arg(&source_path)
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "resident CLI source failed:\n{}",
        combined_output(&output)
    );

    let combined = combined_output(&output);
    let count = combined
        .lines()
        .filter(|line| *line == "resident-once")
        .count();
    assert_eq!(
        count, 1,
        "resident source executed {count} times:\n{combined}"
    );
}

#[cfg(all(feature = "run", feature = "build", feature = "cli_host"))]
#[test]
fn resident_cli_source_and_bytecode_have_matching_behavior() {
    let root = temp_root("resident-cli-bytecode");
    let source_path = write_cli_host_source(&root);
    let bytecode_path = root.join("cli_host.mecb");

    let build = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("--no-config")
        .arg("build")
        .arg(&source_path)
        .arg("--emit")
        .arg("bytecode")
        .arg("--out")
        .arg(&bytecode_path)
        .arg("--offline")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "terminal bytecode build failed:\n{}",
        combined_output(&build)
    );

    let execute = |path: &std::path::Path| {
        std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
            .arg("run")
            .arg(path)
            .current_dir(&root)
            .env("MECH_CLI_HOST_TEST", "source-bytecode-match")
            .output()
            .unwrap()
    };
    let source = execute(&source_path);
    let bytecode = execute(&bytecode_path);
    assert!(
        source.status.success(),
        "terminal source execution failed:\n{}",
        combined_output(&source)
    );
    assert!(
        bytecode.status.success(),
        "terminal bytecode execution failed:\n{}",
        combined_output(&bytecode)
    );

    let emitted_line = |output: &std::process::Output| {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| *line == "source-bytecode-match")
            .count()
    };
    assert_eq!(emitted_line(&source), 1);
    assert_eq!(emitted_line(&bytecode), 1);
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

// These actual-command witnesses use canonical scalar output as their oracle.
// Full distribution selects all four gates; a reduced non-compute or
// non-presentation profile must not claim this product coverage.
#[cfg(all(
    feature = "run",
    feature = "cli_host",
    feature = "compute_backends_native",
    feature = "pretty_print"
))]
mod native_mixed_compute {
    use super::{assert_failure_contains, combined_output, temp_root};

    // The existing CLI/env provider supplies one driverless admitted bootstrap.
    // No timer or pointer driver can manufacture additional recurrence turns.
    // The one requested compute turn then publishes 2 * 3 + 1 = 7, and the CLI
    // drains exactly one corresponding telemetry packet before exiting.
    const SOURCE: &str = "\
+> @env := cli/env\n\
seed := @env/MECH_NATIVE_MIXED_COMPUTE_SEED\n\
@compute := compute://filters/kernel{:write(turn), :read(sample/result)}\n\
@compute/turn <- 1\n\
answer := @compute/sample/result\n\
answer\n\
\n\
calculation @compute\n\
-------------------\n\
~counter := 2f32\n\
counter = counter * 3f32 + 1f32\n\
counter\n";

    fn fixture(name: &str) -> std::path::PathBuf {
        let root = temp_root(name);
        std::fs::write(
            root.join("mech.mcfg"),
            r#"config := {
  hosts: [{ name: "filters" provider: "compute" settings: { region: "calculation" backend: "cpu" } }]
  run: { grants: [
    { target: "filters/kernel" operations: ["read", "write"] paths: ["sample/result", "turn"] }
    { target: "cli/env" operations: ["read"] paths: ["MECH_NATIVE_MIXED_COMPUTE_SEED"] }
  ] }
}
"#,
        )
        .unwrap();
        root
    }

    fn execute(root: &std::path::Path, source: &str, inline: bool) -> std::process::Output {
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_mech"));
        command
            .arg("run")
            .arg("--backend")
            .arg("cpu-scalar")
            .arg("--max-live-turns")
            .arg("1")
            .arg("--runtime-info")
            .current_dir(root)
            .env("MECH_NATIVE_MIXED_COMPUTE_SEED", "once");
        if inline {
            // A single argument is essential: this must select InlineSource,
            // not path collection or reconstruction from separately split words.
            command.arg(source);
        } else {
            let path = root.join("mixed.mec");
            std::fs::write(&path, source).unwrap();
            command.arg(path);
        }
        command.output().unwrap()
    }

    fn assert_completed_once(output: std::process::Output) {
        assert!(
            output.status.success(),
            "native mixed CPU command failed:\n{}",
            combined_output(&output)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(
            stdout.lines().filter(|line| *line == "7").count(),
            1,
            "the exact first recurrence value must be printed once:\n{stdout}"
        );
        assert!(stdout.lines().any(|line| line == "f64"), "{stdout}");
        let reports = stdout
            .lines()
            .filter_map(|line| line.strip_prefix("MECH_RUNTIME_INFO "))
            .collect::<Vec<_>>();
        assert_eq!(reports.len(), 1, "{stdout}");
        let info: serde_json::Value = serde_json::from_str(reports[0]).unwrap();
        assert_eq!(info["route"], "resident-external", "{info}");
        assert_eq!(info["routing_policy"], "require-resident", "{info}");
        assert_eq!(info["resident_accepted_turns"], 2, "{info}");
        assert_eq!(info["resident_rejected_turns"], 0, "{info}");
        assert_eq!(info["coalesced_host_packets"], 0, "{info}");
        assert_eq!(info["ignored_host_packets"], 0, "{info}");
        assert_eq!(info["effects"], 1, "{info}");
        assert_eq!(info["observations"], 2, "{info}");
        assert_eq!(info["layout_generation"], 1, "{info}");
        assert_eq!(info["plan_generation"], 1, "{info}");
        assert_eq!(info["program_revision"].as_str().unwrap().len(), 64);
    }

    #[test]
    fn native_mixed_compute_source_path_executes_one_admitted_cpu_turn() {
        let root = fixture("native-mixed-compute-file");
        assert_completed_once(execute(&root, SOURCE, false));
    }

    #[test]
    fn native_mixed_compute_single_argument_inline_executes_one_admitted_cpu_turn() {
        let root = fixture("native-mixed-compute-inline");
        assert_completed_once(execute(&root, SOURCE, true));
    }

    #[test]
    fn native_mixed_compute_positioned_refusal_allows_valid_retry_in_both_entry_modes() {
        // End at the operator: a later identifier must not be eligible for the
        // canonical grammar's accepted multiline expression continuation.
        let malformed = SOURCE.replace("3f32 + 1f32\ncounter\n", "3f32 +\n");
        for inline in [false, true] {
            let root = fixture(if inline {
                "native-mixed-compute-invalid-inline"
            } else {
                "native-mixed-compute-invalid-file"
            });
            let rejected = assert_failure_contains(
                execute(&root, &malformed, inline),
                "syntax/missing-operator-operand",
            );
            assert!(
                rejected.contains("at 11:27"),
                "the missing operand must retain its exact source position:\n{rejected}"
            );
            assert!(
                !rejected.contains("MECH_RUNTIME_INFO "),
                "invalid source must not report an activated program:\n{rejected}"
            );
            assert!(
                !rejected.lines().any(|line| line == "7"),
                "invalid source must not publish the compute result:\n{rejected}"
            );
            // This is a new actual invocation through the same entry point and
            // config/cwd after refusal, not a claim of same-instance recovery.
            assert_completed_once(execute(&root, SOURCE, inline));
        }
    }
}
