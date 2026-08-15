#![forbid(unsafe_code)]

mod case;
mod current;
mod determinism;
mod evidence;
mod model;
mod observer;
mod reference;
mod registry;
mod source_profile;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

pub use determinism::{RecordedBundle, ReplayReport};
pub use model::{
    CheckReport, ConformanceStatus, ConstraintResult, CoverageReport, DemonstrationReport,
    DemonstrationScenario, EvidenceBatch, EvidenceGrade, EvidenceStatus, ExecutorObservations,
    ProfileResult, RequirementArchReport, SpecificationStatus,
};
pub use source_profile::{DocumentArtifact, MechArtifact, SourceProfile};

use model::ViolationWitness;
use reference::CONTRACT_PROFILE;
use registry::{EvidenceRule, RequirementRegistry};

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

#[derive(Clone, Debug)]
struct SpecificationDocument {
    display_path: String,
    source: String,
}

/// Runs the reflective specification vertical slice against the v0.4 resident
/// executor and policy-free observation providers.
pub fn check(specification_path: impl AsRef<Path>) -> Result<CheckReport> {
    let layout = SpecLayout::discover(specification_path.as_ref())?;
    let artifacts = compile_source_profiles(&layout)?;
    validate_document_links(&artifacts)?;
    let registry = RequirementRegistry::load(&layout.primary_specification)?;
    if registry.contract_profile != CONTRACT_PROFILE {
        return Err(SpecError::new(format!(
            "registry profile {:?} does not match reference evaluator {:?}",
            registry.contract_profile, CONTRACT_PROFILE
        )));
    }
    let cases = case::load_manifests(&layout.spec_root.join("cases"))?;
    let executor_case = case::ExecutorCase::load(
        &layout
            .spec_root
            .join("cases/transactions/commit-and-abort.mec"),
    )?;
    let observations = observer::observe(&executor_case, &layout.repo_root, &cases)?;
    evaluate(&layout, artifacts, registry, cases, observations, None)
}

/// Records a complete check as a content-addressed determinism bundle.
pub fn record(
    specification_path: impl AsRef<Path>,
    store: impl AsRef<Path>,
) -> Result<RecordedBundle> {
    determinism::record(check(specification_path)?, store.as_ref())
}

/// Re-evaluates a recorded bundle using its stored observations and source
/// blobs. Runtime observation is intentionally not repeated during replay.
pub fn replay(bundle_hash: &str, store: impl AsRef<Path>) -> Result<ReplayReport> {
    determinism::replay(bundle_hash, store.as_ref())
}

