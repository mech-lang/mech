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
fn mech_file_execution_loads_cli_host_provider() {
  let root = temp_root("file");
  let source_path = write_cli_host_source(&root);

  let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
    .arg(&source_path)
    .env("MECH_CLI_HOST_TEST", "mech-cli-host-ok")
    .output()
    .unwrap();

  assert_success_contains(output, "mech-cli-host-ok");
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_subcommand_loads_cli_host_provider() {
  let root = temp_root("run-subcommand");
  let source_path = write_cli_host_source(&root);

  let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
    .arg("run")
    .arg(&source_path)
    .env("MECH_CLI_HOST_TEST", "mech-cli-host-ok")
    .output()
    .unwrap();

  assert_success_contains(output, "mech-cli-host-ok");
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_uses_config_run_paths() {
  let root = temp_root("config-run");
  write_cli_host_source(&root);
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
    .env("MECH_CLI_HOST_TEST", "mech-config-run-ok")
    .output()
    .unwrap();

  assert_success_contains(output, "mech-config-run-ok");
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_project_directory_uses_config_run_paths() {
  let root = temp_root("project-run");
  let project = root.join("project");
  std::fs::create_dir_all(&project).unwrap();
  write_cli_host_source(&project);
  std::fs::write(
    project.join("mech.mcfg"),
    r#"config := {
  run: {
    paths: ["cli_host.mec"]
  }
}
"#,
  )
  .unwrap();

  let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
    .arg("project")
    .current_dir(&root)
    .env("MECH_CLI_HOST_TEST", "mech-project-run-ok")
    .output()
    .unwrap();

  assert_success_contains(output, "mech-project-run-ok");
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_profile_output_comes_from_runtime_event() {
  let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
    .arg("--time")
    .arg("1 + 1")
    .output()
    .unwrap();

  assert_success_contains(output, "Cycle Time:");
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

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_explicit_stdout_profile_permits_stdout() {
  let root = temp_root("profile-stdout");
  let source = root.join("stdout.mec");
  std::fs::write(
    &source,
    "+> @out := cli/stdout
@out/line <- \"stdout-profile-ok\"
",
  )
  .unwrap();

  let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
    .arg("run")
    .arg("--deny-default-capabilities")
    .arg("--capabilities")
    .arg(":cli/stdout")
    .arg(&source)
    .output()
    .unwrap();

  assert_success_contains(output, "stdout-profile-ok");
}


#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_capability_passthrough_file_runs_once() {
  let root = temp_root("cap-passthrough-once");
  let source = root.join("once.mec");
  std::fs::write(
    &source,
    "+> @out := cli/stdout
@out/line <- \"cap-passthrough-once\"
\"done\"
",
  )
  .unwrap();

  let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
    .arg("run")
    .arg("--deny-default-capabilities")
    .arg("--capabilities")
    .arg(":cli/stdout")
    .arg(&source)
    .output()
    .unwrap();

  assert!(
    output.status.success(),
    "expected command to succeed:
{}",
    combined_output(&output)
  );

  let combined = combined_output(&output);
  let count = combined.matches("cap-passthrough-once").count();
  assert_eq!(
    count, 1,
    "source file should execute exactly once, got {count} occurrences:
{combined}"
  );
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_single_capabilities_arg_accepts_stdout_and_env_profiles() {
  let root = temp_root("profile-stdout-env");
  let source = root.join("stdout_env.mec");
  std::fs::write(
    &source,
    r#"+> @env := cli/env
+> @out := cli/stdout

@out/line <- @env/MECH_CLI_HOST_TEST
"done"
"#,
  )
  .unwrap();

  let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
    .arg("run")
    .arg("--deny-default-capabilities")
    .arg("--capabilities")
    .arg(":cli/stdout")
    .arg("--capabilities")
    .arg(":cli/env")
    .arg(&source)
    .env("MECH_CLI_HOST_TEST", "stdout-env-profile-ok")
    .output()
    .unwrap();

  assert_success_contains(output, "stdout-env-profile-ok");
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_capabilities_preserves_inline_colon_syntax() {
  let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
    .arg("run")
    .arg("--deny-default-capabilities")
    .arg("--capabilities")
    .arg(":cli/stdout")
    .arg("x := 1")
    .output()
    .unwrap();

  assert!(
    output.status.success(),
    "inline source after --capabilities should run successfully:
{}",
    combined_output(&output)
  );
  let combined = combined_output(&output);
  assert!(
    !combined.contains("unknown") && !combined.contains(":="),
    "inline := token should not be parsed as a capability profile:
{combined}"
  );
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_explicit_stdout_profile_denies_env_before_write() {
  let root = temp_root("profile-stdout-deny-env");
  let source = root.join("stdout_env.mec");
  std::fs::write(
    &source,
    r#"+> @out := cli/stdout
+> @env := cli/env

@out/line <- "must-not-write"
x := @env/HOME
"done"
"#,
  )
  .unwrap();

  let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
    .arg("run")
    .arg("--deny-default-capabilities")
    .arg("--capabilities")
    .arg(":cli/stdout")
    .arg(&source)
    .output()
    .unwrap();

  let combined = assert_failure_contains(output, "RuntimeCapabilityGrantDenied");
  assert!(
    !combined.contains("must-not-write"),
    "provider wrote denied string: {combined}"
  );
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_unknown_capability_profile_fails() {
  let root = temp_root("unknown-profile");
  let source = root.join("stdout.mec");
  std::fs::write(
    &source,
    "+> @out := cli/stdout
@out/line <- \"should-not-run\"
",
  )
  .unwrap();

  let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
    .arg("run")
    .arg("--deny-default-capabilities")
    .arg("--capabilities")
    .arg(":quxx")
    .arg(&source)
    .output()
    .unwrap();

  let combined = assert_failure_contains(output, "invalid value ':quxx' for '--capabilities <CAPABILITY>'");
  assert!(
    !combined.contains("should-not-run"),
    "program ran despite unknown profile: {combined}"
  );
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_config_can_deny_stdout() {
  let root = temp_root("config-deny-stdout");
  std::fs::write(
    root.join("cli_host.mec"),
    "+> @out := cli/stdout\n@out/line <- \"denied-by-config\"\n",
  )
  .unwrap();
  std::fs::write(
    root.join("mech.mcfg"),
    r#"config := {
  run: {
    paths: ["cli_host.mec"]
    cli: { stdout: { write: [] } }
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

  assert_failure_contains(output, "RuntimeCapabilityGrantDenied");
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_config_can_narrow_stdout_to_line() {
  let root = temp_root("config-stdout-line");
  std::fs::write(
    root.join("line.mec"),
    "+> @out := cli/stdout\n@out/line <- \"line-ok\"\n",
  )
  .unwrap();
  std::fs::write(
    root.join("text.mec"),
    "+> @out := cli/stdout\n@out/text <- \"text-bad\"\n",
  )
  .unwrap();
  std::fs::write(
    root.join("mech.mcfg"),
    r#"config := { run: { cli: { stdout: { write: ["line"] } } } }"#,
  )
  .unwrap();

  let ok = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
    .arg("run")
    .arg("line.mec")
    .current_dir(&root)
    .output()
    .unwrap();
  assert_success_contains(ok, "line-ok");

  let bad = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
    .arg("run")
    .arg("text.mec")
    .current_dir(&root)
    .output()
    .unwrap();
  assert_failure_contains(bad, "RuntimeCapabilityGrantDenied");
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_config_can_narrow_env_to_path() {
  let root = temp_root("config-env-path");
  std::fs::write(
    root.join("path.mec"),
    "+> @env := cli/env\nx := @env/PATH\n\"ok\"\n",
  )
  .unwrap();
  std::fs::write(
    root.join("home.mec"),
    "+> @env := cli/env\nx := @env/HOME\n\"bad\"\n",
  )
  .unwrap();
  std::fs::write(
    root.join("mech.mcfg"),
    r#"config := { run: { cli: { env: { read: ["PATH"] } } } }"#,
  )
  .unwrap();

  let ok = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
    .arg("run")
    .arg("path.mec")
    .current_dir(&root)
    .output()
    .unwrap();
  assert!(
    ok.status.success(),
    "PATH read failed:\n{}",
    combined_output(&ok)
  );

  let bad = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
    .arg("run")
    .arg("home.mec")
    .current_dir(&root)
    .output()
    .unwrap();
  assert_failure_contains(bad, "RuntimeCapabilityGrantDenied");
}
