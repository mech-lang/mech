#[cfg(all(feature = "run", feature = "cli_host"))]
#[test]
fn mech_file_execution_loads_cli_host_provider() {
    let root = std::env::temp_dir().join(format!(
        "mech-cli-host-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();

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

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg(&source_path)
        .env("MECH_CLI_HOST_TEST", "mech-cli-host-ok")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "mech command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("mech-cli-host-ok"),
        "expected cli stdout send to appear in process stdout, got:\n{}",
        stdout,
    );
}
