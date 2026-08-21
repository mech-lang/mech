//! Shared resident session state for interactive hosts.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use mech_core::{GenericError, MResult, MechError};

use crate::{
    DiagnosticEvent, DiagnosticId, DiagnosticNote, DiagnosticPhase, MechEvent, MechEventBus,
    MechEventEnvelope, MechRuntime, OutputArtifact, OutputContent, OutputSource, ReplEvent,
    ReplResponse, ReplResponseKind, ReplResponseStatus, ResidentDurabilityPolicy,
    RuntimeProgramLoadOutcome, RuntimeValueSnapshot, Severity, SourcePosition, SourceSpan,
    ValueOutput,
};

/// Shared upper bound for one synchronous resident-REPL step request.
///
/// Platform hosts may reject this earlier for a better interaction, but every
/// call is checked here before the runtime loop so an adapter cannot block its
/// event loop with an effectively unbounded request.
pub const MAX_RESIDENT_STEP_COUNT: u64 = 1_000_000;

/// Reject an unsafe synchronous resident step request before any host enters
/// its runtime loop.
pub fn validate_resident_step_count(count: u64) -> MResult<()> {
    if count == 0 || count > MAX_RESIDENT_STEP_COUNT {
        return Err(interactive_error(format!(
            "resident step count must be between 1 and {MAX_RESIDENT_STEP_COUNT}"
        )));
    }
    Ok(())
}

/// A cloneable sink used by platform host adapters while a resident program is
/// active. Events are collected transactionally by [`ResidentReplSession`].
#[derive(Clone, Debug, Default)]
pub struct MechEventBuffer {
    events: Arc<Mutex<VecDeque<MechEvent>>>,
}

impl MechEventBuffer {
    pub fn emit(&self, event: MechEvent) -> MResult<()> {
        self.events
            .lock()
            .map_err(|_| interactive_error("program event buffer lock poisoned"))?
            .push_back(event);
        Ok(())
    }

    pub fn drain(&self) -> MResult<Vec<MechEvent>> {
        let mut events = self
            .events
            .lock()
            .map_err(|_| interactive_error("program event buffer lock poisoned"))?;
        Ok(events.drain(..).collect())
    }
}

/// Platform construction boundary for an interactive resident runtime.
pub trait ResidentReplRuntimeFactory {
    fn build(&self, events: MechEventBuffer) -> MResult<MechRuntime>;

    /// Build and activate one complete candidate source.
    ///
    /// Standalone hosts use the default interactive source loader. Document
    /// hosts may override this boundary to retain their source resolver,
    /// configured hosts, and root-program identity while preserving the same
    /// transactional session semantics.
    fn activate(
        &self,
        events: MechEventBuffer,
        source: &str,
    ) -> MResult<(MechRuntime, RuntimeProgramLoadOutcome)> {
        let mut runtime = self.build(events)?;
        let outcome = match runtime
            .load_interactive_source_program(source, ResidentDurabilityPolicy::Volatile)
        {
            Ok(outcome) => outcome,
            Err(error) => {
                let _ = runtime.shutdown();
                return Err(error);
            }
        };
        Ok((runtime, outcome))
    }
}

/// Durable, renderer-neutral REPL state shared by terminal, WASM, and native
/// app hosts.
///
/// Every candidate is compiled and activated in a separate runtime. A failed
/// entry therefore leaves the accepted source and live runtime unchanged.
pub struct ResidentReplSession<F: ResidentReplRuntimeFactory> {
    factory: F,
    initial_source: String,
    source: String,
    runtime: Option<MechRuntime>,
    program_events: Option<MechEventBuffer>,
    events: MechEventJournal,
    quiet: bool,
}

impl<F: ResidentReplRuntimeFactory> ResidentReplSession<F> {
    pub fn new(factory: F) -> Self {
        Self::with_quiet(factory, false)
    }

    pub fn with_quiet(factory: F, quiet: bool) -> Self {
        Self {
            factory,
            initial_source: String::new(),
            source: String::new(),
            runtime: None,
            program_events: None,
            events: MechEventJournal::default(),
            quiet,
        }
    }

    /// Construct a session whose reset point is an already loaded source
    /// document rather than an empty prompt.
    pub fn from_source(factory: F, source: String) -> MResult<Self> {
        let mut session = Self {
            factory,
            initial_source: source.clone(),
            source: String::new(),
            runtime: None,
            program_events: None,
            events: MechEventJournal::default(),
            quiet: false,
        };
        session.replace_source(source)?;
        Ok(session)
    }

