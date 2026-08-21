//! Shared resident session state for interactive hosts.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use mech_core::{GenericError, MResult, MechError};

use crate::{
    DiagnosticEvent, DiagnosticId, DiagnosticNote, DiagnosticOwner, DiagnosticPhase, MechEvent,
    MechEventBus, MechEventEnvelope, MechRuntime, OutputArtifact, OutputContent, OutputSource,
    ReplEvent, ReplResponse, ReplResponseKind, ReplResponseStatus, ResidentDurabilityPolicy,
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

    /// Prepare a successfully activated candidate for commit while the
    /// currently accepted runtime is still available for rollback.
    fn prepare_commit(&self, _runtime: &mut MechRuntime) -> MResult<()> {
        Ok(())
    }

    /// Publish factory-owned state associated with the accepted candidate.
    /// Preparation must make this operation infallible.
    fn commit(&self) {}

    /// Discard factory-owned state associated with a rejected candidate.
    fn abort(&self) {}
}

/// Durable, renderer-neutral REPL state shared by terminal, WASM, and native
/// app hosts.
///
/// Every candidate is compiled and activated in a separate runtime. A failed
/// entry therefore leaves the accepted source and live runtime unchanged.
pub struct ResidentReplSession<F: ResidentReplRuntimeFactory> {
    factory: F,
    initial_source: Option<String>,
    source: String,
    runtime: Option<MechRuntime>,
    program_events: Option<MechEventBuffer>,
    pending_selection: Option<RuntimeValueSnapshot>,
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
            initial_source: None,
            source: String::new(),
            runtime: None,
            program_events: None,
            pending_selection: None,
            events: MechEventJournal::default(),
            quiet,
        }
    }

    /// Construct a session whose reset point is an already loaded source
    /// document rather than an empty prompt.
    pub fn from_source(factory: F, source: String) -> MResult<Self> {
        let mut session = Self {
            factory,
            initial_source: Some(source.clone()),
            source: String::new(),
            runtime: None,
            program_events: None,
            pending_selection: None,
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

    /// Inspect an already resident value without recompiling the active
    /// document. The canonical expression is folded into the next ordinary
    /// submission so subsequent source can consume the selected `ans`.
    pub fn select_value(
        &mut self,
        source_echo: &str,
        value: RuntimeValueSnapshot,
    ) -> Option<ValueOutput> {
        self.emit_source_echo(source_echo);
        let visible_value = if !self.quiet && !value.is_empty() {
            Some(ValueOutput::new(
                value.kind().to_string(),
                value.format_canonical_inline(),
            ))
        } else {
            None
        };
        self.pending_selection = Some(value);
        if let Some(value) = &visible_value {
            self.emit(MechEvent::Repl(ReplEvent::Response(ReplResponse::new(
                ReplResponseKind::ValueInspection,
                ReplResponseStatus::Neutral,
                None,
                OutputContent::Value(value.clone()),
            ))));
        }
        visible_value
    }

    fn submit_without_source_echo(
        &mut self,
        entry: &str,
        emit_value_response: bool,
    ) -> MResult<RuntimeValueSnapshot> {
        let (entry, suppress_value) = executable_submission(entry);
        let mut candidate_source = self.source.clone();
        if let Some(selection) = &self.pending_selection {
            if !candidate_source.is_empty() && !candidate_source.ends_with('\n') {
                candidate_source.push('\n');
            }
            candidate_source.push_str(&selection.format_canonical_inline());
            if !candidate_source.ends_with('\n') {
                candidate_source.push('\n');
            }
        }
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
        let (mut candidate, outcome) = match self
            .factory
            .activate(candidate_events.clone(), &candidate_source)
        {
            Ok(candidate) => candidate,
            Err(error) => {
                return Err(error);
            }
        };

        if let Err(error) = self.factory.prepare_commit(&mut candidate) {
            let _ = candidate.shutdown();
            self.factory.abort();
            return Err(error);
        }

        // Shutdown is the irreversible handoff boundary: closing ingress and
        // stopping drivers mutate the retired runtime even when cleanup later
        // reports an error. The prepared candidate must therefore commit once
        // shutdown begins; cleanup failures are surfaced as host warnings and
        // never resurrect a partially stopped runtime.
        let mut retirement_failures = Vec::new();
        if let Some(mut previous) = self.runtime.take() {
            if let Err(error) = previous.shutdown() {
                retirement_failures.push(("PreviousRuntimeShutdown", error));
            }
            if let Err(error) = self.collect_program_events() {
                retirement_failures.push(("PreviousRuntimeEvents", error));
            }
        }
        self.factory.commit();
        self.runtime = Some(candidate);
        self.program_events = Some(candidate_events);
        self.source = candidate_source;
        self.pending_selection = None;
        for (code, error) in retirement_failures {
            self.emit_message_diagnostic(
                Severity::Warning,
                DiagnosticPhase::Host,
                code,
                format!(
                    "The replacement runtime was accepted, but retired runtime cleanup reported: {}",
                    error.display_message(),
                ),
            );
        }
        Ok(outcome.initial_value)
    }

    pub fn reset(&mut self) -> MResult<()> {
        if let Some(initial_source) = self.initial_source.clone() {
            self.replace_source(initial_source)?;
            return Ok(());
        }
        if let Some(mut runtime) = self.runtime.take() {
            runtime.shutdown()?;
            self.collect_program_events()?;
        }
        self.program_events = None;
        self.source.clear();
        self.pending_selection = None;
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

    pub fn symbol(&self, name: &str) -> MResult<Option<RuntimeValueSnapshot>> {
        if name == "ans"
            && let Some(value) = &self.pending_selection
        {
            return Ok(Some(value.clone()));
        }
        let Some(runtime) = self.runtime.as_ref() else {
            return Ok(None);
        };
        runtime.root_symbol_value(name).map(Some)
    }

    pub fn symbols(&self, names: &[String]) -> MResult<Vec<(String, RuntimeValueSnapshot)>> {
        if self.runtime.is_none() {
            return Ok(Vec::new());
        }
        let requested_names = if self.pending_selection.is_some() && !names.is_empty() {
            names
                .iter()
                .filter(|name| name.as_str() != "ans")
                .map(String::as_str)
                .collect::<Vec<_>>()
        } else {
            names.iter().map(String::as_str).collect::<Vec<_>>()
        };
        let mut values = if names.is_empty() {
            self.runtime
                .as_ref()
                .expect("resident session was checked above")
                .root_symbol_values_all()
        } else if requested_names.is_empty() {
            Ok(Vec::new())
        } else {
            self.runtime
                .as_ref()
                .expect("resident session was checked above")
                .root_symbol_values(&requested_names)
        }?;
        if let Some(selected) = &self.pending_selection
            && (names.is_empty() || names.iter().any(|name| name == "ans"))
        {
            if let Some((_, value)) = values.iter_mut().find(|(name, _)| name == "ans") {
                *value = selected.clone();
            } else {
                values.push(("ans".to_string(), selected.clone()));
                values.sort_by(|left, right| left.0.cmp(&right.0));
            }
        }
        Ok(values)
    }

    pub fn integrity_constraints(
        &self,
        names: &[String],
    ) -> MResult<Vec<(String, RuntimeValueSnapshot)>> {
        let Some(runtime) = self.runtime.as_ref() else {
            return Ok(Vec::new());
        };
        runtime
            .root_integrity_constraint_values(&names.iter().map(String::as_str).collect::<Vec<_>>())
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

    /// Publish an event produced by the active program into the same bounded
    /// stream used by runtime host adapters. Program producers may publish
    /// output, diagnostics, and telemetry; REPL control events remain owned by
    /// the interactive session itself.
    pub fn publish_program_event(&self, event: MechEvent) -> MResult<()> {
        if matches!(event, MechEvent::Repl(_)) {
            return Err(interactive_error(
                "program producers cannot publish REPL control events",
            ));
        }
        let events = self
            .program_events
            .as_ref()
            .ok_or_else(|| interactive_error("no resident program event stream is active"))?;
        events.emit(event)
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
        self.pending_selection = None;
        Ok(())
    }

    fn collect_program_events(&mut self) -> MResult<()> {
        let Some(events) = self.program_events.as_ref() else {
            return Ok(());
        };
        self.events
            .absorb(events.drain()?.into_iter().map(own_program_diagnostic));
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
            owner: DiagnosticOwner::Interaction,
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
            owner: DiagnosticOwner::Interaction,
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

fn own_program_diagnostic(event: MechEvent) -> MechEvent {
    match event {
        MechEvent::Diagnostic(mut diagnostic) => {
            diagnostic.owner = DiagnosticOwner::Program;
            MechEvent::Diagnostic(diagnostic)
        }
        event => event,
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
    use std::cell::Cell;

    struct NeverBuild;

    impl ResidentReplRuntimeFactory for NeverBuild {
        fn build(&self, _events: MechEventBuffer) -> MResult<MechRuntime> {
            panic!("invalid step counts must be rejected before runtime access")
        }
    }

    #[derive(Debug)]
    struct FailingStopDriver;

    impl crate::RuntimeHostInputDriver for FailingStopDriver {
        fn drives(&self, _source: &crate::RuntimeHostInputSource) -> bool {
            false
        }

        fn attach(&mut self, _ingress: crate::RuntimeIngress) -> MResult<()> {
            Ok(())
        }

        fn start(&mut self) -> MResult<()> {
            Ok(())
        }

        fn stop(&mut self) -> MResult<()> {
            Err(interactive_error("deliberate retired runtime stop failure"))
        }

        fn is_live(&self) -> bool {
            false
        }
    }

    struct FailingRetirementFactory {
        activations: Cell<usize>,
    }

    struct CapturingProgramEventFactory {
        events: Arc<Mutex<Option<MechEventBuffer>>>,
    }

    impl ResidentReplRuntimeFactory for CapturingProgramEventFactory {
        fn build(&self, _events: MechEventBuffer) -> MResult<MechRuntime> {
            unreachable!("the test factory supplies an activated runtime")
        }

        fn activate(
            &self,
            events: MechEventBuffer,
            _source: &str,
        ) -> MResult<(MechRuntime, RuntimeProgramLoadOutcome)> {
            *self.events.lock().unwrap() = Some(events);
            Ok((
                MechRuntime::builder().build()?,
                RuntimeProgramLoadOutcome {
                    route: crate::RuntimeProgramRoute::None,
                    initial_value: RuntimeValueSnapshot::empty(),
                    info: crate::RuntimeProgramExecutionInfo::default(),
                },
            ))
        }
    }

    impl ResidentReplRuntimeFactory for FailingRetirementFactory {
        fn build(&self, _events: MechEventBuffer) -> MResult<MechRuntime> {
            unreachable!("the test factory supplies activated runtimes directly")
        }

        fn activate(
            &self,
            _events: MechEventBuffer,
            _source: &str,
        ) -> MResult<(MechRuntime, RuntimeProgramLoadOutcome)> {
            let activation = self.activations.get();
            self.activations.set(activation + 1);
            let builder = MechRuntime::builder();
            let runtime = if activation == 0 {
                builder.test_input_driver(FailingStopDriver).build()?
            } else {
                builder.build()?
            };
            Ok((
                runtime,
                RuntimeProgramLoadOutcome {
                    route: crate::RuntimeProgramRoute::None,
                    initial_value: RuntimeValueSnapshot::empty(),
                    info: crate::RuntimeProgramExecutionInfo::default(),
                },
            ))
        }
    }

    #[test]
    fn replacement_commits_after_retired_runtime_shutdown_has_begun() {
        let factory = FailingRetirementFactory {
            activations: Cell::new(0),
        };
        let mut session =
            ResidentReplSession::from_source(factory, "baseline".to_string()).unwrap();

        session.replace_source("replacement".to_string()).unwrap();

        assert_eq!(session.source(), "replacement");
        assert!(
            !session
                .runtime()
                .expect("the prepared candidate must become active")
                .ingress()
                .is_closed()
                .unwrap(),
            "the session must not restore the retired runtime with closed ingress",
        );
        let events = session.drain_events().unwrap();
        assert!(
            events
                .iter()
                .any(|event| format!("{event:?}")
                    .contains("deliberate retired runtime stop failure")),
            "retirement failure must remain observable as a host warning",
        );
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

    #[test]
    fn diagnostic_ownership_is_assigned_at_the_producer_boundary() {
        let mut journal = MechEventJournal::default();
        journal.emit_message_diagnostic(
            Severity::Error,
            DiagnosticPhase::Host,
            "ReplCommand",
            "bad command",
        );
        let interactive = journal.drain_pending();
        assert!(matches!(
            &interactive[0].event,
            MechEvent::Diagnostic(diagnostic)
                if diagnostic.owner == DiagnosticOwner::Interaction
        ));

        let captured = Arc::new(Mutex::new(None));
        let mut session = ResidentReplSession::from_source(
            CapturingProgramEventFactory {
                events: Arc::clone(&captured),
            },
            "baseline".to_string(),
        )
        .unwrap();
        session
            .publish_program_event(MechEvent::Diagnostic(DiagnosticEvent {
                id: DiagnosticId::new("program-diagnostic"),
                owner: DiagnosticOwner::Interaction,
                severity: Severity::Error,
                phase: DiagnosticPhase::Execute,
                code: None,
                message: "program failed".to_string(),
                source: None,
                notes: Vec::new(),
                related: Vec::new(),
            }))
            .unwrap();
        let program = session.drain_events().unwrap();
        assert!(matches!(
            &program[0].event,
            MechEvent::Diagnostic(diagnostic) if diagnostic.owner == DiagnosticOwner::Program
        ));
        assert!(
            session
                .publish_program_event(MechEvent::Repl(ReplEvent::Clear(
                    crate::ReplClearTarget::Interaction,
                )))
                .is_err(),
            "program producers must not impersonate the session control protocol",
        );
    }
}
