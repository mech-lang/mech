use crate::*;
use mech_core::*;
use mech_program::*;
use serde::Serialize;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};

use crate::source_discovery::{
  collect_sources_with_events,
  DedupePolicy,
  DiscoveryOptions,
  MissingPathPolicy,
};

const TEST_EXPLICIT_EXTENSIONS: &[&str] = &["mec", "🤖", "mecb"];
const TEST_RECURSIVE_EXTENSIONS: &[&str] = &["mec", "🤖"];
const TEST_SKIP_DIRS: &[&str] = &["target", ".git", "dist", "out"];

fn collect_test_targets(path: &Path) -> MResult<Vec<PathBuf>> {
  let base_dir = if path.is_dir() {
    path
  } else {
    path.parent().unwrap_or_else(|| Path::new(""))
  };

  let discovery = collect_sources_with_events(
    &[path.to_path_buf()],
    base_dir,
    DiscoveryOptions {
      allowed_file_extensions: TEST_EXPLICIT_EXTENSIONS,
      recursive_file_extensions: TEST_RECURSIVE_EXTENSIONS,
      skip_dir_names: TEST_SKIP_DIRS,
      follow_file_symlinks: false,
      follow_dir_symlinks: false,
      missing_path_policy: MissingPathPolicy::SkipBrokenSymlink,
      dedupe_policy: DedupePolicy::CanonicalPath,
    },
  )?;

  let mut targets = discovery
    .entries
    .into_iter()
    .map(|entry| entry.logical_path)
    .collect::<Vec<_>>();

  targets.sort();
  Ok(targets)
}

fn is_bytecode_test_path(path: &Path) -> bool {
  matches!(mech_runtime::SourceKind::from_path(path), mech_runtime::SourceKind::MechBytecode)
}

fn bytecode_test_unsupported_error(path: &str) -> MechError {
  MechError::new(
    GenericError {
      msg: format!(
        "Bytecode test input `{}` is not supported because compiled bytecode does not currently include invariant metadata. Run tests from source files instead.",
        path
      ),
    },
    None,
  ).with_compiler_loc()
}

// Test
// -----------------------------------------------------------------------------

#[derive(Debug, Serialize, Clone)]
struct CaseDetail {
  name: String,
  expression: String,
  reason: String,
  #[serde(rename = "evaluated-kind")]
  evaluated_kind: String,
  actual: String,
  expected: String,
}

#[derive(Debug, Serialize)]
struct FileResult {
  total: usize,
  passed: usize,
  failed: usize,
}

#[derive(Debug, Serialize)]
struct FileReport {
  path: String,
  result: FileResult,
  failed: Vec<CaseDetail>,
  passed: Vec<CaseDetail>,
  #[serde(rename = "run-error")]
  run_error: Option<String>,
}

#[derive(Debug, Serialize)]
struct SummaryResult {
  #[serde(rename = "files-total")]
  files_total: usize,
  #[serde(rename = "files-passed")]
  files_passed: usize,
  #[serde(rename = "files-failed")]
  files_failed: usize,
  #[serde(rename = "tests-total")]
  tests_total: usize,
  #[serde(rename = "tests-passed")]
  tests_passed: usize,
  #[serde(rename = "tests-failed")]
  tests_failed: usize,
}

#[derive(Debug, Serialize)]
struct TestReport {
  result: SummaryResult,
  files: Vec<FileReport>,
}

impl FileReport {
  fn failed_file(&self) -> bool {
    self.run_error.is_some() || self.result.failed > 0
  }
}

impl SummaryResult {
  fn failed_run(&self) -> bool {
    self.files_failed > 0 || self.tests_failed > 0
  }
}

impl TestReport {
  fn status_label(&self) -> &'static str {
    if self.result.failed_run() { "FAILED" } else { "SUCCESS" }
  }

  fn exit_code(&self) -> i32 {
    if self.result.failed_run() { 1 } else { 0 }
  }
}
#[derive(Debug, Serialize)]
struct NamedCase {
  name: String,
}
#[derive(Debug, Serialize)]
struct FileReportOut {
  path: String,
  result: FileResult,
  failed: Vec<CaseDetail>,
  passed: Vec<NamedCase>,
  #[serde(rename = "run-error")]
  run_error: Option<String>,
}
#[derive(Debug, Serialize)]
struct TestReportOut {
  result: SummaryResult,
  files: Vec<FileReportOut>,
}

