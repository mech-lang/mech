use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::source_profile::MechArtifact;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutorObservations {
    pub case_id: String,
    pub executor: String,
    pub resident_route: String,
    pub resident_initial_state: String,
    pub resident_next_turn_state: String,
    pub resident_turns_advanced: bool,
    pub commit_before_state: String,
    pub commit_visible_state: String,
    pub commit_after_state: String,
    pub commit_outcome: String,
    pub commit_record_observed: bool,
    pub commit_event_observed: bool,
    pub abort_before_state: String,
    pub abort_visible_state: String,
    pub abort_after_state: String,
    pub abort_outcome: String,
    pub abort_event_observed: bool,
    pub next_turn_state: String,
    pub shutdown_ingress_closed: bool,
    pub shutdown_input_rejected: bool,
    pub shutdown_event_observed: bool,
    pub event_names: Vec<String>,
    pub activation_outcome: String,
    pub activation_failure_class: String,
    pub activation_instance_created: bool,
    pub backend_admission_result: String,
    pub backend_admission_reason: String,
    pub repository_resident_parser_imports: bool,
    pub repository_parser_import_paths: Vec<String>,
    pub repository_scanned_paths: Vec<String>,
    pub benchmark_reference_protocol: String,
    pub benchmark_candidate_protocol: String,
}

impl ExecutorObservations {
    pub(crate) fn to_mech_source(&self) -> String {
        format!(
            concat!(
                "resident-route := {resident_route}\n",
                "resident-initial-state := {resident_initial}\n",
                "resident-next-turn-state := {resident_next}\n",
                "resident-turns-advanced := {resident_advanced}\n",
                "commit-before-state := {commit_before}\n",
                "commit-visible-state := {commit_visible}\n",
                "commit-after-state := {commit_after}\n",
                "commit-outcome := {commit_outcome}\n",
                "commit-record-observed := {commit_record}\n",
                "commit-event-observed := {commit_event}\n",
                "abort-before-state := {abort_before}\n",
                "abort-visible-state := {abort_visible}\n",
                "abort-after-state := {abort_after}\n",
                "abort-outcome := {abort_outcome}\n",
                "abort-event-observed := {abort_event}\n",
                "next-turn-state := {next_turn}\n",
                "shutdown-ingress-closed := {shutdown_closed}\n",
                "shutdown-input-rejected := {shutdown_rejected}\n",
                "shutdown-event-observed := {shutdown_event}\n",
                "activation-outcome := {activation_outcome}\n",
                "activation-failure-class := {activation_failure_class}\n",
                "activation-instance-created := {activation_instance_created}\n",
                "backend-admission-result := {backend_admission}\n",
                "repository-resident-parser-imports := {repository_imports}\n",
                "benchmark-reference-protocol := {benchmark_reference}\n",
                "benchmark-candidate-protocol := {benchmark_candidate}\n",
            ),
            resident_route = quote(&self.resident_route),
            resident_initial = quote(&self.resident_initial_state),
            resident_next = quote(&self.resident_next_turn_state),
            resident_advanced = self.resident_turns_advanced,
            commit_before = quote(&self.commit_before_state),
            commit_visible = quote(&self.commit_visible_state),
            commit_after = quote(&self.commit_after_state),
            commit_outcome = quote(&self.commit_outcome),
            commit_record = self.commit_record_observed,
            commit_event = self.commit_event_observed,
            abort_before = quote(&self.abort_before_state),
            abort_visible = quote(&self.abort_visible_state),
            abort_after = quote(&self.abort_after_state),
            abort_outcome = quote(&self.abort_outcome),
            abort_event = self.abort_event_observed,
            next_turn = quote(&self.next_turn_state),
            shutdown_closed = self.shutdown_ingress_closed,
            shutdown_rejected = self.shutdown_input_rejected,
            shutdown_event = self.shutdown_event_observed,
            activation_outcome = quote(&self.activation_outcome),
            activation_failure_class = quote(&self.activation_failure_class),
            activation_instance_created = self.activation_instance_created,
            backend_admission = quote(&self.backend_admission_result),
            repository_imports = self.repository_resident_parser_imports,
            benchmark_reference = quote(&self.benchmark_reference_protocol),
            benchmark_candidate = quote(&self.benchmark_candidate_protocol),
        )
    }
}

