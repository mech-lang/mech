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
#[test]
fn mech_run_accepts_single_cli_host_capability_arg_without_clap_panic() {
  let root = temp_root("cap-single");
  let source_path = root.join("stdout_only.mec");
  std::fs::write(
    &source_path,
    r#"+> @out := cli/stdout
@out/line <- "stdout-only-ok"
"#,
  )
  .unwrap();

  let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
    .arg("run")
    .arg("--deny-default-capabilities")
    .arg("--capabilities")
    .arg(":cli/stdout")
    .arg(&source_path)
    .output()
    .unwrap();

  assert_success_contains(output, "stdout-only-ok");
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_accepts_multi_value_cli_host_capability_arg() {
  let root = temp_root("cap-multi");
  let source_path = write_cli_host_source(&root);

  let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
    .arg("run")
    .arg("--deny-default-capabilities")
    .arg("--capabilities")
    .arg(":cli/stdout")
    .arg(":cli/env")
    .arg(&source_path)
    .env("MECH_CLI_HOST_TEST", "multi-capability-ok")
    .output()
    .unwrap();

  assert_success_contains(output, "multi-capability-ok");
}

#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_run_unknown_cli_host_capability_profile_errors() {
  let root = temp_root("cap-unknown");
  let source_path = write_cli_host_source(&root);

  let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
    .arg("run")
    .arg("--deny-default-capabilities")
    .arg("--capabilities")
    .arg(":quxx")
    .arg(&source_path)
    .output()
    .unwrap();

  assert!(!output.status.success(), "unknown profile should fail");
  let combined = format!(
    "{}{}",
    String::from_utf8_lossy(&output.stdout),
    String::from_utf8_lossy(&output.stderr)
  );
  assert!(
    combined.contains("unknown CLI capability profile `:quxx`"),
    "expected unknown profile error, got:\n{}",
    combined,
  );
}
