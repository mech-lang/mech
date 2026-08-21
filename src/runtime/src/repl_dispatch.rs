//! Renderer-neutral command dispatch and typed host-resource requests.

use std::collections::BTreeMap;

use mech_core::MResult;

use crate::{
    ARGUMENT_QUOTING_HELP, ClearTarget, DiagnosticPhase, MechEvent, OutputArtifactStatus,
    OutputContent, OutputSource, REPL_COMMAND_SPECS, ReplClearTarget, ReplCommand, ReplCommandId,
    ReplEvent, ReplHostRequirement, ReplRequest, ReplResponse, ReplResponseKind,
    ReplResponseStatus, ResidentReplRuntimeFactory, ResidentReplSession, Severity, TableOutput,
    TextOutput,
};

#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReplHostAvailability {
    unavailable: BTreeMap<ReplHostRequirement, String>,
}

impl ReplHostAvailability {
    pub fn all_available() -> Self {
        Self::default()
    }

    pub fn deny(mut self, requirement: ReplHostRequirement, reason: impl Into<String>) -> Self {
        self.unavailable.insert(requirement, reason.into());
        self
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
                Some("REPL commands"),
                command_help(availability),
            );
            emit_info(
                session,
                &format!(
                    "{ARGUMENT_QUOTING_HELP}\nEnter submits; Ctrl+Enter inserts a line break. A trailing `;` suppresses automatic value display.",
                ),
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
            let symbols = session.symbols(&names)?;
            emit_response(
                session,
                ReplResponseKind::SymbolInspection,
                ReplResponseStatus::Neutral,
                Some("Resident values"),
                symbol_values(symbols),
            );
        }
        ReplCommand::Plan => {
            if let Some(runtime) = session.runtime() {
                let info = runtime.program_execution_info();
                emit_response(
                    session,
                    ReplResponseKind::Command,
                    ReplResponseStatus::Neutral,
                    Some("Resident execution plan"),
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
            emit_response(session, ReplResponseKind::Command, ReplResponseStatus::Neutral, Some("Session outputs"), content);
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
        ReplCommand::Clear(ClearTarget::Session) => {
            session.reset()?;
            emit_success(session, "Resident REPL state cleared.");
        }
        ReplCommand::Clear(ClearTarget::Output) => {
            session.clear_outputs();
            emit_success(session, "Output history cleared.");
        }
        ReplCommand::Clear(ClearTarget::Diagnostics) => session.clear_diagnostics(),
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
    let symbols = session.symbols(&[])?;
    emit_response(
        session,
        ReplResponseKind::SymbolInspection,
        ReplResponseStatus::Neutral,
        Some("Resident values"),
        symbol_values(symbols),
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
    OutputContent::Table(
        TableOutput::new(vec!["Command".to_string(), "Description".to_string()], rows)
            .with_muted_rows(muted_rows),
    )
}

fn symbol_values(mut symbols: Vec<(String, crate::RuntimeValueSnapshot)>) -> OutputContent {
    const VALUE_PREVIEW_LIMIT: usize = 96;

    symbols.sort_by(|left, right| left.0.cmp(&right.0));
    if symbols.is_empty() {
        return OutputContent::Text(TextOutput::new("No resident symbols matched."));
    }
    OutputContent::Table(TableOutput::new(
        vec!["Name".to_string(), "Type".to_string(), "Value".to_string()],
        symbols
            .into_iter()
            .map(|(name, value)| {
                vec![
                    name,
                    value.kind().to_string(),
                    value.to_value().format_preview_inline(VALUE_PREVIEW_LIMIT),
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
        let OutputContent::Table(help) = command_help(&availability) else {
            panic!("help table")
        };
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
}