/// Runs the intentionally broken cases that prove the checker rejects drift,
/// missing evidence, broken semantic links, and non-reproducible results.
pub fn demonstrate(
    specification_path: impl AsRef<Path>,
    store: impl AsRef<Path>,
) -> Result<DemonstrationReport> {
    let layout = SpecLayout::discover(specification_path.as_ref())?;
    let artifacts = compile_source_profiles(&layout)?;
    validate_document_links(&artifacts)?;
    let registry = RequirementRegistry::load(&layout.primary_specification)?;
    let cases = case::load_manifests(&layout.spec_root.join("cases"))?;
    let executor_case = case::ExecutorCase::load(
        &layout
            .spec_root
            .join("cases/transactions/commit-and-abort.mec"),
    )?;
    let observations = observer::observe(&executor_case, &layout.repo_root, &cases)?;
    let base = evaluate(
        &layout,
        artifacts.clone(),
        registry.clone(),
        cases.clone(),
        observations.clone(),
        None,
    )?;
    let mut scenarios = Vec::new();

    scenarios.push(DemonstrationScenario {
        id: "canonical-run".to_string(),
        expected: "CONFORMING".to_string(),
        observed: if base.passed {
            "CONFORMING".to_string()
        } else {
            "NON-CONFORMING".to_string()
        },
        passed: base.passed,
        detail: format!(
            "{} claims, {} predicates, and {} evidence batches were evaluated",
            base.requirements.len(),
            base.constraints.len(),
            base.evidence.len(),
        ),
    });

    let mut semantic_observations = observations.clone();
    semantic_observations.abort_after_state = "deliberately-corrupted".to_string();
    let semantic_report = evaluate(
        &layout,
        artifacts.clone(),
        registry.clone(),
        cases.clone(),
        semantic_observations,
        None,
    )?;
    let semantic = required_constraint(&semantic_report, "TURN-004")?;
    let semantic_detected = !semantic.passed
        && !semantic.reference_passed
        && semantic.mech_passed == Some(false)
        && semantic.witness.is_some();
    scenarios.push(DemonstrationScenario {
        id: "semantic-drift".to_string(),
        expected: "TURN-004 FAIL with matching evaluator witness".to_string(),
        observed: constraint_outcome(semantic),
        passed: semantic_detected,
        detail: semantic
            .witness
            .as_ref()
            .map(|witness| witness.detail.clone())
            .unwrap_or_else(|| "no violation witness was produced".to_string()),
    });

    let recorded = determinism::record(semantic_report.clone(), store.as_ref())?;
    let replayed = determinism::replay(&recorded.bundle_hash, store.as_ref())?;
    scenarios.push(DemonstrationScenario {
        id: "semantic-drift-replay".to_string(),
        expected: "identical failed judgment and witness".to_string(),
        observed: if replayed.passed {
            "identical replay".to_string()
        } else {
            "replay mismatch".to_string()
        },
        passed: replayed.passed && !replayed.original_conforming,
        detail: format!(
            "bundle {}; judgments={}, witnesses={}",
            replayed.bundle_hash, replayed.judgments_match, replayed.witnesses_match,
        ),
    });

    let mut activation_observations = observations.clone();
    activation_observations.activation_instance_created = true;
    let activation_report = evaluate(
        &layout,
        artifacts.clone(),
        registry.clone(),
        cases.clone(),
        activation_observations,
        None,
    )?;
    let activation = required_constraint(&activation_report, "ACT-002")?;
    scenarios.push(DemonstrationScenario {
        id: "failed-activation-instance-leak".to_string(),
        expected: "ACT-002 FAIL".to_string(),
        observed: constraint_outcome(activation),
        passed: !activation.passed && activation.mech_passed == Some(false),
        detail: "the mutant reports a runnable instance after authorization denial".to_string(),
    });

    let mut architecture_observations = observations.clone();
    architecture_observations.repository_resident_parser_imports = true;
    architecture_observations
        .repository_parser_import_paths
        .push("src/runtime/src/runtime/program/hosts/gpu.rs -> mech_syntax::parser".to_string());
    let architecture_report = evaluate(
        &layout,
        artifacts.clone(),
        registry.clone(),
        cases.clone(),
        architecture_observations,
        None,
    )?;
    let architecture = required_constraint(&architecture_report, "ARCH-011")?;
    scenarios.push(DemonstrationScenario {
        id: "architecture-drift".to_string(),
        expected: "ARCH-011 FAIL".to_string(),
        observed: constraint_outcome(architecture),
        passed: !architecture.passed && architecture.mech_passed == Some(false),
        detail: "injected dependency: resident hosts/gpu module -> parser internals".to_string(),
    });

    let mut backend_observations = observations.clone();
    backend_observations.backend_admission_result = "fallback".to_string();
    backend_observations.backend_admission_reason =
        "mutant silently selected another backend".to_string();
    let backend_report = evaluate(
        &layout,
        artifacts.clone(),
        registry.clone(),
        cases.clone(),
        backend_observations,
        None,
    )?;
    let backend = required_constraint(&backend_report, "GPU-001")?;
    scenarios.push(DemonstrationScenario {
        id: "backend-admission-drift".to_string(),
        expected: "GPU-001 FAIL".to_string(),
        observed: constraint_outcome(backend),
        passed: !backend.passed && backend.mech_passed == Some(false),
        detail: "a silent fallback is rejected; explicit unsupported remains conforming"
            .to_string(),
    });

    let specification_source =
        fs::read_to_string(&layout.primary_specification).map_err(|error| {
            SpecError::new(format!(
                "could not read {} for link mutation: {error}",
                layout.primary_specification.display()
            ))
        })?;
    let broken_source = specification_source.replacen(
        "{transaction-abort-status}",
        "{missing-transaction-status}",
        1,
    );
    let broken_artifact = MechArtifact::compile_source(
        Path::new("mutations/broken-link.mspec"),
        SourceProfile::Specification,
        &broken_source,
    )?;
    let broken_link_result = validate_document_links(&[broken_artifact]);
    let broken_detail = broken_link_result
        .as_ref()
        .err()
        .map(ToString::to_string)
        .unwrap_or_else(|| "broken link incorrectly passed validation".to_string());
    scenarios.push(DemonstrationScenario {
        id: "broken-prose-reference".to_string(),
        expected: "SPEC INVALID before conformance evaluation".to_string(),
        observed: if broken_link_result.is_err() {
            "SPEC INVALID".to_string()
        } else {
            "accepted".to_string()
        },
        passed: broken_link_result.is_err()
            && broken_detail.contains("missing-transaction-status")
            && broken_detail.contains("before conformance evaluation"),
        detail: broken_detail,
    });

    let missing_evidence = base
        .evidence
        .iter()
        .filter(|batch| batch.provider != "runtime")
        .cloned()
        .collect::<Vec<_>>();
    let missing_profiles = evaluate_profiles(
        &registry,
        &base.requirements,
        &base.constraints,
        &missing_evidence,
        base.evaluators_agree,
    );
    let missing_status = missing_profiles
        .iter()
        .find(|result| result.requirement == "TURN-004" && result.profile == "resident-cpu")
        .map(|result| result.implementation_status);
    scenarios.push(DemonstrationScenario {
        id: "missing-evidence".to_string(),
        expected: "NOT EVALUATED, never PASS".to_string(),
        observed: missing_status
            .map(|status| status.to_string())
            .unwrap_or_else(|| "missing result".to_string()),
        passed: missing_status == Some(ConformanceStatus::NotEvaluated),
        detail: "the runtime evidence batch was removed from an otherwise passing run".to_string(),
    });

    let gpu_status = base
        .profiles
        .iter()
        .find(|result| result.requirement == "GPU-001" && result.profile == "gpu")
        .map(|result| result.implementation_status);
    scenarios.push(DemonstrationScenario {
        id: "permitted-backend-rejection".to_string(),
        expected: "CONFORMING REJECTION".to_string(),
        observed: gpu_status
            .map(|status| status.to_string())
            .unwrap_or_else(|| "missing result".to_string()),
        passed: gpu_status == Some(ConformanceStatus::ConformingRejection),
        detail: "this prototype declares GPU observation unsupported instead of claiming unrun behavior passed"
            .to_string(),
    });

    let critical_mutations = base
        .requirements
        .iter()
        .flat_map(|requirement| requirement.mutations.iter())
        .filter(|mutation| mutation.severity == "critical")
        .collect::<Vec<_>>();
    let killed_mutations = critical_mutations
        .iter()
        .filter(|mutation| mutation.detected)
        .count();
    scenarios.push(DemonstrationScenario {
        id: "critical-mutation-strength".to_string(),
        expected: "all critical specification mutants killed".to_string(),
        observed: format!("{killed_mutations}/{} killed", critical_mutations.len()),
        passed: !critical_mutations.is_empty() && killed_mutations == critical_mutations.len(),
        detail:
            "positive and counterexample bindings execute against each critical predicate mutant"
                .to_string(),
    });

    let passed = scenarios.iter().all(|scenario| scenario.passed);
    let commit = base
        .evidence
        .first()
        .map(|batch| batch.repository_commit.clone())
        .unwrap_or_else(|| "unknown".to_string());
    Ok(DemonstrationReport {
        commit,
        specification_version: base.specification_version,
        scenarios,
        passed,
    })
}

