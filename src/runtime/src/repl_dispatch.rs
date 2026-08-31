//! Renderer-neutral command dispatch and typed host-resource requests.

use std::collections::BTreeMap;

use mech_core::MResult;

use crate::{
    ARGUMENT_QUOTING_HELP, DiagnosticPhase, MechEvent, OutputArtifactStatus, OutputContent,
    OutputSource, REPL_COMMAND_SPECS, ReplClearTarget, ReplCommand, ReplCommandId, ReplEvent,
    ReplHostRequirement, ReplRequest, ReplResponse, ReplResponseKind, ReplResponseStatus,
    ResidentReplRuntimeFactory, ResidentReplSession, ResidentSymbolInspection, Severity,
    TableOutput, TextOutput,
};

pub const REPL_TEXT_LOGO: &str = r#"
  ┌─────────┐ ┌──────┐ ┌─┐ ┌──┐ ┌─┐  ┌─┐
  └───┐ ┌───┘ └──────┘ │ │ └┐ │ │ │  │ │
  ┌─┐ │ │ ┌─┐ ┌──────┐ │ │  └─┘ │ └─┐│ │
  │ │ │ │ │ │ │ ┌────┘ │ │  ┌─┐ │ ┌─┘│ │
  │ │ └─┘ │ │ │ └────┐ │ └──┘ │ │ │  │ │
  └─┘     └─┘ └──────┘ └──────┘ └─┘  └─┘"#;

pub const MECH_DOCUMENTATION_URL: &str = "https://docs.mech-lang.org/";
pub const MECH_HOMEPAGE_URL: &str = "https://mech-lang.org/";

#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplComponentKind {
    Product,
    Library,
    Host,
}

impl ReplComponentKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Product => "product",
            Self::Library => "library",
            Self::Host => "host",
        }
    }
}

#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplComponentVersion {
    pub name: String,
    pub kind: ReplComponentKind,
    pub version: String,
}

impl ReplComponentVersion {
    pub fn new(
        name: impl Into<String>,
        kind: ReplComponentKind,
        version: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            kind,
            version: version.into(),
        }
    }
}

#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplHostAvailability {
    unavailable: BTreeMap<ReplHostRequirement, String>,
    product: ReplComponentVersion,
    components: Vec<ReplComponentVersion>,
}

impl Default for ReplHostAvailability {
    fn default() -> Self {
        Self {
            unavailable: BTreeMap::new(),
            product: ReplComponentVersion::new(
                "Mech",
                ReplComponentKind::Product,
                env!("CARGO_PKG_VERSION"),
            ),
            components: Vec::new(),
        }
    }
}

impl ReplHostAvailability {
    pub fn all_available() -> Self {
        Self::default()
    }

    pub fn deny(mut self, requirement: ReplHostRequirement, reason: impl Into<String>) -> Self {
        self.unavailable.insert(requirement, reason.into());
        self
    }

    pub fn with_product(mut self, name: impl Into<String>, version: impl Into<String>) -> Self {
        self.product = ReplComponentVersion::new(name, ReplComponentKind::Product, version);
        self
    }

    pub fn with_component(
        mut self,
        name: impl Into<String>,
        kind: ReplComponentKind,
        version: impl Into<String>,
    ) -> Self {
        self.components
            .push(ReplComponentVersion::new(name, kind, version));
        self
    }

    pub fn product(&self) -> &ReplComponentVersion {
        &self.product
    }

    pub fn versions(&self) -> impl Iterator<Item = &ReplComponentVersion> {
        core::iter::once(&self.product).chain(self.components.iter())
    }

    pub fn unavailable_reason(&self, requirement: ReplHostRequirement) -> Option<&str> {
        self.unavailable.get(&requirement).map(String::as_str)
    }

    pub fn command_unavailable_reason(&self, id: ReplCommandId) -> Option<&str> {
        REPL_COMMAND_SPECS
            .iter()
            .find(|spec| spec.id == id)
            .and_then(|spec| self.unavailable_reason(spec.requirement))
    }
}

