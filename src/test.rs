use crate::*;
use mech_interpreter::interpreter::*;
#[cfg(feature = "invariant_define")]
use mech_interpreter::InvariantViolationError;
use serde::Serialize;
use std::ffi::OsStr;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
struct TestCaseResult {
  name: String,
  passed: bool,
  reason: Option<String>,
  evaluated_kind: Option<String>,
  actual: Option<String>,
  expected: Option<String>,
}

#[derive(Debug, Serialize)]
struct TestFileResult {
  path: String,
  total: usize,
  passed: usize,
  failed: usize,
  cases: Vec<TestCaseResult>,
  run_error: Option<String>,
}

#[derive(Debug, Serialize)]
struct TestReport {
  total_files: usize,
  files_passed: usize,
  files_failed: usize,
  total_tests: usize,
  passed_tests: usize,
  failed_tests: usize,
  files: Vec<TestFileResult>,
}

fn mech_bool(v: bool) -> &'static str {
  if v { "✓" } else { "✗" }
}

fn mech_str(v: &str) -> String {
  format!("{:?}", v)
}

fn mech_opt_str(name: &str, v: &Option<String>) -> String {
  match v {
    Some(s) => format!("{name}: {}", mech_str(s)),
    None => format!("{name}: empty"),
  }
}

fn test_case_to_mech(test_case: &TestCaseResult) -> String {
  format!(
    "{{ name: {}, passed: {}, {}, {}, {}, {} }}",
    mech_str(&test_case.name),
    mech_bool(test_case.passed),
    mech_opt_str("reason", &test_case.reason),
    mech_opt_str("evaluated-kind", &test_case.evaluated_kind),
    mech_opt_str("actual", &test_case.actual),
    mech_opt_str("expected", &test_case.expected),
  )
}

fn test_file_to_mech(file: &TestFileResult) -> String {
  let cases = file.cases.iter().map(test_case_to_mech).collect::<Vec<_>>().join(" ");
  format!(
    "{{ path: {}, total: {}, passed: {}, failed: {}, cases: [{}], {} }}",
    mech_str(&file.path),
    file.total,
    file.passed,
    file.failed,
    cases,
    mech_opt_str("run-error", &file.run_error),
  )
}

fn report_to_mech(report: &TestReport) -> String {
  let files = report.files.iter().map(test_file_to_mech).collect::<Vec<_>>().join(" ");
  format!(
    "test-output := {{ total-files: {}, files-passed: {}, files-failed: {}, total-tests: {}, passed-tests: {}, failed-tests: {}, files: [{}] }}",
    report.total_files,
    report.files_passed,
    report.files_failed,
    report.total_tests,
    report.passed_tests,
    report.failed_tests,
    files
  )
}