fn required_constraint<'a>(
    report: &'a CheckReport,
    requirement: &str,
) -> Result<&'a ConstraintResult> {
    report
        .constraints
        .iter()
        .find(|constraint| constraint.requirement == requirement && !constraint.passed)
        .or_else(|| {
            report
                .constraints
                .iter()
                .find(|constraint| constraint.requirement == requirement)
        })
        .ok_or_else(|| SpecError::new(format!("no predicate was evaluated for {requirement}")))
}

fn constraint_outcome(constraint: &ConstraintResult) -> String {
    format!(
        "{} (reference={}, resident Mech={})",
        if constraint.passed { "PASS" } else { "FAIL" },
        constraint.reference_passed,
        constraint
            .mech_passed
            .map(|value| value.to_string())
            .unwrap_or_else(|| "missing".to_string()),
    )
}

fn evaluate(
    layout: &SpecLayout,
    artifacts: Vec<MechArtifact>,
    registry: RequirementRegistry,
    cases: Vec<case::CaseManifest>,
    observations: ExecutorObservations,
    evidence_override: Option<Vec<EvidenceBatch>>,
) -> Result<CheckReport> {
    let observation_source = observations.to_mech_source();
    let specifications = load_specifications(&layout.spec_root)?;
    let (constraints, all_evaluators_agree) =
        evaluate_constraints(&observation_source, &specifications)?;
    let requirements = registry::evaluate_arch(&registry, &constraints)?;
    let specification_version = registry::specification_version(&requirements);
    let evidence = evidence_override.unwrap_or_else(|| {
        evidence::collect(
            &layout.spec_root,
            &layout.repo_root,
            &specification_version,
            &observations,
        )
    });
    let profiles = evaluate_profiles(
        &registry,
        &requirements,
        &constraints,
        &evidence,
        all_evaluators_agree,
    );
    let coverage = coverage(&registry, &cases, &evidence, &profiles);
    let passed = all_evaluators_agree
        && requirements
            .iter()
            .all(|requirement| requirement.status == SpecificationStatus::Ratified)
        && constraints.iter().all(|constraint| constraint.passed)
        && profiles
            .iter()
            .all(|result| result.implementation_status.is_conforming())
        && coverage.uncovered.is_empty();

    Ok(CheckReport {
        profile: CONTRACT_PROFILE.to_string(),
        specification_version,
        observations,
        specifications: specifications
            .into_iter()
            .map(|document| document.display_path)
            .collect(),
        artifacts,
        requirements,
        constraints,
        evidence,
        profiles,
        coverage,
        change_attempts: Vec::new(),
        evaluators_agree: all_evaluators_agree,
        passed,
    })
}