    pub fn set_quiet(&mut self, quiet: bool) {
        self.quiet = quiet;
    }

    pub fn is_quiet(&self) -> bool {
        self.quiet
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn runtime(&self) -> Option<&MechRuntime> {
        self.runtime.as_ref()
    }

    pub fn runtime_mut(&mut self) -> Option<&mut MechRuntime> {
        self.runtime.as_mut()
    }

    pub fn submit(&mut self, entry: &str) -> MResult<RuntimeValueSnapshot> {
        self.submit_with_source_echo(entry, entry)
    }

    /// Submit source whose user-visible entry differs from the executable
    /// payload, such as a host `:code` command. The event bus still receives
    /// exactly one causal source echo.
    pub fn submit_with_source_echo(
        &mut self,
        entry: &str,
        source_echo: &str,
    ) -> MResult<RuntimeValueSnapshot> {
        self.emit_source_echo(source_echo);
        self.submit_without_source_echo(entry, true)
    }

    /// Append a host-supplied source document without fabricating a second
    /// user prompt. This is used after a typed host request (for example,
    /// browser documentation loading) already emitted its causal source echo.
    pub fn submit_host_source(&mut self, entry: &str) -> MResult<RuntimeValueSnapshot> {
        self.submit_without_source_echo(entry, false)
    }

    fn submit_without_source_echo(
        &mut self,
        entry: &str,
        emit_value_response: bool,
    ) -> MResult<RuntimeValueSnapshot> {
        let (entry, suppress_value) = executable_submission(entry);
        let mut candidate_source = self.source.clone();
        if !candidate_source.is_empty() && !candidate_source.ends_with('\n') {
            candidate_source.push('\n');
        }
        candidate_source.push_str(&entry);
        if !candidate_source.ends_with('\n') {
            candidate_source.push('\n');
        }
        let value = self.replace_source(candidate_source)?;
        if emit_value_response && !self.quiet && !suppress_value && !value.is_empty() {
            let canonical = value.format_canonical_inline();
            self.emit(MechEvent::Repl(ReplEvent::Response(ReplResponse::new(
                ReplResponseKind::ValueInspection,
                ReplResponseStatus::Neutral,
                None,
                OutputContent::Value(ValueOutput::new(value.kind().to_string(), canonical)),
            ))));
        }
        Ok(value)
    }

    pub fn emit_source_echo(&mut self, source: &str) {
        if !self.quiet {
            self.emit(MechEvent::Repl(ReplEvent::SourceEcho {
                source: source.trim_end_matches(['\r', '\n']).to_string(),
            }));
        }
    }

    pub fn submission_displays_result(&self, source: &str) -> bool {
        !self.quiet && !submission_suppresses_value(source)
    }

    pub fn replace_source(&mut self, candidate_source: String) -> MResult<RuntimeValueSnapshot> {
        let candidate_events = MechEventBuffer::default();
        let (candidate, outcome) = match self
            .factory
            .activate(candidate_events.clone(), &candidate_source)
        {
            Ok(candidate) => candidate,
            Err(error) => {
                return Err(error);
            }
        };

        if let Some(mut previous) = self.runtime.take() {
            previous.shutdown()?;
            self.collect_program_events()?;
        }
        self.runtime = Some(candidate);
        self.program_events = Some(candidate_events);
        self.source = candidate_source;
        Ok(outcome.initial_value)
    }

    pub fn reset(&mut self) -> MResult<()> {
        if !self.initial_source.is_empty() {
            self.replace_source(self.initial_source.clone())?;
            return Ok(());
        }
        if let Some(mut runtime) = self.runtime.take() {
            runtime.shutdown()?;
            self.collect_program_events()?;
        }
        self.program_events = None;
        self.source.clear();
        Ok(())
    }

    pub fn start_input_drivers(&mut self) -> MResult<()> {
        if let Some(runtime) = self.runtime.as_mut() {
            runtime.start_input_drivers()?;
        }
        Ok(())
    }

    pub fn drain_pending_inputs(&mut self, max_inputs: usize) -> MResult<usize> {
        let Some(runtime) = self.runtime.as_mut() else {
            return Ok(0);
        };
        let count = runtime
            .drain_host_inputs(max_inputs)
            .map(|outcomes| outcomes.len())?;
        self.collect_program_events()?;
        Ok(count)
    }

    pub fn drain_all_pending_inputs(&mut self) -> MResult<usize> {
        let Some(runtime) = self.runtime.as_mut() else {
            return Ok(0);
        };
        let pending = runtime.pending_host_input_count()?;
        let count = runtime
            .drain_host_inputs(pending)
            .map(|outcomes| outcomes.len())?;
        self.collect_program_events()?;
        Ok(count)
    }

    pub fn symbols(&mut self, names: &[String]) -> MResult<Vec<(String, RuntimeValueSnapshot)>> {
        if self.runtime.is_none() {
            return Ok(Vec::new());
        }
        if names.is_empty() {
            self.runtime
                .as_ref()
                .expect("resident session was checked above")
                .root_symbol_values_all()
        } else {
            let names = names.iter().map(String::as_str).collect::<Vec<_>>();
            self.runtime
                .as_ref()
                .expect("resident session was checked above")
                .root_symbol_values(&names)
        }
    }

    pub fn step(&mut self, count: u64) -> MResult<Vec<(String, RuntimeValueSnapshot)>> {
        self.step_chunk(count)?;
        self.runtime
            .as_ref()
            .expect("step chunk requires an active resident program")
            .root_symbol_values_all()
    }

    /// Advance one bounded scheduling chunk without performing a full symbol
    /// projection after every browser yield.
    pub fn step_chunk(&mut self, count: u64) -> MResult<()> {
        validate_resident_step_count(count)?;
        let runtime = self
            .runtime
            .as_mut()
            .ok_or_else(|| interactive_error("no resident program is active"))?;
        for _ in 0..count {
            runtime.step_active_program()?;
        }
        self.collect_program_events()?;
        Ok(())
    }

    pub fn emit(&mut self, event: MechEvent) {
        self.events.emit(event);
    }

    pub fn emit_error(
        &mut self,
        error: &MechError,
        phase: DiagnosticPhase,
        source_name: Option<&str>,
    ) {
        self.events.emit_error(error, phase, source_name);
    }

    pub fn emit_message_diagnostic(
        &mut self,
        severity: Severity,
        phase: DiagnosticPhase,
        code: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.events
            .emit_message_diagnostic(severity, phase, code, message);
    }

    pub fn drain_events(&mut self) -> MResult<Vec<MechEventEnvelope>> {
        self.collect_program_events()?;
        Ok(self.events.drain_pending())
    }

    pub fn outputs(&self) -> Vec<OutputArtifact> {
        self.events.outputs()
    }

    pub fn output(&self, id: &str) -> Option<OutputArtifact> {
        self.events.output(id)
    }

    pub fn clear_outputs(&mut self) {
        self.events.clear_outputs();
    }

    pub fn clear_diagnostics(&mut self) {
        self.events.clear_diagnostics();
    }

    pub fn shutdown(&mut self) -> MResult<()> {
        if let Some(mut runtime) = self.runtime.take() {
            runtime.shutdown()?;
            self.collect_program_events()?;
        }
        self.program_events = None;
        Ok(())
    }

    fn collect_program_events(&mut self) -> MResult<()> {
        let Some(events) = self.program_events.as_ref() else {
            return Ok(());
        };
        self.events.absorb(events.drain()?);
        Ok(())
    }
}

fn executable_submission(source: &str) -> (String, bool) {
    let Some(terminal) = mech_syntax::submission_terminal(source) else {
        return (source.to_string(), false);
    };
    if !terminal.suppresses_value {
        return (source.to_string(), false);
    }
    let mut executable = source.to_string();
    executable.remove(terminal.byte_offset);
    (executable, true)
}

fn submission_suppresses_value(source: &str) -> bool {
    mech_syntax::submission_terminal(source).is_some_and(|terminal| terminal.suppresses_value)
}

#[derive(Debug, Default)]
struct MechEventJournal {
    bus: MechEventBus,
    next_diagnostic: u64,
}

impl MechEventJournal {
    fn emit(&mut self, event: MechEvent) {
        self.bus.publish(event);
    }

