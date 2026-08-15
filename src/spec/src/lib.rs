#![forbid(unsafe_code)]

mod case;
mod current;
mod model;
mod observer;
mod reference;

use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

pub use model::{CheckReport, ConstraintResult, CoverageReport, ExecutorObservations};
use reference::CONTRACT_PROFILE;

pub type Result<T> = std::result::Result<T, SpecError>;

#[derive(Clone, Debug)]
pub struct SpecError {
    message: String,
}

impl SpecError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for SpecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SpecError {}

#[derive(Debug)]
struct SpecificationDocument {
    display_path: String,
    source: String,
}

/// Runs the first executable-specification vertical slice against the v0.4
/// resident executor and its host transaction shell.
pub fn check(spec_root: impl AsRef<Path>) -> Result<CheckReport> {
    let spec_root = spec_root.as_ref();
    let case_path = spec_root.join("cases/transactions/commit-and-abort.mec");
    let case = case::ExecutorCase::load(&case_path)?;
    let observations = observer::observe(&case)?;
    let observation_source = observations.to_mech_source();
    let specifications = load_specifications(spec_root)?;
    let mut constraints = Vec::new();
    let mut all_evaluators_agree = true;

    for specification in &specifications {
        let reference = reference::evaluate(&observation_source, &specification.source)?;
        let current = current::evaluate(&observation_source, &reference)?;
        let document_agrees = current.len() == reference.len()
            && reference.iter().all(|evaluation| {
                current
                    .get(&evaluation.name)
                    .is_some_and(|mech| mech.passed == evaluation.passed)
            });
        all_evaluators_agree &= document_agrees;

        for evaluation in reference {
            let mech_passed = current.get(&evaluation.name).map(|item| item.passed);
            let evaluators_agree = mech_passed == Some(evaluation.passed);
            let passed = evaluation.passed && evaluators_agree;
            let detail = if evaluators_agree {
                evaluation.detail
            } else {
                format!(
                    "evaluator disagreement: reference={}, resident Mech={}",
                    evaluation.passed,
                    mech_passed
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "missing".to_string()),
                )
            };
            constraints.push(ConstraintResult {
                specification: specification.display_path.clone(),
                requirement: evaluation.requirement,
                name: evaluation.name,
                expression: evaluation.expression,
                reference_passed: evaluation.passed,
                mech_passed,
                evaluators_agree,
                passed,
                detail,
            });
        }
    }

    constraints.sort_by(|left, right| {
        (&left.requirement, &left.name).cmp(&(&right.requirement, &right.name))
    });
    let specified_requirements = constraints
        .iter()
        .map(|constraint| constraint.requirement.clone())
        .collect::<BTreeSet<_>>();
    let evidenced_requirements = case.requirements.iter().cloned().collect::<BTreeSet<_>>();
    let uncovered = specified_requirements
        .difference(&evidenced_requirements)
        .cloned()
        .collect::<Vec<_>>();
    let requirements_with_evidence = specified_requirements
        .intersection(&evidenced_requirements)
        .count();
    let coverage = CoverageReport {
        requirements: specified_requirements.len(),
        requirements_with_evidence,
        uncovered,
    };
    let passed = all_evaluators_agree
        && coverage.uncovered.is_empty()
        && constraints.iter().all(|constraint| constraint.passed);

    Ok(CheckReport {
        profile: CONTRACT_PROFILE.to_string(),
        observations,
        specifications: specifications
            .into_iter()
            .map(|document| document.display_path)
            .collect(),
        constraints,
        coverage,
        evaluators_agree: all_evaluators_agree,
        passed,
    })
}

pub fn write_json_report(report: &CheckReport, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            SpecError::new(format!(
                "could not create report directory {}: {error}",
                parent.display(),
            ))
        })?;
    }
    let json = serde_json::to_string_pretty(report)
        .map_err(|error| SpecError::new(format!("could not serialize report: {error}")))?;
    fs::write(path, format!("{json}\n")).map_err(|error| {
        SpecError::new(format!(
            "could not write report {}: {error}",
            path.display()
        ))
    })
}

fn load_specifications(root: &Path) -> Result<Vec<SpecificationDocument>> {
    let mut paths = Vec::new();
    collect_mec_files(root, &mut paths)?;
    paths.sort();
    let mut specifications = Vec::new();
    for path in paths {
        if path
            .components()
            .any(|component| component.as_os_str() == "cases")
        {
            continue;
        }
        let source = fs::read_to_string(&path).map_err(|error| {
            SpecError::new(format!(
                "could not read specification {}: {error}",
                path.display(),
            ))
        })?;
        if !source.contains("contract-profile :=") {
            continue;
        }
        let display_path = path
            .strip_prefix(root.parent().unwrap_or(root))
            .unwrap_or(&path)
            .display()
            .to_string();
        specifications.push(SpecificationDocument {
            display_path,
            source,
        });
    }
    if specifications.is_empty() {
        return Err(SpecError::new(format!(
            "no executable contract specifications found under {}",
            root.display(),
        )));
    }
    Ok(specifications)
}

fn collect_mec_files(directory: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(directory).map_err(|error| {
        SpecError::new(format!(
            "could not inspect specification directory {}: {error}",
            directory.display(),
        ))
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            SpecError::new(format!(
                "could not inspect specification directory {}: {error}",
                directory.display(),
            ))
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| {
            SpecError::new(format!("could not inspect {}: {error}", path.display()))
        })?;
        if file_type.is_dir() {
            collect_mec_files(&path, output)?;
        } else if file_type.is_file() && path.extension().is_some_and(|value| value == "mec") {
            output.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_vertical_slice_conforms_end_to_end() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec");
        let report = check(root).unwrap();
        assert!(report.passed, "{}", report.render_text());
        assert_eq!(report.observations.resident_route, "resident-pure");
        assert!(report.evaluators_agree);
        assert!(report.coverage.uncovered.is_empty());
        assert!(
            report
                .constraints
                .iter()
                .any(|constraint| constraint.requirement == "TURN-004")
        );
    }

    #[test]
    fn injected_abort_drift_fails_turn_004_in_both_evaluators() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec");
        let case = case::ExecutorCase::load(&root.join("cases/transactions/commit-and-abort.mec"))
            .unwrap();
        let mut observations = observer::observe(&case).unwrap();
        observations.abort_after_state = "deliberately-corrupted".to_string();
        let observation_source = observations.to_mech_source();
        let specification = load_specifications(&root).unwrap().remove(0);

        let reference = reference::evaluate(&observation_source, &specification.source).unwrap();
        let current = current::evaluate(&observation_source, &reference).unwrap();
        let rollback = reference
            .iter()
            .find(|evaluation| evaluation.requirement == "TURN-004")
            .unwrap();

        assert!(!rollback.passed);
        assert_eq!(
            current.get(&rollback.name).map(|item| item.passed),
            Some(false)
        );
    }
}