fn evaluate_constraints(
    observations: &str,
    specifications: &[SpecificationDocument],
) -> Result<(Vec<ConstraintResult>, bool)> {
    let mut constraints = Vec::new();
    let mut all_evaluators_agree = true;
    for specification in specifications {
        let reference = reference::evaluate(observations, &specification.source)?;
        let current = current::evaluate(observations, &reference)?;
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
                evaluation.detail.clone()
            } else {
                format!(
                    "evaluator disagreement: reference={}, resident Mech={}",
                    evaluation.passed,
                    mech_passed
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "missing".to_string()),
                )
            };
            let witness = (!passed).then(|| ViolationWitness {
                kind: if evaluators_agree {
                    "value-mismatch".to_string()
                } else {
                    "evaluator-disagreement".to_string()
                },
                observed: evaluation.observed.clone(),
                expected: evaluation.expected.clone(),
                detail: detail.clone(),
            });
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
                witness,
            });
        }
    }
    constraints.sort_by(|left, right| {
        (&left.requirement, &left.name).cmp(&(&right.requirement, &right.name))
    });
    Ok((constraints, all_evaluators_agree))
}

fn evaluate_profiles(
    registry: &RequirementRegistry,
    arch: &[RequirementArchReport],
    constraints: &[ConstraintResult],
    evidence: &[EvidenceBatch],
    all_evaluators_agree: bool,
) -> Vec<ProfileResult> {
    let profiles = registry
        .requirements
        .iter()
        .flat_map(|requirement| requirement.profiles.iter().map(|rule| rule.profile.clone()))
        .collect::<BTreeSet<_>>();
    let arch = arch
        .iter()
        .map(|report| (report.requirement.as_str(), report))
        .collect::<BTreeMap<_, _>>();
    let mut results = Vec::new();
    for requirement in &registry.requirements {
        for profile in &profiles {
            let rule = requirement
                .profiles
                .iter()
                .find(|rule| rule.profile == *profile);
            let expectation = rule
                .map(|rule| rule.expectation.as_str())
                .unwrap_or("not-applicable");
            let specification_status = arch
                .get(requirement.id.as_str())
                .map(|report| report.status)
                .unwrap_or(SpecificationStatus::SpecInvalid);
            let requirement_constraints = constraints
                .iter()
                .filter(|constraint| constraint.requirement == requirement.id)
                .collect::<Vec<_>>();
            let required_batches = requirement
                .evidence
                .iter()
                .map(|rule| {
                    (
                        rule,
                        evidence.iter().find(|batch| {
                            batch.provider == rule.provider && batch.schema_version == rule.schema
                        }),
                    )
                })
                .collect::<Vec<_>>();
            let evidence_runs = required_batches
                .iter()
                .filter_map(|(_, batch)| batch.map(|batch| batch.run_id.clone()))
                .collect::<Vec<_>>();
            let (implementation_status, detail) = profile_status(
                expectation,
                specification_status,
                &requirement_constraints,
                &required_batches,
                all_evaluators_agree,
            );
            results.push(ProfileResult {
                requirement: requirement.id.clone(),
                profile: profile.clone(),
                expectation: expectation.to_string(),
                specification_status,
                implementation_status,
                evidence_runs,
                detail,
            });
        }
    }
    results
}