fn mech_bool(v: bool) -> &'static str { if v { "✓" } else { "✗" } }
fn mech_str(v: &str) -> String { format!("{:?}", v) }
fn mech_kind(v: &str) -> String { format!("<{}>", v) }
fn indent_block(block: &str, spaces: usize) -> String {
  let pad = " ".repeat(spaces);
  block.lines().map(|line| format!("{pad}{line}")).collect::<Vec<_>>().join("\n")
}
fn case_to_mech(c: &CaseDetail) -> String {
  format!(
    "{{\n  name: {}\n  expression: {}\n  reason: {}\n  evaluated-kind: {}\n  actual: {}\n  expected: {}\n}}",
    mech_str(&c.name), mech_str(&c.expression), mech_str(&c.reason), mech_kind(&c.evaluated_kind), mech_str(&c.actual), mech_str(&c.expected)
  )
}
fn file_to_mech(file: &FileReport, verbose: bool) -> String {
  let failed_items = file.failed.iter().map(case_to_mech).collect::<Vec<_>>().join("\n");
  let passed_items = if verbose {
    file.passed.iter().map(case_to_mech).collect::<Vec<_>>().join("\n")
  } else {
    file.passed.iter().map(|p| format!("{{\n  name: {}\n}}", mech_str(&p.name))).collect::<Vec<_>>().join("\n")
  };
  let run_error = file.run_error.as_ref().map(|e| mech_str(e)).unwrap_or("_".to_string());
  format!(
    "{{\n  path: {}\n  result: {{\n    total: {}\n    passed: {}\n    failed: {}\n  }}\n  failed: {{\n{}\n  }}\n  passed: {{\n{}\n  }}\n  run-error: {}\n}}",
    mech_str(&file.path),
    file.result.total, file.result.passed, file.result.failed,
    if failed_items.is_empty() { "".to_string() } else { indent_block(&failed_items, 4) },
    if passed_items.is_empty() { "".to_string() } else { indent_block(&passed_items, 4) },
    run_error
  )
}
fn report_to_mech(report: &TestReport, verbose: bool) -> String {
  let files = report.files.iter().map(|f| file_to_mech(f, verbose)).collect::<Vec<_>>().join("\n");
  format!(
    "{{\n  result: {{\n    files-total: {}\n    files-passed: {}\n    files-failed: {}\n    tests-total: {}\n    tests-passed: {}\n    tests-failed: {}\n  }}\n  files: {{\n{}\n  }}\n}}",
    report.result.files_total, report.result.files_passed, report.result.files_failed, report.result.tests_total, report.result.tests_passed, report.result.tests_failed,
    indent_block(&files, 4)
  )
}

fn report_to_json(report: &TestReport, verbose: bool) -> Result<String, io::Error> {
  if verbose {
    serde_json::to_string_pretty(report).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
  } else {
    let out = TestReportOut {
      result: SummaryResult {
        files_total: report.result.files_total,
        files_passed: report.result.files_passed,
        files_failed: report.result.files_failed,
        tests_total: report.result.tests_total,
        tests_passed: report.result.tests_passed,
        tests_failed: report.result.tests_failed,
      },
      files: report.files.iter().map(|f| FileReportOut {
        path: f.path.clone(),
        result: FileResult { total: f.result.total, passed: f.result.passed, failed: f.result.failed },
        failed: f.failed.clone(),
        passed: f.passed.iter().map(|p| NamedCase { name: p.name.clone() }).collect(),
        run_error: f.run_error.clone(),
      }).collect(),
    };
    serde_json::to_string_pretty(&out).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
  }
}

