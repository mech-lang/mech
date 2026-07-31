use crate::*;
use serde::Serialize;
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};

use crate::cli::module_execution::{
  execute_source_module_roots_with_report,
  module_runtime_config,
};
use crate::fs_paths::{extension_allowed, unsupported_source_path_error};
use crate::source_discovery::{
  collect_sources_with_events,
  DedupePolicy,
  DiscoveryOptions,
  MissingPathPolicy,
};
use mech_program::{
  IntegrityConstraintEvaluation, IntegrityConstraintFailureReason,
  IntegrityConstraintViolation, IntegrityConstraintViolationSet,
};

const TEST_EXPLICIT_EXTENSIONS: &[&str] = &["mec", "🤖", "mecb"];
const TEST_RECURSIVE_EXTENSIONS: &[&str] = &["mec", "🤖"];
const TEST_SKIP_DIRS: &[&str] = &["target", ".git", "dist", "out"];

/// A discovered test source keeps its filesystem identity separate from the
/// human-readable label used in terminal and serialized reports. On Unix, a
/// valid source filename need not be UTF-8, so the label must never be used to
/// reconstruct an execution path.
#[derive(Clone, Debug)]
struct TestSourceTarget {
  path: PathBuf,
  display: String,
}

impl TestSourceTarget {
  fn from_path(path: PathBuf) -> Self {
    Self {
      display: path.display().to_string(),
      path,
    }
  }
}