fn profile_status(
    expectation: &str,
    specification_status: SpecificationStatus,
    constraints: &[&ConstraintResult],
    evidence: &[(&EvidenceRule, Option<&EvidenceBatch>)],
    all_evaluators_agree: bool,
) -> (ConformanceStatus, String) {
    if expectation == "not-applicable" {
        return (
            ConformanceStatus::NotApplicable,
            "the specification declares this profile outside the requirement scope".to_string(),
        );
    }
    if specification_status != SpecificationStatus::Ratified {
        return (
            ConformanceStatus::SpecInvalid,
            format!("governing requirement is {specification_status}"),
        );
    }
    if !all_evaluators_agree
        || constraints
            .iter()
            .any(|constraint| !constraint.evaluators_agree)
    {
        return (
            ConformanceStatus::EvaluatorDisagreement,
            "the current and reference contract evaluators disagree".to_string(),
        );
    }
    if evidence.iter().any(|(_, batch)| batch.is_none()) {
        return (
            ConformanceStatus::NotEvaluated,
            "required evidence is unavailable".to_string(),
        );
    }
    for (rule, batch) in evidence {
        let batch = batch.expect("missing evidence returned above");
        if !grade_satisfies(batch.grade, &rule.minimum_grade) {
            return (
                ConformanceStatus::InvalidEvidence,
                format!(
                    "{} evidence grade {:?} is below required {}",
                    rule.provider, batch.grade, rule.minimum_grade
                ),
            );
        }
        match batch.status {
            EvidenceStatus::Unsupported | EvidenceStatus::Denied if expectation == "may-reject" => {
                return (
                    ConformanceStatus::ConformingRejection,
                    format!(
                        "{} explicitly reported {} as permitted by the profile",
                        rule.provider, batch.status
                    ),
                );
            }
            EvidenceStatus::Unsupported | EvidenceStatus::Denied => {
                return (
                    ConformanceStatus::NotEvaluated,
                    format!(
                        "{} reported {}; the required behavior was not evaluated",
                        rule.provider, batch.status
                    ),
                );
            }
            EvidenceStatus::Error | EvidenceStatus::Stale | EvidenceStatus::Incomplete => {
                return (
                    ConformanceStatus::InvalidEvidence,
                    format!("{} evidence status is {}", rule.provider, batch.status),
                );
            }
            EvidenceStatus::Present => {}
        }
    }
    if constraints.is_empty() {
        return (
            ConformanceStatus::NotEvaluated,
            "the claim has no evaluated predicate".to_string(),
        );
    }
    if constraints.iter().all(|constraint| constraint.passed) {
        (
            ConformanceStatus::Pass,
            "all bound predicates passed with current evidence".to_string(),
        )
    } else {
        let failed = constraints
            .iter()
            .filter(|constraint| !constraint.passed)
            .map(|constraint| constraint.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        (
            ConformanceStatus::Fail,
            format!("failed predicates: {failed}"),
        )
    }
}

fn grade_satisfies(actual: EvidenceGrade, required: &str) -> bool {
    let required = match required {
        "declared" => EvidenceGrade::Declared,
        "observed" => EvidenceGrade::Observed,
        "differential" => EvidenceGrade::Differential,
        "replayed" => EvidenceGrade::Replayed,
        "reproduced" => EvidenceGrade::Reproduced,
        "proved" => EvidenceGrade::Proved,
        _ => return false,
    };
    actual >= required
}

fn coverage(
    registry: &RequirementRegistry,
    cases: &[case::CaseManifest],
    evidence: &[EvidenceBatch],
    profiles: &[ProfileResult],
) -> CoverageReport {
    let case_requirements = cases
        .iter()
        .flat_map(|case| case.requirements.iter().cloned())
        .collect::<BTreeSet<_>>();
    let mut with_case = 0;
    let mut with_provider = 0;
    let mut with_current = 0;
    let mut judged = 0;
    let mut uncovered = Vec::new();
    for requirement in &registry.requirements {
        let has_case = case_requirements.contains(&requirement.id);
        let has_provider = !requirement.evidence.is_empty();
        let has_current = has_provider
            && requirement.evidence.iter().all(|rule| {
                evidence.iter().any(|batch| {
                    batch.provider == rule.provider && batch.schema_version == rule.schema
                })
            });
        let is_judged = profiles.iter().any(|result| {
            result.requirement == requirement.id
                && !matches!(
                    result.implementation_status,
                    ConformanceStatus::NotApplicable
                        | ConformanceStatus::NotEvaluated
                        | ConformanceStatus::InvalidEvidence
                        | ConformanceStatus::SpecInvalid
                        | ConformanceStatus::EvaluatorDisagreement
                )
        });
        with_case += usize::from(has_case);
        with_provider += usize::from(has_provider);
        with_current += usize::from(has_current);
        judged += usize::from(is_judged);
        if !(has_case && has_provider && has_current && is_judged) {
            uncovered.push(requirement.id.clone());
        }
    }
    CoverageReport {
        requirements: registry.requirements.len(),
        requirements_with_named_case: with_case,
        requirements_with_provider: with_provider,
        requirements_with_current_evidence: with_current,
        judged_requirements: judged,
        uncovered,
    }
}

pub fn write_json_report(report: &CheckReport, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    ensure_parent(path)?;
    let json = serde_json::to_string_pretty(report)
        .map_err(|error| SpecError::new(format!("could not serialize report: {error}")))?;
    fs::write(path, format!("{json}\n")).map_err(|error| {
        SpecError::new(format!(
            "could not write report {}: {error}",
            path.display()
        ))
    })
}

pub fn write_demonstration_report(
    report: &DemonstrationReport,
    path: impl AsRef<Path>,
) -> Result<()> {
    let path = path.as_ref();
    ensure_parent(path)?;
    let json = serde_json::to_string_pretty(report).map_err(|error| {
        SpecError::new(format!("could not serialize demonstration report: {error}"))
    })?;
    fs::write(path, format!("{json}\n")).map_err(|error| {
        SpecError::new(format!(
            "could not write demonstration report {}: {error}",
            path.display()
        ))
    })
}

pub fn write_html_report(report: &CheckReport, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    ensure_parent(path)?;
    let mut html = String::from(
        "<!doctype html><meta charset=\"utf-8\"><title>Mech conformance</title>\
         <style>body{font:15px system-ui;max-width:1100px;margin:2rem auto;padding:0 1rem}\
         table{border-collapse:collapse;width:100%}th,td{border:1px solid #bbb;padding:.45rem;text-align:left}\
         .PASS,.RATIFIED,.CONFORMING{color:#087830}.FAIL,.INVALID,.DISAGREEMENT{color:#b42318}\
         code{background:#eee;padding:.1rem .25rem}</style>",
    );
    html.push_str(&format!(
        "<h1>Mech reflective conformance</h1><p>Specification <code>{}</code>; result <strong class=\"{}\">{}</strong>.</p>",
        escape(&report.specification_version),
        if report.passed { "CONFORMING" } else { "FAIL" },
        if report.passed { "CONFORMING" } else { "NON-CONFORMING" },
    ));
    html.push_str("<h2>Claims</h2><table><tr><th>Claim</th><th>Prose</th><th>Predicate gloss</th><th>Specification</th></tr>");
    for requirement in &report.requirements {
        html.push_str(&format!(
            "<tr id=\"claim-{}\"><td><a href=\"#result-{}\">{}</a></td><td>{}</td><td>{}</td><td class=\"{}\">{}</td></tr>",
            escape(&requirement.requirement),
            escape(&requirement.requirement),
            escape(&requirement.requirement),
            escape(&requirement.normative_prose),
            escape(&requirement.generated_gloss),
            escape(&requirement.status.to_string()),
            requirement.status,
        ));
    }
    html.push_str("</table><h2>Conformance</h2><table><tr><th>Claim</th><th>Profile</th><th>Result</th><th>Evidence</th><th>Witness</th></tr>");
    for result in &report.profiles {
        let witness = report
            .constraints
            .iter()
            .find(|constraint| {
                constraint.requirement == result.requirement && constraint.witness.is_some()
            })
            .and_then(|constraint| constraint.witness.as_ref())
            .map(|witness| witness.detail.clone())
            .unwrap_or_default();
        html.push_str(&format!(
            "<tr id=\"result-{}\"><td><a href=\"#claim-{}\">{}</a></td><td>{}</td><td class=\"{}\">{}</td><td>{}</td><td>{}</td></tr>",
            escape(&result.requirement),
            escape(&result.requirement),
            escape(&result.requirement),
            escape(&result.profile),
            escape(&result.implementation_status.to_string()),
            result.implementation_status,
            escape(&result.evidence_runs.join(", ")),
            escape(&witness),
        ));
    }
    html.push_str("</table>");
    fs::write(path, html).map_err(|error| {
        SpecError::new(format!(
            "could not write HTML report {}: {error}",
            path.display()
        ))
    })
}

fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            SpecError::new(format!(
                "could not create report directory {}: {error}",
                parent.display()
            ))
        })?;
    }
    Ok(())
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn compile_source_profiles(layout: &SpecLayout) -> Result<Vec<MechArtifact>> {
    [
        layout
            .spec_root
            .join("cases/transactions/commit-and-abort.mec"),
        layout.spec_root.join("check.mcfg"),
        layout.primary_specification.clone(),
        layout.repo_root.join("docs/specification-guide.mdoc"),
    ]
    .iter()
    .map(|path| MechArtifact::compile(path))
    .collect()
}

