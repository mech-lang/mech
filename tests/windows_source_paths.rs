#![cfg(windows)]

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_TEMP_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new(label: &str) -> Self {
        let sequence = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must follow the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "Mech Windows ü source path # % spaces {label} {} {sequence} {nanos}",
            std::process::id(),
        ));
        std::fs::create_dir_all(&path).expect("Windows source-path fixture must be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn mech_command(current_dir: &Path, command: &str) -> Command {
    let mut process = Command::new(env!("CARGO_BIN_EXE_mech"));
    process
        .current_dir(current_dir)
        .arg("--no-config")
        .arg(command);
    process
}

fn run_command(mut command: Command, label: &str) -> Output {
    command
        .output()
        .unwrap_or_else(|error| panic!("{label} failed to start: {error}"))
}

fn assert_command_success(label: &str, output: &Output) {
    assert!(
        output.status.success(),
        "{label} failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn assert_no_missing_root_diagnostic(label: &str, output: &Output) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stdout.contains("RuntimeRootModuleSourceNotFound")
            && !stderr.contains("RuntimeRootModuleSourceNotFound"),
        "{label} reported a missing root\nstatus: {}\nstdout:\n{stdout}\nstderr:\n{stderr}",
        output.status,
    );
}

#[cfg(feature = "pretty_print")]
fn assert_final_value(label: &str, output: &Output, expected: &str) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(expected),
        "{label} did not print `{expected}`\nstatus: {}\nstdout:\n{stdout}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );
}

fn write_module_graph(project: &Path, constraint: bool) -> PathBuf {
    let dependency_dir = project.join("lib");
    std::fs::create_dir_all(&dependency_dir).expect("module directory must be created");
    std::fs::write(dependency_dir.join("dep.mec"), "value := 41\n<+ value\n")
        .expect("dependency source must be written");
    let root = project.join("main.mec");
    let source = if constraint {
        "+> ./lib/dep.mec\nanswer := dep/value + 1\nwindows-path-pass! := answer == 42\n"
    } else {
        "+> ./lib/dep.mec\nanswer := dep/value + 1\nanswer\n"
    };
    std::fs::write(&root, source).expect("root source must be written");
    root
}

#[cfg(feature = "run")]
#[test]
fn windows_run_resolves_relative_and_absolute_source_paths() {
    let fixture = TestDirectory::new("run relative absolute");
    let source_dir = fixture
        .path()
        .join("source tree")
        .join("level one")
        .join("level two");
    let working_dir = fixture.path().join("launcher").join("nested");
    std::fs::create_dir_all(&source_dir).unwrap();
    std::fs::create_dir_all(&working_dir).unwrap();
    let source = source_dir.join("main # %.mec");
    std::fs::write(&source, "answer := 42\nanswer\n").unwrap();
    let relative = PathBuf::from("..")
        .join("..")
        .join("source tree")
        .join("level one")
        .join("level two")
        .join("main # %.mec");

    let mut relative_command = mech_command(&working_dir, "run");
    relative_command.arg(&relative);
    let relative_output = run_command(relative_command, "relative Windows run");
    assert_command_success("relative Windows run", &relative_output);
    assert_no_missing_root_diagnostic("relative Windows run", &relative_output);
    #[cfg(feature = "pretty_print")]
    assert_final_value("relative Windows run", &relative_output, "42");

    let absolute = source.canonicalize().unwrap();
    let mut absolute_command = mech_command(&working_dir, "run");
    absolute_command.arg(&absolute);
    let absolute_output = run_command(absolute_command, "absolute Windows run");
    assert_command_success("absolute Windows run", &absolute_output);
    assert_no_missing_root_diagnostic("absolute Windows run", &absolute_output);
    #[cfg(feature = "pretty_print")]
    assert_final_value("absolute Windows run", &absolute_output, "42");
}

#[cfg(feature = "run")]
#[test]
fn windows_run_resolves_relative_module_imports() {
    let fixture = TestDirectory::new("run module import");
    let project = fixture.path().join("nested").join("project # %");
    let root = write_module_graph(&project, false);

    let mut command = mech_command(fixture.path(), "run");
    command.arg(&root);
    let output = run_command(command, "Windows run module graph");

    assert_command_success("Windows run module graph", &output);
    assert_no_missing_root_diagnostic("Windows run module graph", &output);
    #[cfg(feature = "pretty_print")]
    assert_final_value("Windows run module graph", &output, "42");
}

#[cfg(feature = "run")]
#[test]
fn windows_run_accepts_directory_input() {
    let fixture = TestDirectory::new("run directory");
    let project = fixture.path().join("directory input").join("nested");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join("main.mec"), "answer := 42\nanswer\n").unwrap();

    let mut command = mech_command(fixture.path(), "run");
    command.arg(&project);
    let output = run_command(command, "Windows run directory");

    assert_command_success("Windows run directory", &output);
    assert_no_missing_root_diagnostic("Windows run directory", &output);
    #[cfg(feature = "pretty_print")]
    assert_final_value("Windows run directory", &output, "42");
}

#[cfg(feature = "build")]
#[test]
fn windows_build_executes_source_module_graph() {
    let fixture = TestDirectory::new("build module graph");
    let project = fixture.path().join("source tree").join("nested # %");
    let output_dir = fixture.path().join("output tree").join("compiled ü # %");
    let root = write_module_graph(&project, false);

    let mut command = mech_command(fixture.path(), "build");
    command.arg(&root).arg(OsStr::new("--out")).arg(&output_dir);
    let output = run_command(command, "Windows build module graph");
    assert_command_success("Windows build module graph", &output);

    let bytecode = output_dir.join("output.mecb");
    assert!(
        bytecode
            .metadata()
            .map(|metadata| metadata.len())
            .unwrap_or(0)
            > 0,
        "Windows build did not create nonempty bytecode at {}",
        bytecode.display(),
    );

    #[cfg(feature = "run")]
    {
        let mut run = mech_command(fixture.path(), "run");
        run.arg(&bytecode);
        let run_output = run_command(run, "Windows run built bytecode");
        assert_command_success("Windows run built bytecode", &run_output);
    }
}

#[cfg(feature = "test")]
#[test]
fn windows_test_executes_source_module_graph() {
    let fixture = TestDirectory::new("test module graph");
    let project = fixture.path().join("source tests").join("nested # %");
    let report = fixture.path().join("reports ü # %").join("result # %.json");
    std::fs::create_dir_all(report.parent().unwrap()).unwrap();
    let root = write_module_graph(&project, true);

    let mut command = mech_command(fixture.path(), "test");
    command.arg(&root).arg(OsStr::new("--out")).arg(&report);
    let output = run_command(command, "Windows test module graph");
    assert_command_success("Windows test module graph", &output);
    assert!(
        report
            .metadata()
            .map(|metadata| metadata.len())
            .unwrap_or(0)
            > 0,
        "Windows test did not create nonempty report at {}",
        report.display(),
    );
}