    fn absorb(&mut self, events: impl IntoIterator<Item = MechEvent>) {
        self.bus.publish_all(events);
    }

    fn drain_pending(&mut self) -> Vec<MechEventEnvelope> {
        self.bus.drain()
    }

    fn outputs(&self) -> Vec<OutputArtifact> {
        self.bus.outputs()
    }

    fn output(&self, id: &str) -> Option<OutputArtifact> {
        self.bus.output(id)
    }

    fn clear_outputs(&mut self) {
        self.bus.clear_outputs(OutputSource::Host {
            name: "repl".to_string(),
            span: None,
        });
    }

    fn clear_diagnostics(&mut self) {
        self.bus.clear_diagnostics();
    }

    fn emit_error(
        &mut self,
        error: &MechError,
        fallback_phase: DiagnosticPhase,
        source_name: Option<&str>,
    ) {
        self.next_diagnostic = self.next_diagnostic.saturating_add(1);
        let phase = classify_error_phase(error, fallback_phase);
        let source = error
            .primary_range()
            .or_else(|| error.tokens.first().map(|token| token.src_range.clone()))
            .map(|range| SourceSpan {
                source: source_name.map(str::to_string),
                start: SourcePosition {
                    line: range.start.row,
                    column: range.start.col,
                },
                end: SourcePosition {
                    line: range.end.row,
                    column: range.end.col,
                },
            });
        let mut notes = Vec::new();
        let mut cause = &error.source;
        while let Some(error) = cause {
            notes.push(DiagnosticNote {
                message: error.simple_message(),
                source: None,
            });
            cause = &error.source;
        }
        self.emit(MechEvent::Diagnostic(DiagnosticEvent {
            id: DiagnosticId::new(format!("diagnostic-{}", self.next_diagnostic)),
            severity: Severity::Error,
            phase,
            code: Some(error.kind_name()),
            message: error.display_message(),
            source,
            notes,
            related: Vec::new(),
        }));
    }