fn quote(value: &str) -> String {
    serde_json::to_string(value).expect("serializing a string cannot fail")
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConstraintResult {
    pub specification: String,
    pub requirement: String,
    pub name: String,
    pub expression: String,
    pub reference_passed: bool,
    pub mech_passed: Option<bool>,
    pub evaluators_agree: bool,
    pub passed: bool,
    pub detail: String,
    pub witness: Option<ViolationWitness>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViolationWitness {
    pub kind: String,
    pub observed: String,
    pub expected: String,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SpecificationStatus {
    Draft,
    MutationSurvived,
    ReviewRequired,
    Ratified,
    SpecInvalid,
}

impl Display for SpecificationStatus {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Draft => "DRAFT",
            Self::MutationSurvived => "MUTATION SURVIVED",
            Self::ReviewRequired => "REVIEW REQUIRED",
            Self::Ratified => "RATIFIED",
            Self::SpecInvalid => "SPEC INVALID",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConformanceStatus {
    Pass,
    ConformingRejection,
    NotApplicable,
    Fail,
    NotEvaluated,
    InvalidEvidence,
    EvaluatorDisagreement,
    SpecInvalid,
}

impl ConformanceStatus {
    pub fn is_conforming(self) -> bool {
        matches!(
            self,
            Self::Pass | Self::ConformingRejection | Self::NotApplicable
        )
    }
}

impl Display for ConformanceStatus {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Pass => "PASS",
            Self::ConformingRejection => "CONFORMING REJECTION",
            Self::NotApplicable => "NOT APPLICABLE",
            Self::Fail => "FAIL",
            Self::NotEvaluated => "NOT EVALUATED",
            Self::InvalidEvidence => "INVALID EVIDENCE",
            Self::EvaluatorDisagreement => "EVALUATOR DISAGREEMENT",
            Self::SpecInvalid => "SPEC INVALID",
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArchIssue {
    pub code: String,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MutationReport {
    pub name: String,
    pub contract: String,
    pub severity: String,
    pub expression: String,
    pub detected: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RequirementArchReport {
    pub requirement: String,
    pub title: String,
    pub level: String,
    pub area: String,
    pub normative_prose: String,
    pub generated_gloss: String,
    pub current_arch_hash: String,
    pub ratified_arch_hash: String,
    pub reviewer: String,
    pub status: SpecificationStatus,
    pub issues: Vec<ArchIssue>,
    pub mutations: Vec<MutationReport>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceStatus {
    Present,
    Unsupported,
    Denied,
    Error,
    Stale,
    Incomplete,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceGrade {
    Declared,
    Observed,
    Differential,
    Replayed,
    Reproduced,
    Proved,
}

impl Display for EvidenceStatus {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Present => "PRESENT",
            Self::Unsupported => "UNSUPPORTED",
            Self::Denied => "DENIED",
            Self::Error => "ERROR",
            Self::Stale => "STALE",
            Self::Incomplete => "INCOMPLETE",
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvidenceBatch {
    pub run_id: String,
    pub parent_run_id: Option<String>,
    pub provider: String,
    pub provider_version: String,
    pub schema_version: String,
    pub status: EvidenceStatus,
    pub grade: EvidenceGrade,
    pub repository_commit: String,
    pub specification_version: String,
    pub input_hashes: Vec<String>,
    pub execution_profile: String,
    pub runtime_version: String,
    pub host_configuration: String,
    pub operating_system: String,
    pub timestamp_unix_ms: u128,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileResult {
    pub requirement: String,
    pub profile: String,
    pub expectation: String,
    pub specification_status: SpecificationStatus,
    pub implementation_status: ConformanceStatus,
    pub evidence_runs: Vec<String>,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoverageReport {
    pub requirements: usize,
    pub requirements_with_named_case: usize,
    pub requirements_with_provider: usize,
    pub requirements_with_current_evidence: usize,
    pub judged_requirements: usize,
    pub uncovered: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AttemptStatus {
    Landed,
    Missed,
    CollateralFailure,
    Unjudged,
    TargetChanged,
    Reverted,
}

impl Display for AttemptStatus {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Landed => "LANDED",
            Self::Missed => "MISSED",
            Self::CollateralFailure => "COLLATERAL FAILURE",
            Self::Unjudged => "UNJUDGED",
            Self::TargetChanged => "TARGET CHANGED",
            Self::Reverted => "REVERTED",
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChangeAttemptReport {
    pub id: String,
    pub base_commit: String,
    pub intent: String,
    pub targets: Vec<String>,
    pub preserves: Vec<String>,
    pub evidence_providers: Vec<String>,
    pub recovery_point: String,
    pub status: AttemptStatus,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckReport {
    pub profile: String,
    pub specification_version: String,
    pub observations: ExecutorObservations,
    pub specifications: Vec<String>,
    pub artifacts: Vec<MechArtifact>,
    pub requirements: Vec<RequirementArchReport>,
    pub constraints: Vec<ConstraintResult>,
    pub evidence: Vec<EvidenceBatch>,
    pub profiles: Vec<ProfileResult>,
    pub coverage: CoverageReport,
    pub change_attempts: Vec<ChangeAttemptReport>,
    pub evaluators_agree: bool,
    pub passed: bool,
}

impl CheckReport {
    pub fn render_text(&self) -> String {
        let mut lines = vec![
            "Executable Mech specification".to_string(),
            format!("  contract: {}", self.profile),
            format!("  spec:     {}", self.specification_version),
            format!("  executor: {}", self.observations.executor),
            format!("  route:    {}", self.observations.resident_route),
            format!("  case:     {}", self.observations.case_id),
            String::new(),
            "Specification arch".to_string(),
        ];
        for requirement in &self.requirements {
            lines.push(format!(
                "  [{status}] {id}  {title}",
                status = requirement.status,
                id = requirement.requirement,
                title = requirement.title,
            ));
            for issue in &requirement.issues {
                lines.push(format!("             {}: {}", issue.code, issue.detail));
            }
        }
        lines.push(String::new());
        lines.push("Executable contracts".to_string());
        for constraint in &self.constraints {
            let status = if constraint.passed { "PASS" } else { "FAIL" };
            lines.push(format!(
                "  [{status}] {}  {}",
                constraint.requirement, constraint.name,
            ));
            if !constraint.passed {
                lines.push(format!("         {}", constraint.detail));
            }
        }
        lines.push(String::new());
        lines.push("Conformance matrix".to_string());
        for profile in &self.profiles {
            lines.push(format!(
                "  [{status}] {requirement} @ {profile}",
                status = profile.implementation_status,
                requirement = profile.requirement,
                profile = profile.profile,
            ));
            if !profile.implementation_status.is_conforming() {
                lines.push(format!("         {}", profile.detail));
            }
        }
        let passed = self.constraints.iter().filter(|item| item.passed).count();
        let failed = self.constraints.len().saturating_sub(passed);
        let ratified = self
            .requirements
            .iter()
            .filter(|item| item.status == SpecificationStatus::Ratified)
            .count();
        lines.extend([
            String::new(),
            format!(
                "  specification: {ratified}/{} requirements ratified",
                self.requirements.len()
            ),
            format!("  constraints:   {passed} passed, {failed} failed"),
            format!(
                "  evaluators:    {}",
                if self.evaluators_agree {
                    "reference and resident Mech agree"
                } else {
                    "DISAGREE"
                }
            ),
            format!(
                "  coverage:      {}/{} requirements judged",
                self.coverage.judged_requirements, self.coverage.requirements,
            ),
            format!(
                "  result:        {}",
                if self.passed {
                    "CONFORMING"
                } else {
                    "NON-CONFORMING"
                }
            ),
        ]);
        lines.join("\n")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DemonstrationScenario {
    pub id: String,
    pub expected: String,
    pub observed: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DemonstrationReport {
    pub commit: String,
    pub specification_version: String,
    pub scenarios: Vec<DemonstrationScenario>,
    pub passed: bool,
}

impl DemonstrationReport {
    pub fn render_text(&self) -> String {
        let mut lines = vec![
            "Executable specification failure demonstration".to_string(),
            format!("  commit: {}", self.commit),
            format!("  spec:   {}", self.specification_version),
            String::new(),
        ];
        for scenario in &self.scenarios {
            lines.push(format!(
                "  [{status}] {id}: expected {expected}; observed {observed}",
                status = if scenario.passed { "PROVED" } else { "MISSED" },
                id = scenario.id,
                expected = scenario.expected,
                observed = scenario.observed,
            ));
            lines.push(format!("           {}", scenario.detail));
        }
        lines.extend([
            String::new(),
            format!(
                "  result: {}",
                if self.passed {
                    "ALL FAILURE MODES DETECTED"
                } else {
                    "DEMONSTRATION FAILED"
                }
            ),
        ]);
        lines.join("\n")
    }
}