pub fn run_mech_tests(
  mech_paths: Vec<String>,
  tree_flag: bool,
  debug_flag: bool,
  time_flag: bool,
  trace_flag: bool,
  output_path: Option<String>,
) -> Result<i32, MechError> {
  let mut any_failed = false;
  let mut run_errors = false;
  let mut file_results = Vec::new();
  println!("{} Running tests...\n", "[Test]".truecolor(153, 221, 85));
  for path in &mech_paths {
    let uuid = generate_uuid();
    let mut intrp = Interpreter::new(uuid);
    let mut mechfs = MechFileSystem::new();
    if let Err(err) = mechfs.watch_source(path) {
      eprintln!("{} {}", "[Error]".truecolor(246,98,78), err.display_message());
      run_errors = true;
      any_failed = true;
      file_results.push(TestFileResult {
        path: path.clone(),
        total: 0,
        passed: 0,
        failed: 0,
        cases: vec![],
        run_error: Some(err.display_message()),
      });
      continue;
    }
    let result = run_mech_code(&mut intrp, &mechfs, tree_flag, debug_flag, time_flag, trace_flag);
    if let Err(err) = result {
      eprintln!("{} {}", "[Error]".truecolor(246,98,78), err.display_message());
      run_errors = true;
      any_failed = true;
      file_results.push(TestFileResult {
        path: path.clone(),
        total: 0,
        passed: 0,
        failed: 0,
        cases: vec![],
        run_error: Some(err.display_message()),
      });
      continue;
    }
    let mut passed = 0usize;
    let mut failed = 0usize;
    let state_brrw = intrp.state.borrow();
    let test_name_width = state_brrw.invariants.values().map(|(n, _)| n.len()).max().unwrap_or(0);
    println!("{} {}\n", "[Test]".truecolor(153, 221, 85), path);
    let mut cases = Vec::new();
    for (_id, (name, value)) in state_brrw.invariants.iter() {
      match &*value.borrow() {
        Value::Bool(b) if *b.borrow() => {
          println!("{:<width$}   ✓", name, width=test_name_width);
          passed += 1;
          cases.push(TestCaseResult { name: name.clone(), passed: true, reason: None, evaluated_kind: None, actual: None, expected: None });
        },
        _ => {
          println!("{:<width$}   ✗", name, width=test_name_width);
          failed += 1;
          cases.push(TestCaseResult {
            name: name.clone(),
            passed: false,
            reason: Some("Invariant evaluated to false or non-bool value".to_string()),
            evaluated_kind: None,
            actual: None,
            expected: None,
          });
        },
      }
    }
    let total = passed + failed;
    if failed == 0 {
      println!("\n{} SUCCESS: {} total | {} passed | {} failed\n", "[Test]".truecolor(153, 221, 85), total, passed, failed);
    } else {
      any_failed = true;
      println!("\n{} FAILURE: {} total | {} passed | {} failed", "[Test]".truecolor(153, 221, 85), total, passed, failed);
      if !state_brrw.invariant_violations.is_empty() {
        println!("\nfailures:\n");
        for violation in &state_brrw.invariant_violations {
          let name = state_brrw.invariants.get(&violation.id).map(|(n, _)| n.clone()).unwrap_or_else(|| format!("#{}", violation.id));
          if let Some(inv_err) = violation.error.kind_as::<InvariantViolationError>() {
            let lhs = inv_err.lhs_value.clone().unwrap_or_else(|| "?".to_string());
            let rhs = inv_err.rhs_value.clone().unwrap_or_else(|| "?".to_string());
            println!("  {}: {}", name, inv_err.expression);
            println!("    reason = {}", inv_err.reason);
            println!("    evaluated_kind = {}", inv_err.evaluated_kind);
            println!("    actual = {}", lhs);
            println!("    expected = {}", rhs);
          } else {
            println!("  {}: {}", name, violation.error.display_message());
          }
        }
      }
      println!();
    }
    file_results.push(TestFileResult { path: path.clone(), total, passed, failed, cases, run_error: None });
  }

  if let Some(output_path) = output_path {
    let files_passed = file_results.iter().filter(|f| f.run_error.is_none() && f.failed == 0).count();
    let files_failed = file_results.len().saturating_sub(files_passed);
    let total_tests = file_results.iter().map(|f| f.total).sum();
    let passed_tests = file_results.iter().map(|f| f.passed).sum();
    let failed_tests = file_results.iter().map(|f| f.failed).sum();
    let report = TestReport { total_files: file_results.len(), files_passed, files_failed, total_tests, passed_tests, failed_tests, files: file_results };
    let path = PathBuf::from(&output_path);
    let extension = path.extension().and_then(OsStr::to_str).unwrap_or("");
    match extension {
      "json" => {
        let content = serde_json::to_string_pretty(&report).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        save_to_file(path, &content)?;
      }
      "mec" => {
        let content = report_to_mech(&report);
        save_to_file(path, &content)?;
      }
      _ => {
        eprintln!("{} Unsupported --out extension `.{}`. Use .json or .mec.", "[Error]".truecolor(246,98,78), extension);
        return Ok(1);
      }
    }
  }
  if run_errors {
    println!("{} One or more files failed to load/execute, but all requested files were attempted.", "[Warn]".truecolor(255,210,77));
  }
  Ok(if any_failed { 1 } else { 0 })
}