pub fn run_mech_tests(
  mech_paths: Vec<String>,
  tree_flag: bool,
  debug_flag: bool,
  time_flag: bool,
  trace_flag: bool,
  output_path: Option<String>,
  verbose: bool,
) -> Result<i32, MechError> {
  let mut expanded_paths = Vec::new();
  for input in mech_paths {
    let input_path = Path::new(&input);
    let targets = collect_test_targets(input_path)?;
    for target in targets {
      expanded_paths.push(target.display().to_string());
    }
  }

  let mut file_reports = Vec::new();
  println!("{} Running tests...\n", "[Test]".truecolor(153, 221, 85));
  for path in &expanded_paths {
    let uuid = generate_uuid();
    let mut program = MechProgram::new(MechProgramConfig {
      name: format!("test-{}", uuid),
      environment: MechProgramEnvironment::default(),
    });
    let _ = tree_flag;
    program.configure(debug_flag, trace_flag, time_flag, 10_000);
    if is_bytecode_test_path(Path::new(path)) {
      let err = bytecode_test_unsupported_error(path);
      eprintln!("{} {}", "[Error]".truecolor(246,98,78), err.display_message());
      file_reports.push(FileReport { path: path.clone(), result: FileResult{total:0,passed:0,failed:0}, failed: vec![], passed: vec![], run_error: Some(err.display_message()) });
      continue;
    }
    let source = match mech_runtime::read_runtime_source_file(Path::new(path)) {
      Ok(source) => source,
      Err(err) => {
        let err = MechError::new(
          GenericError {
            msg: format!("Unable to read test source `{}`: {:?}", path, err),
          },
          None,
        )
        .with_compiler_loc();
        eprintln!("{} {}", "[Error]".truecolor(246,98,78), err.display_message());
        file_reports.push(FileReport { path: path.clone(), result: FileResult{total:0,passed:0,failed:0}, failed: vec![], passed: vec![], run_error: Some(err.display_message()) });
        continue;
      }
    };
    if let Err(err) = program.run_source(&source) {
      eprintln!("{} {}", "[Error]".truecolor(246,98,78), err.display_message());
      file_reports.push(FileReport { path: path.clone(), result: FileResult{total:0,passed:0,failed:0}, failed: vec![], passed: vec![], run_error: Some(err.display_message()) });
      continue;
    }

    let state = &program.interpreter().state.borrow();
    println!("{} {}\n", "[Test]".truecolor(153, 221, 85), path);

    let mut violations: HashMap<u64, CaseDetail> = HashMap::new();
    for v in &state.invariant_violations {
      if let Some(inv) = v.error.kind_as::<InvariantViolationError>() {
        violations.insert(v.id, CaseDetail {
          name: state.invariants.get(&v.id).map(|(n, _)| n.clone()).unwrap_or_else(|| format!("#{}", v.id)),
          expression: inv.expression.clone(),
          reason: inv.reason.clone(),
          evaluated_kind: inv.evaluated_kind.clone(),
          actual: inv.lhs_value.clone().unwrap_or_else(|| "?".to_string()),
          expected: inv.rhs_value.clone().unwrap_or_else(|| "?".to_string()),
        });
      }
    }

    let mut passed_cases = Vec::new();
    let mut failed_cases = Vec::new();
    let width = state.invariants.values().map(|(n, _)| n.len()).max().unwrap_or(0);
    for (id, (name, value)) in state.invariants.iter() {
      match &*value.borrow() {
        Value::Bool(b) if *b.borrow() => {
          println!("{:<width$}   ✓", name, width=width);
          passed_cases.push(CaseDetail {
            name: name.clone(),
            expression: state.invariant_expressions.get(id).cloned().unwrap_or_else(|| name.clone()),
            reason: state.invariant_evaluations.get(id).map(|e| e.reason.clone()).unwrap_or_else(|| "evaluated to true".to_string()),
            evaluated_kind: state.invariant_evaluations.get(id).map(|e| e.evaluated_kind.clone()).unwrap_or_else(|| "bool".to_string()),
            actual: state.invariant_evaluations.get(id).map(|e| e.actual.clone()).unwrap_or_else(|| "true".to_string()),
            expected: state.invariant_evaluations.get(id).map(|e| e.expected.clone()).unwrap_or_else(|| "true".to_string()),
          });
        }
        _ => {
          println!("{:<width$}   ✗", name, width=width);
          failed_cases.push(violations.remove(id).unwrap_or(CaseDetail {
            name: name.clone(),
            expression: state.invariant_expressions.get(id).cloned().unwrap_or_default(),
            reason: "Invariant evaluated to false or non-bool value".to_string(),
            evaluated_kind: "bool".to_string(),
            actual: "?".to_string(),
            expected: "?".to_string()
          }));
        }
      }
    }

    let passed = passed_cases.len();
    let failed = failed_cases.len();
    let total = passed + failed;
    if failed == 0 {
      println!("\n{} SUCCESS: {} total | {} passed | {} failed\n", "[Test]".truecolor(153, 221, 85), total, passed, failed);
      if verbose {
        println!("passed:\n");
        for p in &passed_cases {
          println!("  {}: {}", p.name, p.expression);
          println!("    reason = {}", p.reason);
          println!("    evaluated_kind = {}", p.evaluated_kind);
          println!("    actual = {}", p.actual);
          println!("    expected = {}", p.expected);
        }
        println!();
      }
    } else {
      println!("\n{} FAILURE: {} total | {} passed | {} failed\n", "[Test]".truecolor(153, 221, 85), total, passed, failed);
      println!("failures:\n");
      for f in &failed_cases {
        println!("  {}: {}", f.name, f.expression);
        println!("    reason = {}", f.reason);
        println!("    evaluated_kind = {}", f.evaluated_kind);
        println!("    actual = {}", f.actual);
        println!("    expected = {}", f.expected);
      }
      if verbose {
        println!("\npassed:\n");
        for p in &passed_cases {
          println!("  {}: {}", p.name, p.expression);
          println!("    reason = {}", p.reason);
          println!("    evaluated_kind = {}", p.evaluated_kind);
          println!("    actual = {}", p.actual);
          println!("    expected = {}", p.expected);
        }
        println!();
      }
    }
    file_reports.push(FileReport { path: path.clone(), result: FileResult { total, passed, failed }, failed: failed_cases, passed: passed_cases, run_error: None });
  }

  let files_passed = file_reports.iter().filter(|f| !f.failed_file()).count();
  let files_failed = file_reports.len().saturating_sub(files_passed);
  let tests_total = file_reports.iter().map(|f| f.result.total).sum();
  let tests_passed = file_reports.iter().map(|f| f.result.passed).sum();
  let tests_failed = file_reports.iter().map(|f| f.result.failed).sum();
  let report = TestReport {
    result: SummaryResult { files_total: file_reports.len(), files_passed, files_failed, tests_total, tests_passed, tests_failed },
    files: file_reports,
  };

  if expanded_paths.len() > 1 {
    let summary_status = report.status_label();
    println!(
      "\n{} {}: files {} total | {} passed | {} failed || tests {} total | {} passed | {} failed",
      "[Test]".truecolor(153, 221, 85),
      summary_status,
      report.result.files_total,
      report.result.files_passed,
      report.result.files_failed,
      report.result.tests_total,
      report.result.tests_passed,
      report.result.tests_failed
    );

    let failing_files = report
      .files
      .iter()
      .filter(|f| f.failed_file())
      .collect::<Vec<_>>();

    if !failing_files.is_empty() {
      println!("\n  failing-files:");
      for file in failing_files {
        println!("    - {}", file.path);
        if let Some(run_error) = &file.run_error {
          println!("      reason: {}", run_error);
        } else {
          for failed_case in &file.failed {
            println!("      {}: {}", failed_case.name, failed_case.reason);
          }
        }
      }
    }

    if verbose {
      let passing_files = report
        .files
        .iter()
        .filter(|f| !f.failed_file())
        .map(|f| f.path.clone())
        .collect::<Vec<_>>();
      if !passing_files.is_empty() {
        println!("\n  passing-files:");
        for path in passing_files {
          println!("    - {}", path);
        }
      }
    }
    println!();

  }

  if let Some(output_path) = output_path {
    let path = PathBuf::from(&output_path);
    let extension = path.extension().and_then(OsStr::to_str).unwrap_or("");
    match extension {
      "json" => save_to_file(path, &report_to_json(&report, verbose)?)?,
      "mec" => save_to_file(path, &report_to_mech(&report, verbose))?,
      _ => { eprintln!("{} Unsupported --out extension `.{}`. Use .json or .mec.", "[Error]".truecolor(246,98,78), extension); return Ok(1); }
    }
  }

  Ok(report.exit_code())
}