#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
#[cfg_attr(
    feature = "serde",
    serde(tag = "kind", content = "data", rename_all = "snake_case")
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplHostRequest {
    Capabilities,
    Documentation { topic: Option<String> },
    ReadSources { resources: Vec<String> },
    WriteSource { resource: String, source: String },
    ListResources { resource: Option<String> },
    ChangeWorkingResource { resource: String },
    Profile { enabled: Option<bool> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplStepMode {
    Synchronous,
    Cooperative,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplDispatchControl {
    Continue,
    Quit,
    PendingStep { count: u64 },
    Host(ReplHostRequest),
}

/// Parse-independent shared dispatcher used after every frontend has produced
/// a typed [`ReplRequest`]. Host-only effects are returned as typed resource
/// requests instead of being performed by the runtime.
pub fn dispatch_repl_request<F: ResidentReplRuntimeFactory>(
    session: &mut ResidentReplSession<F>,
    request: ReplRequest,
    availability: &ReplHostAvailability,
    step_mode: ReplStepMode,
) -> MResult<ReplDispatchControl> {
    match request {
        ReplRequest::SubmitSource { source, .. } => {
            if let Err(error) = session.submit(&source) {
                session.emit_error(&error, DiagnosticPhase::Compile, Some("<repl>"));
            }
            Ok(ReplDispatchControl::Continue)
        }
        ReplRequest::InvokeCommand { command, source } => {
            dispatch_repl_command(session, command, &source, availability, step_mode)
        }
        ReplRequest::Complete { .. } => {
            session.emit_message_diagnostic(
                Severity::Info,
                DiagnosticPhase::Host,
                "CompletionUnavailable",
                "Completion is not available in this host.",
            );
            Ok(ReplDispatchControl::Continue)
        }
        ReplRequest::Interrupt => {
            session.emit_message_diagnostic(
                Severity::Info,
                DiagnosticPhase::Host,
                "Interrupted",
                "Interactive request interrupted.",
            );
            Ok(ReplDispatchControl::Continue)
        }
    }
}

pub fn dispatch_repl_command<F: ResidentReplRuntimeFactory>(
    session: &mut ResidentReplSession<F>,
    command: ReplCommand,
    source_echo: &str,
    availability: &ReplHostAvailability,
    step_mode: ReplStepMode,
) -> MResult<ReplDispatchControl> {
    if let Some(reason) = availability.command_unavailable_reason(command.id()) {
        session.emit_source_echo(source_echo);
        session.emit_message_diagnostic(
            Severity::Error,
            DiagnosticPhase::Host,
            "ReplCommandUnavailable",
            format!(":{} unavailable: {reason}", command.id().as_str()),
        );
        return Ok(ReplDispatchControl::Continue);
    }

    if let ReplCommand::Code(source) = command {
        if let Err(error) = session.submit_with_source_echo(&source, source_echo) {
            session.emit_error(&error, DiagnosticPhase::Compile, Some("<repl>"));
        }
        return Ok(ReplDispatchControl::Continue);
    }

    session.emit_source_echo(source_echo);
    match command {
        ReplCommand::Help => {
            emit_response(
                session,
                ReplResponseKind::Help,
                ReplResponseStatus::Neutral,
                None,
                command_help(availability),
            );
        }
        ReplCommand::Version => {
            emit_response(
                session,
                ReplResponseKind::Command,
                ReplResponseStatus::Neutral,
                None,
                version_inventory(availability),
            );
        }
        ReplCommand::Capabilities => {
            return Ok(ReplDispatchControl::Host(ReplHostRequest::Capabilities));
        }
        ReplCommand::Docs(topic) => {
            return Ok(ReplDispatchControl::Host(ReplHostRequest::Documentation { topic }));
        }
        ReplCommand::Load(resources) => {
            return Ok(ReplDispatchControl::Host(ReplHostRequest::ReadSources { resources }));
        }
        ReplCommand::Save(resource) => {
            return Ok(ReplDispatchControl::Host(ReplHostRequest::WriteSource {
                resource,
                source: session.source().to_string(),
            }));
        }
        ReplCommand::Ls(resource) => {
            return Ok(ReplDispatchControl::Host(ReplHostRequest::ListResources { resource }));
        }
        ReplCommand::Cd(resource) => {
            return Ok(ReplDispatchControl::Host(
                ReplHostRequest::ChangeWorkingResource { resource },
            ));
        }
        ReplCommand::Profile(enabled) => {
            return Ok(ReplDispatchControl::Host(ReplHostRequest::Profile { enabled }));
        }
        ReplCommand::Whos(names) => {
            emit_resident_inspection(session, &names)?;
        }
        ReplCommand::Constraints(names) => {
            emit_integrity_constraint_inspection(session, &names)?;
        }
        ReplCommand::Plan => {
            if let Some(runtime) = session.runtime() {
                let info = runtime.program_execution_info();
                emit_response(
                    session,
                    ReplResponseKind::Command,
                    ReplResponseStatus::Neutral,
                    None,
                    OutputContent::Table(TableOutput::new(
                        vec!["Property".to_string(), "Value".to_string()],
                        vec![
                            vec!["Status".to_string(), "active".to_string()],
                            vec!["Route".to_string(), format!("{:?}", info.route)],
                            vec!["Plan nodes".to_string(), runtime.root_plan_len().to_string()],
                            vec!["Accepted resident turns".to_string(), info.resident_accepted_turns.to_string()],
                            vec!["Accepted source lines".to_string(), session.source().lines().count().to_string()],
                        ],
                    )),
                );
            } else {
                emit_info(session, "No resident program is active.");
            }
        }
        ReplCommand::Outputs => {
            let outputs = session.outputs();
            let content = if outputs.is_empty() {
                OutputContent::Text(TextOutput::new("No output artifacts are active."))
            } else {
                let rows = outputs
                    .into_iter()
                    .map(|artifact| {
                        let source = match &artifact.event.source {
                            OutputSource::Program { span } => span.as_ref().and_then(|span| span.source.clone()).unwrap_or_else(|| "program".to_string()),
                            OutputSource::Host { name, .. } => name.clone(),
                        };
                        vec![
                            artifact.id.to_string(),
                            artifact.event.content.kind_name().to_string(),
                            artifact.event.stream.to_string(),
                            match artifact.status { OutputArtifactStatus::Active => "active", OutputArtifactStatus::Cleared => "cleared" }.to_string(),
                            source,
                        ]
                    })
                    .collect();
                OutputContent::Table(TableOutput::new(
                    vec!["Id".to_string(), "Kind".to_string(), "Stream".to_string(), "Status".to_string(), "Source".to_string()],
                    rows,
                ))
            };
            emit_response(session, ReplResponseKind::Command, ReplResponseStatus::Neutral, None, content);
        }
        ReplCommand::Output(id) => match session.output(&id) {
            Some(artifact) => session.emit(MechEvent::Repl(ReplEvent::FocusDisplay {
                display_id: artifact.id,
                stream: artifact.event.stream,
                content: artifact.event.content,
            })),
            None => session.emit_message_diagnostic(
                Severity::Error,
                DiagnosticPhase::Host,
                "UnknownOutput",
                format!("No output artifact has id `{id}`."),
            ),
        },
        ReplCommand::Step { selector: Some(selector), .. } => session.emit_message_diagnostic(
            Severity::Error,
            DiagnosticPhase::Host,
            "UnsupportedStepSelector",
            format!("Step selector #{selector} is unavailable: the resident runtime only exposes whole-program stepping. Use `:step [count]`."),
        ),
        ReplCommand::Step { selector: None, count } => {
            if step_mode == ReplStepMode::Cooperative {
                return Ok(ReplDispatchControl::PendingStep { count });
            }
            session.step_chunk(count)?;
            emit_step_complete(session, count)?;
        }
        ReplCommand::Clear(names) => {
            match session.clear_variables(&names) {
                Ok(cleared) if cleared.is_empty() => {
                    emit_success(session, "Resident workspace cleared.");
                }
                Ok(cleared) => {
                    emit_success(session, &format!("Cleared {}.", cleared.join(", ")));
                }
                Err(error) => {
                    session.emit_error(&error, DiagnosticPhase::Compile, Some("<repl>"));
                }
            }
        }
        ReplCommand::Clc => session.emit(MechEvent::Repl(ReplEvent::Clear(ReplClearTarget::Interaction))),
        ReplCommand::Quit => {
            emit_success(session, "REPL session terminated.");
            return Ok(ReplDispatchControl::Quit);
        }
        ReplCommand::Code(_) => unreachable!("handled before portable dispatch"),
    }
    Ok(ReplDispatchControl::Continue)
}

pub fn emit_host_response<F: ResidentReplRuntimeFactory>(
    session: &mut ResidentReplSession<F>,
    title: Option<&str>,
    status: ReplResponseStatus,
    content: OutputContent,
) {
    emit_response(session, ReplResponseKind::Command, status, title, content);
}

pub fn emit_step_complete<F: ResidentReplRuntimeFactory>(
    session: &mut ResidentReplSession<F>,
    count: u64,
) -> MResult<()> {
    emit_success(session, &format!("Advanced {count} resident step(s)."));
    emit_resident_inspection(session, &[])
}

fn emit_resident_inspection<F: ResidentReplRuntimeFactory>(
    session: &mut ResidentReplSession<F>,
    names: &[String],
) -> MResult<()> {
    let symbols = session.symbol_inspections(names)?;
    let value_element_limit = session.value_element_limit();
    emit_response(
        session,
        ReplResponseKind::SymbolInspection,
        ReplResponseStatus::Neutral,
        None,
        symbol_values(symbols, value_element_limit),
    );
    Ok(())
}

fn emit_integrity_constraint_inspection<F: ResidentReplRuntimeFactory>(
    session: &mut ResidentReplSession<F>,
    names: &[String],
) -> MResult<()> {
    let constraints = session.integrity_constraints(names)?;
    let value_element_limit = session.value_element_limit();
    emit_response(
        session,
        ReplResponseKind::IntegrityConstraintInspection,
        ReplResponseStatus::Neutral,
        None,
        integrity_constraint_values(constraints, value_element_limit),
    );
    Ok(())
}

fn emit_response<F: ResidentReplRuntimeFactory>(
    session: &mut ResidentReplSession<F>,
    kind: ReplResponseKind,
    status: ReplResponseStatus,
    title: Option<&str>,
    content: OutputContent,
) {
    session.emit(MechEvent::Repl(ReplEvent::Response(ReplResponse::new(
        kind,
        status,
        title.map(str::to_string),
        content,
    ))));
}

fn emit_success<F: ResidentReplRuntimeFactory>(
    session: &mut ResidentReplSession<F>,
    message: &str,
) {
    emit_response(
        session,
        ReplResponseKind::Command,
        ReplResponseStatus::Success,
        None,
        OutputContent::Text(TextOutput::new(message)),
    );
}

fn emit_info<F: ResidentReplRuntimeFactory>(session: &mut ResidentReplSession<F>, message: &str) {
    emit_response(
        session,
        ReplResponseKind::Command,
        ReplResponseStatus::Info,
        None,
        OutputContent::Text(TextOutput::new(message)),
    );
}

fn command_help(availability: &ReplHostAvailability) -> OutputContent {
    OutputContent::Fragments(vec![
        OutputContent::Text(TextOutput::new(REPL_TEXT_LOGO)),
        OutputContent::Text(TextOutput::new(format!(
            "{} v{}\nDocumentation: {MECH_DOCUMENTATION_URL}\nWebsite: {MECH_HOMEPAGE_URL}",
            availability.product().name,
            availability.product().version,
        ))),
        OutputContent::Table(command_help_table(availability)),
        OutputContent::Text(TextOutput::new(format!(
            "{ARGUMENT_QUOTING_HELP}\nEnter submits; Ctrl+Enter inserts a line break. A trailing `;` suppresses automatic value display.",
        ))),
    ])
}

fn version_inventory(availability: &ReplHostAvailability) -> OutputContent {
    OutputContent::Table(TableOutput::new(
        vec![
            "Component".to_string(),
            "Kind".to_string(),
            "Version".to_string(),
        ],
        availability
            .versions()
            .map(|component| {
                vec![
                    component.name.clone(),
                    component.kind.as_str().to_string(),
                    component.version.clone(),
                ]
            })
            .collect(),
    ))
}

fn command_help_table(availability: &ReplHostAvailability) -> TableOutput {
    let muted_rows = REPL_COMMAND_SPECS
        .iter()
        .enumerate()
        .filter_map(|(index, spec)| {
            availability
                .command_unavailable_reason(spec.id)
                .map(|_| index)
        })
        .collect();
    let rows = REPL_COMMAND_SPECS
        .iter()
        .map(|spec| vec![spec.usage.to_string(), spec.description.to_string()])
        .collect();
    TableOutput::new(vec!["Command".to_string(), "Description".to_string()], rows)
        .with_muted_rows(muted_rows)
}

fn symbol_values(
    mut symbols: Vec<ResidentSymbolInspection>,
    value_element_limit: usize,
) -> OutputContent {
    symbols.sort_by(|left, right| left.name.cmp(&right.name));
    if symbols.is_empty() {
        return OutputContent::Text(TextOutput::new("No resident symbols matched."));
    }
    let selection_tokens = symbols
        .iter()
        .map(|symbol| Some(symbol.selection_token.clone()))
        .collect();
    OutputContent::Table(
        TableOutput::new(
            vec!["Name".to_string(), "Type".to_string(), "Value".to_string()],
            symbols
                .into_iter()
                .map(|symbol| {
                    vec![
                        symbol.name,
                        symbol.value.kind().to_string(),
                        symbol.value.format_repl_inline(value_element_limit),
                    ]
                })
                .collect(),
        )
        .with_row_selection_tokens(selection_tokens),
    )
}

fn integrity_constraint_values(
    mut constraints: Vec<(String, crate::RuntimeValueSnapshot)>,
    value_element_limit: usize,
) -> OutputContent {
    constraints.sort_by(|left, right| left.0.cmp(&right.0));
    if constraints.is_empty() {
        return OutputContent::Text(TextOutput::new("No integrity constraints matched."));
    }
    OutputContent::Table(TableOutput::new(
        vec![
            "Constraint".to_string(),
            "Type".to_string(),
            "Value".to_string(),
        ],
        constraints
            .into_iter()
            .map(|(name, value)| {
                vec![
                    name,
                    value.kind().to_string(),
                    value.format_repl_inline(value_element_limit),
                ]
            })
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NeverBuild;

    impl ResidentReplRuntimeFactory for NeverBuild {
        fn build(&self, _events: crate::MechEventBuffer) -> MResult<crate::MechRuntime> {
            unreachable!("focus tests do not activate a resident program")
        }
    }

    #[test]
    fn restricted_hosts_mark_unavailable_commands_from_the_shared_registry() {
        let availability = ReplHostAvailability::all_available().deny(
            ReplHostRequirement::ReadableResources,
            "this host did not provide a readable resource provider",
        );
        let help = command_help_table(&availability);
        let load_index = help
            .rows
            .iter()
            .position(|row| row[0].starts_with(":load"))
            .unwrap();
        let list_index = help
            .rows
            .iter()
            .position(|row| row[0].starts_with(":ls"))
            .unwrap();
        assert_eq!(help.columns, ["Command", "Description"]);
        assert!(help.rows.iter().all(|row| row.len() == 2));
        assert_eq!(help.muted_rows, [load_index, list_index]);
    }

    #[test]
    fn output_focus_carries_the_retained_stream_and_content() {
        let mut session = ResidentReplSession::new(NeverBuild);
        session.emit(MechEvent::Output(crate::OutputEvent {
            source: OutputSource::program(),
            stream: crate::OutputStream::Stderr,
            display_id: Some(crate::DisplayId::new("warning")),
            operation: crate::DisplayOperation::Create,
            content: OutputContent::Text(TextOutput::new("warning text")),
        }));
        session.drain_events().unwrap();

        let control = dispatch_repl_command(
            &mut session,
            ReplCommand::Output("warning".to_string()),
            ":output warning",
            &ReplHostAvailability::all_available(),
            ReplStepMode::Synchronous,
        )
        .unwrap();
        assert_eq!(control, ReplDispatchControl::Continue);
        let events = session.drain_events().unwrap();
        assert!(events.iter().any(|event| matches!(
            &event.event,
            MechEvent::Repl(ReplEvent::FocusDisplay {
                display_id,
                stream: crate::OutputStream::Stderr,
                content: OutputContent::Text(text),
            }) if display_id.as_str() == "warning" && text.text == "warning text"
        )));
    }

    #[test]
    fn symbol_tables_align_opaque_selection_tokens_with_rows() {
        let value = crate::RuntimeValueSnapshot::from_value(
            crate::RuntimeHostInputValue::F64(7.0).into_value().unwrap(),
        )
        .unwrap();
        let content = symbol_values(
            vec![ResidentSymbolInspection {
                name: "x".to_string(),
                value,
                selection_token: "selection:9".to_string(),
            }],
            500,
        );

        let OutputContent::Table(table) = content else {
            panic!("symbol inspection must remain tabular");
        };
        assert_eq!(table.rows[0][0], "x");
        assert_eq!(
            table.row_selection_tokens,
            [Some("selection:9".to_string())],
        );
    }
}
