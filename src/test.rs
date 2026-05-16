use crate::*;
use mech_interpreter::interpreter::*;
#[cfg(feature = "invariant_define")]
use mech_interpreter::InvariantViolationError;
use serde::Serialize;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};

fn collect_test_targets(path: &Path) -> io::Result<Vec<PathBuf>> {
  if !path.is_dir() {
    return Ok(vec![path.to_path_buf()]);
  }

  let mut files = Vec::new();
  for entry in std::fs::read_dir(path)? {
    let entry = entry?;
    let entry_path = entry.path();
    if entry_path.is_dir() {
      files.extend(collect_test_targets(&entry_path)?);
    } else if matches!(
      entry_path.extension().and_then(OsStr::to_str),
      Some("mec" | "🤖" | "mecb")
    ) {
      files.push(entry_path);
    }
  }
  files.sort();
  Ok(files)
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
    let targets = collect_test_targets(input_path).map_err(|e| {
      MechError::new(
        GenericError {
          msg: format!("Unable to collect test targets for `{}`: {}", input, e),
        },
        None,
      )
      .with_compiler_loc()
    })?;
    for target in targets {
      expanded_paths.push(target.display().to_string());
    }
  }

  let mut any_failed = false;
  let mut run_errors = false;
  let mut file_reports = Vec::new();
  println!("{} Running tests...\n", "[Test]".truecolor(153, 221, 85));
  for path in &expanded_paths {
    let uuid = generate_uuid();
    let mut intrp = Interpreter::new(uuid);
    let mut mechfs = MechFileSystem::new();
    if let Err(err) = mechfs.watch_source(path) {
      eprintln!("{} {}", "[Error]".truecolor(246,98,78), err.display_message());
      run_errors = true;
      any_failed = true;
      file_reports.push(FileReport { path: path.clone(), result: FileResult{total:0,passed:0,failed:0}, failed: vec![], passed: vec![], run_error: Some(err.display_message()) });
      continue;
    }
    if let Err(err) = run_mech_code(&mut intrp, &mechfs, tree_flag, debug_flag, time_flag, trace_flag) {
      eprintln!("{} {}", "[Error]".truecolor(246,98,78), err.display_message());
      run_errors = true;
      any_failed = true;
      file_reports.push(FileReport { path: path.clone(), result: FileResult{total:0,passed:0,failed:0}, failed: vec![], passed: vec![], run_error: Some(err.display_message()) });
      continue;
    }

    let state = intrp.state.borrow();
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
      any_failed = true;
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

  let files_passed = file_reports.iter().filter(|f| f.run_error.is_none() && f.result.failed == 0).count();
  let files_failed = file_reports.len().saturating_sub(files_passed);
  let tests_total = file_reports.iter().map(|f| f.result.total).sum();
  let tests_passed = file_reports.iter().map(|f| f.result.passed).sum();
  let tests_failed = file_reports.iter().map(|f| f.result.failed).sum();
  let report = TestReport {
    result: SummaryResult { files_total: file_reports.len(), files_passed, files_failed, tests_total, tests_passed, tests_failed },
    files: file_reports,
  };

  if let Some(output_path) = output_path {
    let path = PathBuf::from(&output_path);
    let extension = path.extension().and_then(OsStr::to_str).unwrap_or("");
    match extension {
      "json" => save_to_file(path, &report_to_json(&report, verbose)?)?,
      "mec" => save_to_file(path, &report_to_mech(&report, verbose))?,
      _ => { eprintln!("{} Unsupported --out extension `.{}`. Use .json or .mec.", "[Error]".truecolor(246,98,78), extension); return Ok(1); }
    }
  }
  if run_errors {
    println!("{} One or more files failed to load/execute, but all requested files were attempted.", "[Warn]".truecolor(255,210,77));
  }
  Ok(if any_failed { 1 } else { 0 })
}