fn collect_test_targets(path: &Path) -> MResult<Vec<PathBuf>> {
  if let Ok(metadata) = std::fs::symlink_metadata(path) {
    if metadata.file_type().is_symlink() {
      let canonical = path.canonicalize()?;
      if canonical.is_file() {
        if extension_allowed(path, TEST_EXPLICIT_EXTENSIONS) {
          return Ok(vec![path.to_path_buf()]);
        }
        return Err(unsupported_source_path_error(path, TEST_EXPLICIT_EXTENSIONS));
      }
      if canonical.is_dir() {
        return Err(MechError::new(
          GenericError {
            msg: format!(
              "Explicit symlinked test directory `{}` is not followed for test discovery.",
              path.display()
            ),
          },
          None,
        )
        .with_compiler_loc());
      }
      return Err(MechError::new(
        GenericError {
          msg: format!(
            "Explicit symlinked test input `{}` does not resolve to a file or directory.",
            path.display()
          ),
        },
        None,
      )
      .with_compiler_loc());
    }
  }

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

fn bytecode_test_unsupported_error(path: &Path) -> MechError {
  MechError::new(
    GenericError {
      msg: format!(
        "Bytecode test input `{}` is not supported because compiled bytecode does not currently include invariant metadata. Run tests from source files instead.",
        path.display()
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

fn integrity_reason(
  reason: Option<&IntegrityConstraintFailureReason>,
) -> String {
  match reason {
    None => "evaluated to true".to_string(),
    Some(IntegrityConstraintFailureReason::EvaluatedFalse) => {
      "evaluated to false".to_string()
    }
    Some(IntegrityConstraintFailureReason::ExpectedBool) => {
      "expected a scalar bool".to_string()
    }
    Some(IntegrityConstraintFailureReason::BorrowConflict) => {
      "could not read the settled constraint result".to_string()
    }
  }
}

fn integrity_evaluation_case(
  evaluation: &IntegrityConstraintEvaluation,
) -> CaseDetail {
  CaseDetail {
    name: evaluation.name.clone(),
    expression: evaluation.expression.clone(),
    reason: integrity_reason(evaluation.reason.as_ref()),
    evaluated_kind: evaluation
      .evaluated_kind
      .as_ref()
      .map(ToString::to_string)
      .unwrap_or_else(|| "unknown".to_string()),
    actual: evaluation.actual.clone().unwrap_or_default(),
    expected: evaluation.expected.clone().unwrap_or_default(),
  }
}

fn integrity_violation_case(
  violation: &IntegrityConstraintViolation,
) -> CaseDetail {
  CaseDetail {
    name: violation.name.clone(),
    expression: violation.expression.clone(),
    reason: integrity_reason(Some(&violation.reason)),
    evaluated_kind: violation
      .evaluated_kind
      .as_ref()
      .map(ToString::to_string)
      .unwrap_or_else(|| "unknown".to_string()),
    actual: violation.actual.clone().unwrap_or_default(),
    expected: violation.expected.clone().unwrap_or_default(),
  }
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
  _tree_flag: bool,
  debug_flag: bool,
  time_flag: bool,
  trace_flag: bool,
  output_path: Option<String>,
  verbose: bool,
) -> Result<i32, MechError> {
  run_mech_tests_without_tree(
    mech_paths,
    debug_flag,
    time_flag,
    trace_flag,
    output_path,
    verbose,
  )
}

pub(crate) fn run_mech_tests_without_tree(
  mech_paths: Vec<String>,
  debug_flag: bool,
  time_flag: bool,
  trace_flag: bool,
  output_path: Option<String>,
  verbose: bool,
) -> Result<i32, MechError> {
  let mut expanded_targets = Vec::new();
  for input in mech_paths {
    let input_path = Path::new(&input);
    let targets = collect_test_targets(input_path)?;
    for target in targets {
      expanded_targets.push(TestSourceTarget::from_path(target));
    }
  }

  if expanded_targets.is_empty() {
    return Err(MechError::new(
      GenericError {
        msg: "No test targets were found.".to_string(),
      },
      None,
    )
    .with_compiler_loc());
  }

  let mut file_reports = Vec::new();
  println!("{} Running tests...\n", "[Test]".truecolor(153, 221, 85));
  for target in &expanded_targets {
    if is_bytecode_test_path(&target.path) {
      let err = bytecode_test_unsupported_error(&target.path);
      eprintln!("{} {}", "[Error]".truecolor(246,98,78), err.display_message());
      file_reports.push(FileReport { path: target.display.clone(), result: FileResult{total:0,passed:0,failed:0}, failed: vec![], passed: vec![], run_error: Some(err.display_message()) });
      continue;
    }
    let config = module_runtime_config(
      format!("test-{}", generate_uuid()),
      debug_flag,
      trace_flag,
      time_flag,
      10_000,
    )?;
    let execution = match execute_source_module_roots_with_report(
      config,
      &[target.path.clone()],
    ) {
      Ok(execution) => execution,
      Err(err) => {
        if let Some(failures) =
          err.kind_as::<IntegrityConstraintViolationSet>()
        {
          let mut passed_cases = Vec::new();
          let mut failed_cases = Vec::new();
          if failures.evaluations.is_empty() {
            failed_cases.extend(
              failures
                .violations
                .iter()
                .map(integrity_violation_case),
            );
          } else {
            for evaluation in &failures.evaluations {
              let detail = integrity_evaluation_case(evaluation);
              if evaluation.passed {
                passed_cases.push(detail);
              } else {
                failed_cases.push(detail);
              }
            }
          }
          let passed = passed_cases.len();
          let failed = failed_cases.len();
          let total = passed + failed;
          println!("{} {}\n", "[Test]".truecolor(153, 221, 85), target.display);
          for detail in &passed_cases {
            println!("{}   ✓", detail.name);
          }
          for detail in &failed_cases {
            println!("{}   ✗", detail.name);
          }
          file_reports.push(FileReport {
            path: target.display.clone(),
            result: FileResult {
              total,
              passed,
              failed,
            },
            failed: failed_cases,
            passed: passed_cases,
            run_error: None,
          });
          continue;
        }
        eprintln!("{} {}", "[Error]".truecolor(246,98,78), err.display_message());
        file_reports.push(FileReport { path: target.display.clone(), result: FileResult{total:0,passed:0,failed:0}, failed: vec![], passed: vec![], run_error: Some(err.display_message()) });
        continue;
      }
    };
    let report = execution.integrity;
    let _runtime = execution.runtime;
    println!("{} {}\n", "[Test]".truecolor(153, 221, 85), target.display);

    let mut passed_cases = Vec::new();
    let mut failed_cases = Vec::new();
    let width = report
      .evaluations
      .iter()
      .map(|case| case.name.len())
      .max()
      .unwrap_or(0);
    for evaluation in report.evaluations {
      println!(
        "{:<width$}   {}",
        evaluation.name,
        if evaluation.passed { "✓" } else { "✗" },
        width=width,
      );
      let detail = integrity_evaluation_case(&evaluation);
      if evaluation.passed {
        passed_cases.push(detail);
      } else {
        failed_cases.push(detail);
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
    file_reports.push(FileReport { path: target.display.clone(), result: FileResult { total, passed, failed }, failed: failed_cases, passed: passed_cases, run_error: None });
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

  if expanded_targets.len() > 1 {
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

  fn imported_invariant_fixture(label: &str) -> (PathBuf, PathBuf) {
    let root = temp_test_root(label);
    let main = root.join("main.mec");
    std::fs::write(
      &main,
      "+> ./dep.mec\nanswer := dep/value + 1\nanswer! := answer == 42\n",
    )
    .unwrap();
    std::fs::write(root.join("dep.mec"), "value := 41\n<+ value\n").unwrap();
    (root, main)
  }

  fn contains_address_like_diagnostic(text: &str) -> bool {
    text.split("0x").skip(1).any(|suffix| {
      suffix
        .chars()
        .take_while(char::is_ascii_hexdigit)
        .count()
        >= 6
    })
  }

  #[test]
  fn mech_tests_resolve_imported_dependencies_with_passing_invariants() {
    let (root, main) = imported_invariant_fixture("imported-pass");
    let output = root.join("report.json");

    let exit_code = run_mech_tests(
      vec![main.display().to_string()],
      false,
      false,
      false,
      false,
      Some(output.display().to_string()),
      false,
    )
    .unwrap();

    let report = std::fs::read_to_string(&output).unwrap();
    assert_eq!(exit_code, 0);
    assert!(report.contains("\"files-passed\": 1"));
    assert!(report.contains("\"run-error\": null"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn mech_tests_classify_each_integrity_violation_as_a_failed_case() {
    let root = temp_test_root("integrity-aggregate");
    let main = root.join("main.mec");
    let output = root.join("report.json");
    std::fs::write(
      &main,
      "first! := false\nsecond! := 42.0\nthird! := 2.0 < 1.0\n",
    )
    .unwrap();

    let exit_code = run_mech_tests(
      vec![main.display().to_string()],
      false,
      false,
      false,
      false,
      Some(output.display().to_string()),
      true,
    )
    .unwrap();

    let report = std::fs::read_to_string(&output).unwrap();
    assert_eq!(exit_code, 1);
    assert!(report.contains("\"tests-total\": 3"));
    assert!(report.contains("\"tests-failed\": 3"));
    assert!(report.contains("\"run-error\": null"));
    assert!(report.contains("\"first!\""));
    assert!(report.contains("\"second!\""));
    assert!(report.contains("\"third!\""));
    assert!(!report.contains("@0x"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[cfg(feature = "linked_stdlib")]
  #[test]
  fn mech_tests_execute_linked_module_imports() {
    let root = temp_test_root("linked-module-import");
    let main = root.join("main.mec");
    let output = root.join("report.json");
    std::fs::write(
      &main,
      "+> math\nresult := math/sin(0)\nresult! := result == 0\n",
    )
    .unwrap();

    let exit_code = run_mech_tests(
      vec![main.display().to_string()],
      false,
      false,
      false,
      false,
      Some(output.display().to_string()),
      false,
    )
    .unwrap();

    let report = std::fs::read_to_string(&output).unwrap();
    assert_eq!(exit_code, 0);
    assert!(report.contains("\"files-passed\": 1"));
    assert!(report.contains("\"run-error\": null"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn mech_tests_resolve_nested_dependencies_with_passing_invariants() {
    let root = temp_test_root("nested-pass");
    let main = root.join("main.mec");
    let lib = root.join("lib");
    let output = root.join("report.json");
    std::fs::create_dir_all(&lib).unwrap();
    std::fs::write(
      &main,
      "+> ./lib/first.mec\nanswer := first/value + 1\nanswer! := answer == 42\n",
    )
    .unwrap();
    std::fs::write(
      lib.join("first.mec"),
      "+> ./second.mec\nvalue := second/value + 1\n<+ value\n",
    )
    .unwrap();
    std::fs::write(lib.join("second.mec"), "value := 40\n<+ value\n").unwrap();

    let exit_code = run_mech_tests(
      vec![main.display().to_string()],
      false,
      false,
      false,
      false,
      Some(output.display().to_string()),
      false,
    )
    .unwrap();

    assert_eq!(exit_code, 0);
    assert!(std::fs::read_to_string(&output).unwrap().contains("\"tests-passed\": 1"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn mech_tests_report_missing_dependencies_as_run_errors() {
    let root = temp_test_root("missing-dependency");
    let main = root.join("main.mec");
    let output = root.join("report.json");
    std::fs::write(&main, "+> ./missing.mec\nanswer := 1\n").unwrap();

    let exit_code = run_mech_tests(
      vec![main.display().to_string()],
      false,
      false,
      false,
      false,
      Some(output.display().to_string()),
      false,
    )
    .unwrap();

    let report = std::fs::read_to_string(&output).unwrap();
    assert_eq!(exit_code, 1);
    assert!(report.contains("\"run-error\": \""));
    assert!(report.contains("missing.mec"));
    assert!(report.contains("main.mec"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn mech_tests_isolate_each_root_runtime() {
    let root = temp_test_root("root-isolation");
    let first = root.join("first.mec");
    let second = root.join("second.mec");
    let output = root.join("report.json");
    std::fs::write(&first, "value := 1\nown! := value == 1\n").unwrap();
    std::fs::write(&second, "value := 2\ntwo! := value == 2\n").unwrap();

    let exit_code = run_mech_tests(
      vec![first.display().to_string(), second.display().to_string()],
      false,
      false,
      false,
      false,
      Some(output.display().to_string()),
      false,
    )
    .unwrap();

    let report = std::fs::read_to_string(&output).unwrap();
    assert_eq!(exit_code, 0);
    assert!(report.contains("\"files-passed\": 2"));
    assert!(report.contains("\"tests-passed\": 2"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn mech_tests_include_passing_dependency_constraint_without_root_proxy() {
    let root = temp_test_root("dependency-pass-no-proxy");
    let main = root.join("main.mec");
    let output = root.join("report.json");
    std::fs::write(
      &main,
      "+> ./dep.mec\n\nanswer := dep/value + 1\n",
    )
    .unwrap();
    std::fs::write(
      root.join("dep.mec"),
      "value := 41\n\ndependency-pass! := value == 41\n\n<+ value\n",
    )
    .unwrap();

    let exit_code = run_mech_tests(
      vec![main.display().to_string()],
      false,
      false,
      false,
      false,
      Some(output.display().to_string()),
      false,
    )
    .unwrap();

    let report = std::fs::read_to_string(&output).unwrap();
    assert_eq!(exit_code, 0);
    assert!(report.contains("\"tests-total\": 1"));
    assert!(report.contains("\"tests-passed\": 1"));
    assert!(report.contains("\"tests-failed\": 0"));
    assert!(report.contains("dependency-pass!"));
    assert!(report.contains("\"run-error\": null"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn mech_tests_aggregate_passing_and_failing_dependency_constraints() {
    let root = temp_test_root("dependency-pass-fail");
    let main = root.join("main.mec");
    let output = root.join("report.json");
    std::fs::write(
      &main,
      "+> ./dep.mec\n\nanswer := dep/value + 1\n",
    )
    .unwrap();
    std::fs::write(
      root.join("dep.mec"),
      "value := 41\n\ndependency-pass! := value == 41\ndependency-fail! := value == 42\n\n<+ value\n",
    )
    .unwrap();

    let exit_code = run_mech_tests(
      vec![main.display().to_string()],
      false,
      false,
      false,
      false,
      Some(output.display().to_string()),
      true,
    )
    .unwrap();

    let report = std::fs::read_to_string(&output).unwrap();
    assert_eq!(exit_code, 1);
    assert!(report.contains("\"tests-total\": 2"));
    assert!(report.contains("\"tests-passed\": 1"));
    assert!(report.contains("\"tests-failed\": 1"));
    assert!(report.contains("dependency-pass!"));
    assert!(report.contains("dependency-fail!"));
    assert!(report.contains("\"run-error\": null"));
    assert!(!report.contains("@0x"));
    assert!(!contains_address_like_diagnostic(&report));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn mech_tests_include_nested_dependency_constraints_once() {
    let root = temp_test_root("nested-dependency-once");
    let main = root.join("main.mec");
    let output = root.join("report.json");
    std::fs::write(
      &main,
      "+> ./left.mec\n+> ./right.mec\n\nanswer := left/value + right/value\n",
    )
    .unwrap();
    std::fs::write(
      root.join("left.mec"),
      "+> ./shared.mec\n\nvalue := shared/value\n\n<+ value\n",
    )
    .unwrap();
    std::fs::write(
      root.join("right.mec"),
      "+> ./shared.mec\n\nvalue := shared/value\n\n<+ value\n",
    )
    .unwrap();
    std::fs::write(
      root.join("shared.mec"),
      "value := 21\n\nnested-once! := value == 21\n\n<+ value\n",
    )
    .unwrap();

    let exit_code = run_mech_tests(
      vec![main.display().to_string()],
      false,
      false,
      false,
      false,
      Some(output.display().to_string()),
      false,
    )
    .unwrap();

    let report = std::fs::read_to_string(&output).unwrap();
    assert_eq!(exit_code, 0);
    assert!(report.contains("\"tests-total\": 1"));
    assert!(report.contains("\"tests-passed\": 1"));
    assert_eq!(report.matches("nested-once!").count(), 1);
    assert!(report.contains("\"run-error\": null"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn mech_tests_preserve_report_formats_for_imported_modules() {
    let (root, main) = imported_invariant_fixture("report-formats");
    let json_output = root.join("report.json");
    let mech_output = root.join("report.mec");
    let input = vec![main.display().to_string()];

    assert_eq!(
      run_mech_tests(input.clone(), false, false, false, false, None, false).unwrap(),
      0
    );
    assert_eq!(
      run_mech_tests(
        input.clone(),
        false,
        false,
        false,
        false,
        Some(json_output.display().to_string()),
        false,
      )
      .unwrap(),
      0
    );
    assert_eq!(
      run_mech_tests(
        input,
        false,
        false,
        false,
        false,
        Some(mech_output.display().to_string()),
        false,
      )
      .unwrap(),
      0
    );
    assert!(std::fs::read_to_string(json_output).unwrap().contains("files-total"));
    assert!(std::fs::read_to_string(mech_output).unwrap().contains("files-total"));
    std::fs::remove_dir_all(root).unwrap();
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
  #[cfg(unix)]
  fn explicit_symlinked_test_file_is_collected() {
    use std::os::unix::fs::symlink;

    let root = temp_test_root("explicit-file-symlink");
    let source = root.join("main.mec");
    let link = root.join("linked.mec");
    std::fs::write(&source, "x := 1\n").unwrap();
    symlink(&source, &link).unwrap();

    let targets = collect_test_targets(&link).unwrap();

    assert_eq!(targets, vec![link]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  #[cfg(unix)]
  fn explicit_symlinked_mecb_input_is_collected_for_rejection() {
    use std::os::unix::fs::symlink;

    let root = temp_test_root("explicit-mecb-symlink");
    let bytecode = root.join("compiled.mecb");
    let link = root.join("linked.mecb");
    std::fs::write(&bytecode, b"not valid bytecode").unwrap();
    symlink(&bytecode, &link).unwrap();

    let targets = collect_test_targets(&link).unwrap();

    assert_eq!(targets, vec![link]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  #[cfg(unix)]
  fn directory_discovery_skips_file_symlink() {
    use std::os::unix::fs::symlink;

    let root = temp_test_root("directory-file-symlink");
    let outside = temp_test_root("directory-file-symlink-target");
    let source = root.join("main.mec");
    let linked_target = outside.join("linked-target.mec");
    let link = root.join("linked.mec");
    std::fs::write(&source, "x := 1\n").unwrap();
    std::fs::write(&linked_target, "y := 2\n").unwrap();
    symlink(&linked_target, &link).unwrap();

    let targets = collect_test_targets(&root).unwrap();

    assert_eq!(targets, vec![source]);
    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(outside).unwrap();
  }

  #[test]
  fn run_mech_tests_errors_when_no_targets_are_found() {
    let root = temp_test_root("empty-directory");

    let result = run_mech_tests(
      vec![root.display().to_string()],
      false,
      false,
      false,
      false,
      None,
      false,
    );

    let error = result.unwrap_err().display_message();
    assert!(error.contains("No test targets were found"));
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
    let message = bytecode_test_unsupported_error(Path::new("compiled.mecb")).display_message();
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

  #[cfg(target_os = "linux")]
  #[test]
  fn mech_tests_preserve_discovered_non_utf8_filename() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let root = temp_test_root("non-utf8-discovery");
    let source = root.join(OsString::from_vec(b"test-\xff.mec".to_vec()));
    let output = root.join("report.json");
    std::fs::write(&source, "answer := 41\nfilename-pass! := answer == 41\n").unwrap();

    let exit_code = run_mech_tests(
      vec![root.display().to_string()],
      false,
      false,
      false,
      false,
      Some(output.display().to_string()),
      false,
    )
    .unwrap();

    let report = std::fs::read_to_string(&output).unwrap();
    assert_eq!(exit_code, 0);
    assert!(report.contains("\"files-total\": 1"));
    assert!(report.contains("\"tests-passed\": 1"));
    assert!(report.contains("\"run-error\": null"));
    std::fs::remove_dir_all(root).unwrap();
  }
}