#[cfg(test)]
mod tests {
  use super::*;

  fn temp_test_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
      "mech-test-{label}-{}",
      std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
  }

  #[test]
  fn test_out_writes_json_for_single_file() {
    let root = std::env::temp_dir().join(format!("mech-test-out-json-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("main.mec");
    let output = root.join("report.json");
    std::fs::write(&source, "x := 1\n").unwrap();

    let exit_code = run_mech_tests(vec![source.display().to_string()], false, false, false, false, Some(output.display().to_string()), false).unwrap();

    assert_eq!(exit_code, 0);
    assert!(output.metadata().unwrap().len() > 0);
    assert!(std::fs::read_to_string(&output).unwrap().contains("files-total"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn test_out_writes_mec_for_single_file() {
    let root = std::env::temp_dir().join(format!("mech-test-out-mec-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("main.mec");
    let output = root.join("report.mec");
    std::fs::write(&source, "x := 1\n").unwrap();

    let exit_code = run_mech_tests(vec![source.display().to_string()], false, false, false, false, Some(output.display().to_string()), false).unwrap();

    assert_eq!(exit_code, 0);
    assert!(output.metadata().unwrap().len() > 0);
    assert!(std::fs::read_to_string(&output).unwrap().contains("files-total"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn test_report_status_fails_when_file_has_run_error_without_failed_tests() {
    let report = TestReport {
      result: SummaryResult { files_total: 1, files_passed: 0, files_failed: 1, tests_total: 0, tests_passed: 0, tests_failed: 0 },
      files: vec![FileReport { path: "broken.mec".to_string(), result: FileResult { total: 0, passed: 0, failed: 0 }, failed: vec![], passed: vec![], run_error: Some("boom".to_string()) }],
    };

    assert_eq!(report.status_label(), "FAILED");
    assert_eq!(report.exit_code(), 1);
  }

  #[test]
  #[cfg(unix)]
  fn test_directory_discovery_skips_broken_symlink() {
    use std::os::unix::fs::symlink;

    let root = temp_test_root("broken-symlink");
    let source = root.join("main.mec");
    std::fs::write(&source, "x := 1\n").unwrap();
    symlink(root.join("missing.mec"), root.join("broken.mec")).unwrap();

    let targets = collect_test_targets(&root).unwrap();

    assert_eq!(targets, vec![source]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  #[cfg(unix)]
  fn test_explicit_broken_symlink_errors() {
    use std::os::unix::fs::symlink;

    let root = temp_test_root("explicit-broken-symlink");
    let broken = root.join("broken.mec");
    symlink(root.join("missing.mec"), &broken).unwrap();

    let result = collect_test_targets(&broken);

    assert!(result.is_err());
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn bytecode_test_path_detection_is_case_insensitive() {
    assert!(is_bytecode_test_path(Path::new("compiled.mecb")));
    assert!(is_bytecode_test_path(Path::new("compiled.MECB")));
    assert!(!is_bytecode_test_path(Path::new("source.mec")));
  }

  #[test]
  fn bytecode_test_error_mentions_invariant_metadata() {
    let message = bytecode_test_unsupported_error("compiled.mecb").display_message();
    assert!(message.contains("Bytecode test input"));
    assert!(message.contains("invariant metadata"));
  }

  #[test]
  fn mech_test_rejects_explicit_mecb_input() {
    let root = std::env::temp_dir().join(format!("mech-test-bytecode-explicit-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&root).unwrap();
    let bytecode = root.join("compiled.mecb");
    std::fs::write(&bytecode, b"not valid bytecode").unwrap();

    let exit_code = run_mech_tests(
      vec![bytecode.display().to_string()],
      false,
      false,
      false,
      false,
      None,
      false,
    ).unwrap();

    assert_eq!(exit_code, 1);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn test_directory_discovery_skips_mecb_artifacts() {
    let root = std::env::temp_dir().join(format!("mech-test-bytecode-skip-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("main.mec");
    let bytecode = root.join("output.mecb");
    std::fs::write(&source, "x := 1").unwrap();
    std::fs::write(&bytecode, b"not valid bytecode").unwrap();

    let targets = collect_test_targets(&root).unwrap();

    assert_eq!(targets, vec![source]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn test_explicit_mecb_input_is_still_collected_for_rejection() {
    let root = std::env::temp_dir().join(format!("mech-test-bytecode-explicit-collect-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&root).unwrap();
    let bytecode = root.join("compiled.mecb");
    std::fs::write(&bytecode, b"not valid bytecode").unwrap();

    let targets = collect_test_targets(&bytecode).unwrap();

    assert_eq!(targets, vec![bytecode]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn test_directory_with_source_and_output_mecb_passes_collection() {
    let root = std::env::temp_dir().join(format!("mech-test-bytecode-source-plus-output-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("main.mec");
    std::fs::write(&source, "# ok := true
").unwrap();
    std::fs::write(root.join("output.mecb"), b"not valid bytecode").unwrap();

    let targets = collect_test_targets(&root).unwrap();

    assert_eq!(targets, vec![source]);
    std::fs::remove_dir_all(root).unwrap();
  }
}
