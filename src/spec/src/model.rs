use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
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
        )
    }
}

fn quote(value: &str) -> String {
    serde_json::to_string(value).expect("serializing a string cannot fail")
}

#[derive(Clone, Debug, Serialize)]
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
}

#[derive(Clone, Debug, Serialize)]
pub struct CoverageReport {
    pub requirements: usize,
    pub requirements_with_evidence: usize,
    pub uncovered: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CheckReport {
    pub profile: String,
    pub observations: ExecutorObservations,
    pub specifications: Vec<String>,
    pub constraints: Vec<ConstraintResult>,
    pub coverage: CoverageReport,
    pub evaluators_agree: bool,
    pub passed: bool,
}

impl CheckReport {
    pub fn render_text(&self) -> String {
        let mut lines = vec![
            "Executable Mech specification".to_string(),
            format!("  profile:  {}", self.profile),
            format!("  executor: {}", self.observations.executor),
            format!("  route:    {}", self.observations.resident_route),
            format!("  case:     {}", self.observations.case_id),
            String::new(),
        ];
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
        let passed = self.constraints.iter().filter(|item| item.passed).count();
        let failed = self.constraints.len().saturating_sub(passed);
        lines.extend([
            String::new(),
            format!("  constraints: {passed} passed, {failed} failed"),
            format!(
                "  evaluators:  {}",
                if self.evaluators_agree {
                    "reference and resident Mech agree"
                } else {
                    "DISAGREE"
                }
            ),
            format!(
                "  coverage:    {}/{} requirements",
                self.coverage.requirements_with_evidence, self.coverage.requirements,
            ),
            format!(
                "  result:      {}",
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