    fn emit_message_diagnostic(
        &mut self,
        severity: Severity,
        phase: DiagnosticPhase,
        code: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.next_diagnostic = self.next_diagnostic.saturating_add(1);
        self.emit(MechEvent::Diagnostic(DiagnosticEvent {
            id: DiagnosticId::new(format!("diagnostic-{}", self.next_diagnostic)),
            severity,
            phase,
            code: Some(code.into()),
            message: message.into(),
            source: None,
            notes: Vec::new(),
            related: Vec::new(),
        }));
    }
}

fn classify_error_phase(error: &MechError, fallback: DiagnosticPhase) -> DiagnosticPhase {
    let name = error.kind_name().to_ascii_lowercase();
    if name.contains("parse") || name.contains("syntax") {
        DiagnosticPhase::Parse
    } else if name.contains("capability") || name.contains("grant") {
        DiagnosticPhase::Capability
    } else {
        fallback
    }
}

fn interactive_error(message: impl Into<String>) -> MechError {
    MechError::new(
        GenericError {
            msg: message.into(),
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NeverBuild;

    impl ResidentReplRuntimeFactory for NeverBuild {
        fn build(&self, _events: MechEventBuffer) -> MResult<MechRuntime> {
            panic!("invalid step counts must be rejected before runtime access")
        }
    }

    #[test]
    fn every_host_is_guarded_by_the_shared_synchronous_step_limit() {
        let mut session = ResidentReplSession::new(NeverBuild);

        for count in [0, MAX_RESIDENT_STEP_COUNT + 1, u64::MAX] {
            let error = session.step(count).unwrap_err();
            assert!(
                error
                    .display_message()
                    .contains("resident step count must be between 1 and 1000000")
            );
        }
    }

    #[test]
    fn submission_terminal_ignores_comments_strings_and_resource_uris() {
        for source in [
            "1 + 1; -- suppressed\n",
            "1 + 1; // suppressed\n",
            "1 + 1;\n-- later comment\n",
        ] {
            let (executable, suppress) = executable_submission(source);
            assert!(suppress, "missing terminal in {source:?}");
            assert!(!executable.contains("1 + 1;"));
        }

        for source in [
            "1 + 1 -- comment ;\n",
            "1 + 1-- comment ;\n",
            "1 + 1// comment ;\n",
            "\"text; -- still text\"\n",
            "@out := console://repl/output{:write(line)}\n",
            "@out := console://repl//output-part{:write(line)}\n",
        ] {
            assert!(
                !submission_suppresses_value(source),
                "false terminal in {source:?}"
            );
        }
    }
}