fn validate_document_links(artifacts: &[MechArtifact]) -> Result<()> {
    let unresolved = artifacts
        .iter()
        .flat_map(|artifact| {
            artifact
                .unresolved_links()
                .into_iter()
                .map(move |link| (artifact, link))
        })
        .collect::<Vec<_>>();
    if unresolved.is_empty() {
        return Ok(());
    }
    let details = unresolved
        .iter()
        .map(|(artifact, link)| {
            format!(
                "{}:{}:{} unresolved inline symbol `{}`",
                artifact.source_path,
                link.span.start_line,
                link.span.start_column,
                link.symbol_name
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    Err(SpecError::new(format!(
        "specification document validation failed before conformance evaluation: {details}"
    )))
}

fn load_specifications(root: &Path) -> Result<Vec<SpecificationDocument>> {
    let mut paths = Vec::new();
    collect_source_files(root, "mspec", &mut paths)?;
    paths.sort();
    let mut specifications = Vec::new();
    for path in paths {
        let source = fs::read_to_string(&path).map_err(|error| {
            SpecError::new(format!(
                "could not read specification {}: {error}",
                path.display()
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
            "no executable .mspec contract specifications found under {}",
            root.display()
        )));
    }
    Ok(specifications)
}

fn collect_source_files(
    directory: &Path,
    extension: &str,
    output: &mut Vec<PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(directory).map_err(|error| {
        SpecError::new(format!(
            "could not inspect specification directory {}: {error}",
            directory.display()
        ))
    })? {
        let entry = entry.map_err(|error| SpecError::new(format!("inspect source: {error}")))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| SpecError::new(format!("inspect {}: {error}", path.display())))?;
        if file_type.is_dir() && path.file_name().is_none_or(|name| name != "cases") {
            collect_source_files(&path, extension, output)?;
        } else if file_type.is_file() && path.extension().is_some_and(|value| value == extension) {
            output.push(path);
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct SpecLayout {
    repo_root: PathBuf,
    spec_root: PathBuf,
    primary_specification: PathBuf,
}

impl SpecLayout {
    fn discover(path: &Path) -> Result<Self> {
        let path = fs::canonicalize(path).map_err(|error| {
            SpecError::new(format!(
                "could not resolve specification path {}: {error}",
                path.display()
            ))
        })?;
        let (spec_root, primary_specification) = if path.is_dir() {
            (path.clone(), path.join("platform.mspec"))
        } else {
            let root = path.parent().ok_or_else(|| {
                SpecError::new(format!("specification {} has no parent", path.display()))
            })?;
            (root.to_path_buf(), path)
        };
        if !primary_specification.is_file() {
            return Err(SpecError::new(format!(
                "primary specification {} does not exist",
                primary_specification.display()
            )));
        }
        let repo_root = spec_root.parent().ok_or_else(|| {
            SpecError::new(format!(
                "specification root {} has no repository parent",
                spec_root.display()
            ))
        })?;
        Ok(Self {
            repo_root: repo_root.to_path_buf(),
            spec_root,
            primary_specification,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
    }

    #[test]
    fn reflective_vertical_slice_conforms_end_to_end() {
        let report = check(root().join("platform.mspec")).unwrap();
        assert!(report.passed, "{}", report.render_text());
        assert_eq!(report.observations.resident_route, "resident-pure");
        assert_eq!(report.observations.activation_outcome, "rejected");
        assert_eq!(
            report.observations.activation_failure_class,
            "authorization-denied"
        );
        assert!(!report.observations.activation_instance_created);
        assert!(report.evaluators_agree);
        assert!(report.coverage.uncovered.is_empty());
        assert_eq!(report.artifacts.len(), 4);
        assert!(
            report
                .artifacts
                .iter()
                .any(|artifact| artifact.profile == SourceProfile::Specification)
        );
    }

    #[test]
    fn injected_abort_drift_produces_a_turn_004_witness_in_both_evaluators() {
        let layout = SpecLayout::discover(&root().join("platform.mspec")).unwrap();
        let artifacts = compile_source_profiles(&layout).unwrap();
        let registry = RequirementRegistry::load(&layout.primary_specification).unwrap();
        let cases = case::load_manifests(&layout.spec_root.join("cases")).unwrap();
        let executor_case = case::ExecutorCase::load(
            &layout
                .spec_root
                .join("cases/transactions/commit-and-abort.mec"),
        )
        .unwrap();
        let mut observations =
            observer::observe(&executor_case, &layout.repo_root, &cases).unwrap();
        observations.abort_after_state = "deliberately-corrupted".to_string();
        let report = evaluate(&layout, artifacts, registry, cases, observations, None).unwrap();
        let rollback = report
            .constraints
            .iter()
            .find(|constraint| constraint.requirement == "TURN-004")
            .unwrap();
        assert!(!rollback.passed);
        assert_eq!(rollback.mech_passed, Some(false));
        assert!(rollback.witness.is_some());
        assert!(report.profiles.iter().any(|result| {
            result.requirement == "TURN-004"
                && result.implementation_status == ConformanceStatus::Fail
        }));
    }

    #[test]
    fn missing_required_evidence_is_not_evaluated_never_pass() {
        let mut report = check(root()).unwrap();
        report.evidence.retain(|batch| batch.provider != "runtime");
        let registry = RequirementRegistry::load(&root().join("platform.mspec")).unwrap();
        report.profiles = evaluate_profiles(
            &registry,
            &report.requirements,
            &report.constraints,
            &report.evidence,
            report.evaluators_agree,
        );
        assert!(report.profiles.iter().any(|result| {
            result.requirement == "TURN-004"
                && result.implementation_status == ConformanceStatus::NotEvaluated
        }));
        assert!(!report.profiles.iter().any(|result| {
            result.requirement == "TURN-004"
                && result.implementation_status == ConformanceStatus::Pass
        }));
    }

    #[test]
    fn content_addressed_bundle_replays_identically() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let store = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/spec/tests")
            .join(format!("bundle-{unique}"));
        let recorded = record(root().join("platform.mspec"), &store).unwrap();
        let replayed = replay(&recorded.bundle_hash, &store).unwrap();
        assert!(replayed.passed, "{}", replayed.render_text());
        assert!(replayed.judgments_match);
        assert!(replayed.witnesses_match);
        assert!(replayed.original_conforming);
    }

    #[test]
    fn failure_demonstration_detects_every_injected_fault() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let store = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/spec/tests")
            .join(format!("demo-{unique}"));
        let report = demonstrate(root().join("platform.mspec"), store).unwrap();
        assert!(report.passed, "{}", report.render_text());
        assert_eq!(report.scenarios.len(), 10);
        assert!(report.scenarios.iter().all(|scenario| scenario.passed));
    }
}
