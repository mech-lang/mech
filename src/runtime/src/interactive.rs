//! Shared resident session state for interactive hosts.

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use mech_core::{GenericError, MResult, MechError};
use mech_syntax::document::{
    AstNode, DocumentStream, OpAssignSyntax, ParseConfig, Revision, TupleDestructureSyntax,
    VariableAssignSyntax, VariableDefineSyntax,
};

use crate::{
    DiagnosticEvent, DiagnosticId, DiagnosticNote, DiagnosticOwner, DiagnosticPhase,
    MAX_RESIDENT_STEP_COUNT, MechEvent, MechEventBus, MechEventEnvelope, MechRuntime,
    OutputArtifact, OutputContent, OutputSource, ReplEvent, ReplResponse, ReplResponseKind,
    ReplResponseStatus, ResidentDurabilityPolicy, RuntimeProgramLoadOutcome, RuntimeValueSnapshot,
    Severity, SourcePosition, SourceSpan, ValueOutput,
};

static NEXT_SELECTION_TOKEN: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
struct PendingSelection {
    value: RuntimeValueSnapshot,
    identity: Option<String>,
}

#[derive(Clone)]
struct RetainedSelection {
    source_echo: String,
    value: RuntimeValueSnapshot,
}

pub struct ResidentSymbolInspection {
    pub name: String,
    pub value: RuntimeValueSnapshot,
    pub selection_token: String,
}

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
        if source.trim().is_empty() {
            return Ok((
                runtime,
                RuntimeProgramLoadOutcome {
                    route: crate::RuntimeProgramRoute::None,
                    initial_value: RuntimeValueSnapshot::empty(),
                    info: crate::RuntimeProgramExecutionInfo::default(),
                },
            ));
        }
        let outcome = match runtime
            .load_interactive_source_program(source, ResidentDurabilityPolicy::Volatile)
        {
            Ok(outcome) => outcome,
            Err(error) => {
                if let Err(shutdown_error) = runtime.shutdown() {
                    return Err(shutdown_error.with_source(error));
                }
                return Err(error);
            }
        };
        Ok((runtime, outcome))
    }

    /// Build and activate one strictly admitted retained canonical revision.
    /// Hosts preparing the S8 cutover override this instead of reconstructing
    /// an executable tree from the source projection.
    fn activate_document(
        &self,
        events: MechEventBuffer,
        document: &crate::SourceDocument,
    ) -> MResult<(MechRuntime, RuntimeProgramLoadOutcome)> {
        self.activate(events, &document.source().to_contiguous_string())
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
pub const DEFAULT_REPL_VALUE_ELEMENT_LIMIT: usize = 500;

pub struct ResidentReplSession<F: ResidentReplRuntimeFactory> {
    factory: F,
    initial_document: Option<crate::SourceDocument>,
    source: String,
    source_document: Option<crate::SourceDocument>,
    runtime: Option<MechRuntime>,
    program_events: Option<MechEventBuffer>,
    pending_selection: Option<PendingSelection>,
    cleared_synthetic_symbols: std::collections::BTreeSet<String>,
    retained_selections: BTreeMap<String, RetainedSelection>,
    reusable_selection_tokens: BTreeMap<String, String>,
    events: MechEventJournal,
    quiet: bool,
    value_element_limit: usize,
}

impl<F: ResidentReplRuntimeFactory> ResidentReplSession<F> {
    pub fn new(factory: F) -> Self {
        Self::with_quiet(factory, false)
    }

    pub fn with_quiet(factory: F, quiet: bool) -> Self {
        Self {
            factory,
            initial_document: None,
            source: String::new(),
            source_document: None,
            runtime: None,
            program_events: None,
            pending_selection: None,
            cleared_synthetic_symbols: std::collections::BTreeSet::new(),
            retained_selections: BTreeMap::new(),
            reusable_selection_tokens: BTreeMap::new(),
            events: MechEventJournal::default(),
            quiet,
            value_element_limit: DEFAULT_REPL_VALUE_ELEMENT_LIMIT,
        }
    }

    /// Construct a session whose reset point is an already loaded source
    /// document rather than an empty prompt.
    pub fn from_source(factory: F, source: String) -> MResult<Self> {
        let document = crate::SourceDocument::parse_resolved(
            "runtime:interactive",
            Revision(0),
            Arc::<str>::from(source),
            ParseConfig::default(),
        )
        .map_err(|error| interactive_error(format!("invalid interactive source: {error:?}")))?;
        Self::from_document(factory, document)
    }

    /// Construct a canonical interactive session around one retained source
    /// revision. No legacy syntax tree is created or retained.
    pub fn from_document(factory: F, document: crate::SourceDocument) -> MResult<Self> {
        let mut session = Self {
            factory,
            initial_document: Some(document.clone()),
            source: String::new(),
            source_document: None,
            runtime: None,
            program_events: None,
            pending_selection: None,
            cleared_synthetic_symbols: std::collections::BTreeSet::new(),
            retained_selections: BTreeMap::new(),
            reusable_selection_tokens: BTreeMap::new(),
            events: MechEventJournal::default(),
            quiet: false,
            value_element_limit: DEFAULT_REPL_VALUE_ELEMENT_LIMIT,
        };
        session.replace_document(document)?;
        Ok(session)
    }

    pub fn set_quiet(&mut self, quiet: bool) {
        self.quiet = quiet;
    }

    pub fn is_quiet(&self) -> bool {
        self.quiet
    }

    pub fn set_value_element_limit(&mut self, max_elements: usize) {
        self.value_element_limit = max_elements.max(1);
    }

    pub fn value_element_limit(&self) -> usize {
        self.value_element_limit
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn source_document(&self) -> Option<&crate::SourceDocument> {
        self.source_document.as_ref()
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

    /// Submit only after S7B has finalized the complete entry stream. Open,
    /// limited, and cancelled streams fail before candidate construction and
    /// therefore cannot execute or replace the accepted runtime.
    pub fn submit_finished_stream(
        &mut self,
        stream: &mut DocumentStream,
    ) -> MResult<RuntimeValueSnapshot> {
        let entry = crate::SourceDocument::from_finished_stream(stream).map_err(|error| {
            interactive_error(format!("interactive source is not final: {error:?}"))
        })?;
        let entry_source = entry.source().to_contiguous_string();
        self.emit_source_echo(&entry_source);
        self.submit_prepared_entry(&entry_source, Some(entry), true)
    }

    /// Replace the accepted program with a finalized streamed document.
    pub fn replace_finished_stream(
        &mut self,
        stream: &mut DocumentStream,
    ) -> MResult<RuntimeValueSnapshot> {
        let document = crate::SourceDocument::from_finished_stream(stream).map_err(|error| {
            interactive_error(format!("interactive source is not final: {error:?}"))
        })?;
        self.replace_document(self.preserve_document_provenance(document))
    }

    /// Inspect an already resident value without recompiling the active
    /// document. The canonical expression is folded into the next ordinary
    /// submission so subsequent source can consume the selected `ans`.
    pub fn select_value(
        &mut self,
        source_echo: &str,
        value: RuntimeValueSnapshot,
    ) -> Option<ValueOutput> {
        self.select_value_with_identity(source_echo, value, None)
    }

    pub fn select_value_with_identity(
        &mut self,
        source_echo: &str,
        value: RuntimeValueSnapshot,
        identity: Option<String>,
    ) -> Option<ValueOutput> {
        self.emit_source_echo(source_echo);
        let visible_value = if !self.quiet && !value.is_empty() {
            Some(ValueOutput::new(
                value.kind().to_string(),
                value.format_repl_inline(self.value_element_limit),
            ))
        } else {
            None
        };
        self.pending_selection = Some(PendingSelection { value, identity });
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
        self.submit_prepared_entry(entry, None, emit_value_response)
    }

    /// One preparation and commit path for typed and finalized-stream entries.
    /// Selection is consumed only by a successfully accepted candidate.
    fn submit_prepared_entry(
        &mut self,
        entry: &str,
        finalized: Option<crate::SourceDocument>,
        emit_value_response: bool,
    ) -> MResult<RuntimeValueSnapshot> {
        if finalized
            .as_ref()
            .is_some_and(|document| !document.is_strictly_clean())
        {
            return Err(interactive_error(
                "interactive canonical document contains syntax diagnostics",
            ));
        }
        let (entry, suppress_value) = executable_submission(entry);
        let mut appended_source = String::new();
        if let Some(selection) = &self.pending_selection {
            appended_source.push_str(&selection.value.format_canonical_inline());
            appended_source.push('\n');
        }
        appended_source.push_str(&entry);
        if !appended_source.ends_with('\n') {
            appended_source.push('\n');
        }

        let mut candidate_source = self.source.clone();
        if !candidate_source.is_empty() && !candidate_source.ends_with('\n') {
            candidate_source.push('\n');
        }
        candidate_source.push_str(&appended_source);
        let parse = |source: &str| {
            crate::SourceDocument::parse_resolved(
                "runtime:interactive",
                Revision(self.source_revision().saturating_add(1)),
                Arc::<str>::from(source),
                ParseConfig::default(),
            )
            .map_err(|error| interactive_error(format!("invalid interactive source: {error:?}")))
            .map(|document| self.preserve_document_provenance(document))
        };
        let overlay = match finalized {
            Some(document) if document.source().to_contiguous_string() == appended_source => {
                self.preserve_document_provenance(document)
            }
            _ => parse(&appended_source)?,
        };
        let changed_state_names = mech_engine::CanonicalSourceFrontend
            .root_state_mutation_names(&overlay.document())
            .map_err(|error| interactive_error(error.to_string()))?;
        let candidate = if self.source.is_empty() {
            overlay
        } else {
            parse(&candidate_source)?
        };
        let value = self.replace_document_preserving(candidate, &changed_state_names)?;
        if emit_value_response && !self.quiet && !suppress_value && !value.is_empty() {
            let canonical = value.format_repl_inline(self.value_element_limit);
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
        self.replace_source_preserving(candidate_source, &std::collections::BTreeSet::new())
    }

    pub fn replace_document(
        &mut self,
        document: crate::SourceDocument,
    ) -> MResult<RuntimeValueSnapshot> {
        self.replace_document_preserving(document, &std::collections::BTreeSet::new())
    }

    fn preserve_document_provenance(
        &self,
        mut document: crate::SourceDocument,
    ) -> crate::SourceDocument {
        if let Some(current) = self.source_document.as_ref() {
            if let Some(origin) = current.nominal_origin() {
                document = document.with_nominal_origin(origin.clone());
            }
            if let Some(package_id) = current.nominal_package_id() {
                document = document.with_nominal_package_id(package_id);
            }
        }
        document
    }

    fn replace_document_preserving(
        &mut self,
        document: crate::SourceDocument,
        changed_state_names: &std::collections::BTreeSet<String>,
    ) -> MResult<RuntimeValueSnapshot> {
        if !document.is_strictly_clean() {
            return Err(interactive_error(
                "interactive canonical document contains syntax diagnostics",
            ));
        }
        let source = document.source().to_contiguous_string();
        self.replace_source_candidate(source, document, Some(changed_state_names))
    }

    fn source_revision(&self) -> u64 {
        self.source_document
            .as_ref()
            .map(|document| document.source().revision().0)
            .unwrap_or(0)
    }

    /// Rebuild the currently accepted program through the normal candidate
    /// handoff while preserving compatible resident state.
    ///
    /// Hosts use this when an execution backend becomes unavailable after the
    /// source generation was accepted. Rebuilding the retained document keeps
    /// the replacement on the same source/state generation.
    pub fn rebuild_runtime_preserving_state(&mut self) -> MResult<RuntimeValueSnapshot> {
        let unchanged = std::collections::BTreeSet::new();
        if let Some(document) = self.source_document.clone() {
            return self.replace_document_preserving(document, &unchanged);
        }
        self.replace_source_preserving(self.source.clone(), &unchanged)
    }

    fn replace_source_preserving(
        &mut self,
        candidate_source: String,
        changed_state_names: &std::collections::BTreeSet<String>,
    ) -> MResult<RuntimeValueSnapshot> {
        let document = crate::SourceDocument::parse_resolved(
            "runtime:interactive",
            Revision(self.source_revision().saturating_add(1)),
            Arc::<str>::from(candidate_source),
            ParseConfig::default(),
        )
        .map_err(|error| interactive_error(format!("invalid interactive source: {error:?}")))?;
        self.replace_document_preserving(document, changed_state_names)
    }

    fn replace_source_candidate(
        &mut self,
        candidate_source: String,
        candidate_document: crate::SourceDocument,
        changed_state_names: Option<&std::collections::BTreeSet<String>>,
    ) -> MResult<RuntimeValueSnapshot> {
        let candidate_events = MechEventBuffer::default();
        let activated = self
            .factory
            .activate_document(candidate_events.clone(), &candidate_document);
        let (mut candidate, outcome) = match activated {
            Ok(candidate) => candidate,
            Err(error) => {
                return Err(error);
            }
        };

        if let (Some(previous), Some(changed_state_names)) =
            (self.runtime.as_ref(), changed_state_names)
        {
            match candidate.preserve_compatible_resident_state_from(previous, changed_state_names) {
                Ok(()) => {}
                Err(error) => {
                    if let Err(shutdown_error) = candidate.shutdown() {
                        self.factory.abort();
                        return Err(shutdown_error.with_source(error));
                    }
                    self.factory.abort();
                    return Err(error);
                }
            }
        }

        let accepted_value = match candidate.program_output_value() {
            Ok(Some(value)) => value,
            Ok(None) => outcome.initial_value,
            Err(error) => {
                if let Err(shutdown_error) = candidate.shutdown() {
                    self.factory.abort();
                    return Err(shutdown_error.with_source(error));
                }
                self.factory.abort();
                return Err(error);
            }
        };

        if let Err(error) = self.factory.prepare_commit(&mut candidate) {
            if let Err(shutdown_error) = candidate.shutdown() {
                self.factory.abort();
                return Err(shutdown_error.with_source(error));
            }
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
        self.source_document = Some(candidate_document);
        self.pending_selection = None;
        self.cleared_synthetic_symbols.clear();
        self.reusable_selection_tokens.clear();
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
        Ok(accepted_value)
    }

    /// Remove resident variables by rebuilding the complete accepted source.
    ///
    /// The candidate runtime is activated before the current runtime is
    /// retired, so a dependency or activation failure leaves the workspace
    /// unchanged. With no names, the complete resident workspace is removed.
    pub fn clear_variables(&mut self, names: &[String]) -> MResult<Vec<String>> {
        if names.is_empty() {
            let document = crate::SourceDocument::parse_resolved(
                "runtime:interactive",
                Revision(self.source_revision().saturating_add(1)),
                Arc::<str>::from(""),
                ParseConfig::default(),
            )
            .map_err(|error| interactive_error(format!("invalid empty source: {error:?}")))?;
            self.replace_document(self.preserve_document_provenance(document))?;
            return Ok(Vec::new());
        }

        let mut requested = names
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        // `ans` is the runtime's synthetic projection of the current root
        // result (or a pending clicked selection), not a syntax definition.
        // Keep it clearable like every other name exposed by `:whos` without
        // deleting the source statement that may also define an ordinary
        // resident variable. A later accepted submission creates a new root
        // result and makes `ans` available again.
        let requested_ans = requested.remove("ans");
        let clear_ans = requested_ans
            && (self.pending_selection.is_some()
                || self
                    .runtime
                    .as_ref()
                    .and_then(|runtime| runtime.root_symbol_output_id("ans"))
                    .is_some());
        if requested_ans && !clear_ans {
            return Err(interactive_error("resident variable `ans` not found"));
        }

        if requested.is_empty() {
            if !clear_ans {
                return Err(interactive_error("resident variable `ans` not found"));
            }
            self.pending_selection = None;
            self.cleared_synthetic_symbols.insert("ans".to_string());
            return Ok(vec!["ans".to_string()]);
        }
        let Some(document) = self.source_document.clone() else {
            return Err(missing_variable_error(
                &requested.into_iter().collect::<Vec<_>>(),
            ));
        };
        let (candidate_source, mut removed) = remove_canonical_definitions(&document, &requested)?;
        let missing = requested.difference(&removed).cloned().collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(missing_variable_error(&missing));
        }
        let candidate = crate::SourceDocument::parse_resolved(
            "runtime:interactive",
            Revision(self.source_revision().saturating_add(1)),
            Arc::<str>::from(candidate_source),
            ParseConfig::default(),
        )
        .map_err(|error| interactive_error(format!("invalid cleared source: {error:?}")))?;
        self.replace_document(self.preserve_document_provenance(candidate))?;
        if clear_ans {
            self.cleared_synthetic_symbols.insert("ans".to_string());
            removed.insert("ans".to_string());
        }
        Ok(removed.into_iter().collect())
    }

    pub fn reset(&mut self) -> MResult<()> {
        if let Some(initial_document) = self.initial_document.clone() {
            self.replace_source_candidate(
                initial_document.source().to_contiguous_string(),
                initial_document,
                None,
            )?;
            return Ok(());
        }
        let document = crate::SourceDocument::parse_resolved(
            "runtime:interactive",
            Revision(self.source_revision().saturating_add(1)),
            Arc::<str>::from(""),
            ParseConfig::default(),
        )
        .map_err(|error| interactive_error(format!("invalid empty source: {error:?}")))?;
        self.replace_source_candidate(String::new(), document, None)?;
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
        if self.cleared_synthetic_symbols.contains(name) {
            return Ok(None);
        }
        if name == "ans"
            && let Some(value) = &self.pending_selection
        {
            return Ok(Some(value.value.clone()));
        }
        let Some(runtime) = self.runtime.as_ref() else {
            return Ok(None);
        };
        if runtime.root_symbol_output_id(name).is_none() {
            return Ok(None);
        }
        runtime.root_symbol_value(name).map(Some)
    }

    pub fn symbol_output_id(&self, name: &str) -> Option<mech_core::OutputId> {
        if self.cleared_synthetic_symbols.contains(name) {
            return None;
        }
        if name == "ans" && self.pending_selection.is_some() {
            return None;
        }
        self.runtime
            .as_ref()
            .and_then(|runtime| runtime.root_symbol_output_id(name))
    }

    pub fn symbol_selection_identity(&self, name: &str) -> Option<&str> {
        if name != "ans" {
            return None;
        }
        self.pending_selection.as_ref()?.identity.as_deref()
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
        values.retain(|(name, _)| !self.cleared_synthetic_symbols.contains(name));
        if let Some(selected) = &self.pending_selection
            && (names.is_empty() || names.iter().any(|name| name == "ans"))
        {
            if let Some((_, value)) = values.iter_mut().find(|(name, _)| name == "ans") {
                *value = selected.value.clone();
            } else {
                values.push(("ans".to_string(), selected.value.clone()));
                values.sort_by(|left, right| left.0.cmp(&right.0));
            }
        }
        Ok(values)
    }

    pub fn symbol_inspections(
        &mut self,
        names: &[String],
    ) -> MResult<Vec<ResidentSymbolInspection>> {
        self.symbols(names)?
            .into_iter()
            .map(|(name, value)| {
                let selection_token = self.retain_selection(&name, value.clone(), None)?;
                Ok(ResidentSymbolInspection {
                    name,
                    value,
                    selection_token,
                })
            })
            .collect()
    }

    pub fn retain_selection(
        &mut self,
        source_echo: &str,
        value: RuntimeValueSnapshot,
        reuse_identity: Option<&str>,
    ) -> MResult<String> {
        if let Some(identity) = reuse_identity
            && let Some(token) = self.reusable_selection_tokens.get(identity).cloned()
        {
            self.retained_selections.insert(
                token.clone(),
                RetainedSelection {
                    source_echo: source_echo.to_string(),
                    value,
                },
            );
            return Ok(token);
        }
        let selection_token = NEXT_SELECTION_TOKEN
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
                next.checked_add(1)
            })
            .map_err(|_| interactive_error("resident selection token space exhausted"))?;
        let token = format!("selection:{selection_token}");
        self.retained_selections.insert(
            token.clone(),
            RetainedSelection {
                source_echo: source_echo.to_string(),
                value,
            },
        );
        if let Some(identity) = reuse_identity {
            self.reusable_selection_tokens
                .insert(identity.to_string(), token.clone());
        }
        Ok(token)
    }

    /// Refresh a previously retained selection without changing its public
    /// token. Long-lived host projections use this when their value is backed
    /// by a replacement runtime but their UI identity belongs to the host
    /// component rather than to that runtime.
    pub fn refresh_retained_selection(
        &mut self,
        token: &str,
        source_echo: &str,
        value: RuntimeValueSnapshot,
    ) -> MResult<()> {
        let Some(selection) = self.retained_selections.get_mut(token) else {
            return Err(interactive_error(format!(
                "retained selection token `{token}` is not available"
            )));
        };
        selection.source_echo = source_echo.to_string();
        selection.value = value;
        Ok(())
    }

    pub fn retained_selection(&self, token: &str) -> Option<(String, RuntimeValueSnapshot)> {
        self.retained_selections
            .get(token)
            .map(|selection| (selection.source_echo.clone(), selection.value.clone()))
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

    /// Establish a causal barrier between program producers and the next
    /// interactive mutation. Hosts call this before dispatch so clear and
    /// inspection commands observe every event published before the command.
    pub fn synchronize_program_events(&mut self) -> MResult<()> {
        self.collect_program_events()
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

fn remove_canonical_definitions(
    document: &crate::SourceDocument,
    requested: &std::collections::BTreeSet<String>,
) -> MResult<(String, std::collections::BTreeSet<String>)> {
    let mut removed = std::collections::BTreeSet::new();
    let mut ranges = Vec::new();
    let statements = mech_engine::CanonicalSourceFrontend
        .root_statement_nodes(&document.document())
        .map_err(|error| interactive_error(error.to_string()))?;
    for node in statements {
        if let Some(definition) = VariableDefineSyntax::cast(node.clone()) {
            let name = definition
                .variable()
                .and_then(|variable| variable.stem())
                .and_then(|stem| stem.syntax().text().ok());
            if name.as_ref().is_some_and(|name| requested.contains(name)) {
                removed.insert(name.unwrap());
                ranges.push(definition.syntax().range());
            }
            continue;
        }
        if let Some(destructure) = TupleDestructureSyntax::cast(node.clone()) {
            let names = destructure
                .names()
                .into_iter()
                .map(|name| {
                    name.syntax().text().map_err(|error| {
                        interactive_error(format!("invalid destructure name: {error:?}"))
                    })
                })
                .collect::<MResult<std::collections::BTreeSet<_>>>()?;
            if names.iter().any(|name| requested.contains(name)) {
                if !names.is_subset(requested) {
                    return Err(interactive_error(format!(
                        "cannot clear tuple destructure targets independently; clear all of {} together",
                        names.iter().cloned().collect::<Vec<_>>().join(", "),
                    )));
                }
                removed.extend(names);
                ranges.push(node.range());
            }
            continue;
        }
        let target = VariableAssignSyntax::cast(node.clone())
            .and_then(|assignment| assignment.target())
            .or_else(|| {
                OpAssignSyntax::cast(node.clone()).and_then(|assignment| assignment.target())
            });
        if let Some(name) = target
            .and_then(|target| target.stem())
            .and_then(|stem| stem.syntax().text().ok())
        {
            if requested.contains(&name) {
                ranges.push(node.range());
            }
            continue;
        }
    }
    let mut source = document.source().to_contiguous_string();
    let bytes = source.as_bytes();
    let mut byte_ranges = ranges
        .into_iter()
        .map(|range| {
            let mut start = range.start.0 as usize;
            let mut end = range.end.0 as usize;
            while end < bytes.len() && matches!(bytes[end], b' ' | b'\t') {
                end += 1;
            }
            if bytes.get(end) == Some(&b';') {
                end += 1;
                while end < bytes.len() && matches!(bytes[end], b' ' | b'\t') {
                    end += 1;
                }
            } else {
                while start > 0 && matches!(bytes[start - 1], b' ' | b'\t') {
                    start -= 1;
                }
                if start > 0 && bytes[start - 1] == b';' {
                    start -= 1;
                } else if start == 0 || bytes[start - 1] == b'\n' {
                    if bytes.get(end) == Some(&b'\r') {
                        end += 1;
                    }
                    if bytes.get(end) == Some(&b'\n') {
                        end += 1;
                    }
                }
            }
            (start, end)
        })
        .collect::<Vec<_>>();
    byte_ranges.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (start, end) in byte_ranges {
        if let Some(previous) = merged.last_mut().filter(|previous| start <= previous.1) {
            previous.1 = previous.1.max(end);
        } else {
            merged.push((start, end));
        }
    }
    for (start, end) in merged.into_iter().rev() {
        source.replace_range(start..end, "");
    }
    Ok((source, removed))
}

fn missing_variable_error(missing: &[String]) -> MechError {
    interactive_error(format!(
        "resident variable{} {} not found",
        if missing.len() == 1 { "" } else { "s" },
        missing
            .iter()
            .map(|name| format!("`{name}`"))
            .collect::<Vec<_>>()
            .join(", "),
    ))
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
    use mech_syntax::document::{DocumentId, StreamProgress};

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

    struct SourceRuntimeFactory;

    struct CanonicalRuntimeFactory {
        activations: std::rc::Rc<Cell<usize>>,
    }

    impl ResidentReplRuntimeFactory for SourceRuntimeFactory {
        fn build(&self, _events: MechEventBuffer) -> MResult<MechRuntime> {
            MechRuntime::builder()
                .function_catalog(mech_stdlib::source_catalog())
                .build()
        }
    }

    impl ResidentReplRuntimeFactory for CanonicalRuntimeFactory {
        fn build(&self, _events: MechEventBuffer) -> MResult<MechRuntime> {
            unreachable!("canonical test activation uses the retained document boundary")
        }

        fn activate_document(
            &self,
            _events: MechEventBuffer,
            document: &crate::SourceDocument,
        ) -> MResult<(MechRuntime, RuntimeProgramLoadOutcome)> {
            self.activations.set(self.activations.get() + 1);
            let mut runtime = MechRuntime::builder()
                .function_catalog(mech_stdlib::source_catalog())
                .build()?;
            if document.source().to_contiguous_string().trim().is_empty() {
                return Ok((
                    runtime,
                    RuntimeProgramLoadOutcome {
                        route: crate::RuntimeProgramRoute::None,
                        initial_value: RuntimeValueSnapshot::empty(),
                        info: crate::RuntimeProgramExecutionInfo::default(),
                    },
                ));
            }
            let mut compiler = crate::RuntimeBuilder::new()
                .function_catalog(mech_stdlib::source_catalog())
                .build_compiler()?;
            let product = compiler.compile_interactive_document(document)?;
            let durability = runtime.config().resident_durability;
            let outcome = runtime.load_bytecode_program(product.bytecode(), durability)?;
            Ok((runtime, outcome))
        }
    }

    fn finished_stream(id: u64, source: &str) -> DocumentStream {
        let mut stream = DocumentStream::new(DocumentId(id), ParseConfig::default());
        stream.append(source, u64::MAX).unwrap();
        assert_eq!(stream.finish(u64::MAX).progress, StreamProgress::Finished);
        stream
    }

    #[test]
    fn canonical_clear_removes_definitions_assignments_and_op_assignments() {
        let initial = crate::SourceDocument::parse_resolved(
            "repl://clear",
            Revision(0),
            Arc::<str>::from("~x := 1\nx = 2\nx += 1\ny := 9\n"),
            ParseConfig::default(),
        )
        .unwrap();
        let mut session = ResidentReplSession::from_document(
            CanonicalRuntimeFactory {
                activations: std::rc::Rc::new(Cell::new(0)),
            },
            initial,
        )
        .unwrap();
        assert_eq!(session.clear_variables(&["x".to_owned()]).unwrap(), ["x"]);
        assert_eq!(session.source(), "y := 9\n");
        assert!(session.clear_variables(&["x".to_owned()]).is_err());
    }

    #[test]
    fn canonical_interactive_edits_retain_nominal_provenance() {
        let origin = mech_core::CanonicalNominalPath::new(vec![
            "test-package".to_owned(),
            "interactive".to_owned(),
        ])
        .unwrap();
        let initial = crate::SourceDocument::parse_resolved(
            "repl://nominal-origin",
            Revision(0),
            Arc::<str>::from("<event> := :idle | :busy\nvalue := :idle\n"),
            ParseConfig::default(),
        )
        .unwrap()
        .with_nominal_origin(origin.clone())
        .with_nominal_package_id("test-package");
        let mut session = ResidentReplSession::from_document(
            CanonicalRuntimeFactory {
                activations: std::rc::Rc::new(Cell::new(0)),
            },
            initial,
        )
        .unwrap();
        session.submit("next := :busy").unwrap();
        assert_eq!(
            session.source_document.as_ref().unwrap().nominal_origin(),
            Some(&origin)
        );
        assert_eq!(
            session
                .source_document
                .as_ref()
                .unwrap()
                .nominal_package_id(),
            Some("test-package")
        );
        session.clear_variables(&["next".to_owned()]).unwrap();
        assert_eq!(
            session.source_document.as_ref().unwrap().nominal_origin(),
            Some(&origin)
        );
        assert_eq!(
            session
                .source_document
                .as_ref()
                .unwrap()
                .nominal_package_id(),
            Some("test-package")
        );
        let mut replacement = finished_stream(904, "<event> := :idle | :busy\nvalue := :busy\n");
        session.replace_finished_stream(&mut replacement).unwrap();
        assert_eq!(
            session.source_document.as_ref().unwrap().nominal_origin(),
            Some(&origin)
        );
        assert_eq!(
            session
                .source_document
                .as_ref()
                .unwrap()
                .nominal_package_id(),
            Some("test-package")
        );
    }

    #[test]
    fn canonical_clear_preserves_same_line_statements_and_tuple_ownership() {
        for source in [
            "~x := 1; y := 2\n",
            "y := 2; ~x := 1\n",
            "~x := 1; x += 3; y := 2\r\n",
        ] {
            let initial = crate::SourceDocument::parse_resolved(
                "repl://clear-inline",
                Revision(0),
                Arc::<str>::from(source),
                ParseConfig::default(),
            )
            .unwrap();
            let mut session = ResidentReplSession::from_document(
                CanonicalRuntimeFactory {
                    activations: std::rc::Rc::new(Cell::new(0)),
                },
                initial,
            )
            .unwrap();
            session.clear_variables(&["x".to_owned()]).unwrap();
            assert_eq!(
                session
                    .symbol("y")
                    .unwrap()
                    .unwrap()
                    .format_canonical_inline(),
                "2"
            );
            assert!(!session.source().contains("x"));
        }
        let initial = crate::SourceDocument::parse_resolved(
            "repl://clear-tuple",
            Revision(0),
            Arc::<str>::from("pair := (1, 2)\n(x, y) := pair; z := 3\n"),
            ParseConfig::default(),
        )
        .unwrap();
        let mut session = ResidentReplSession::from_document(
            CanonicalRuntimeFactory {
                activations: std::rc::Rc::new(Cell::new(0)),
            },
            initial,
        )
        .unwrap();
        let before = session.source().to_owned();
        assert!(session.clear_variables(&["x".to_owned()]).is_err());
        assert_eq!(session.source(), before);
        session
            .clear_variables(&["x".to_owned(), "y".to_owned()])
            .unwrap();
        assert_eq!(
            session
                .symbol("z")
                .unwrap()
                .unwrap()
                .format_canonical_inline(),
            "3"
        );
        assert!(!session.source().contains("(x, y)"));
    }

    #[test]
    fn canonical_inactive_mutations_preserve_root_state() {
        for entry in [
            "```mech:worker\n~counter := 0\ncounter += 9\n```\n",
            "```mech:disabled\ncounter = 99\n```\n",
            "unused() = result<f64> := ~counter := 0.0; counter += 9.0; result := counter.\n",
            "╭◉╮⸢~counter := 0\ncounter += 9\n⸥\n",
        ] {
            let initial = crate::SourceDocument::parse_resolved(
                "repl://scope-mutations",
                Revision(0),
                Arc::<str>::from("~counter := 0\ncounter += 1\n"),
                ParseConfig::default(),
            )
            .unwrap();
            let mut session = ResidentReplSession::from_document(
                CanonicalRuntimeFactory {
                    activations: std::rc::Rc::new(Cell::new(0)),
                },
                initial,
            )
            .unwrap();
            session.step(2).unwrap();
            let before = session.symbol("counter").unwrap().unwrap();
            let mut stream = finished_stream(909, entry);
            session.submit_finished_stream(&mut stream).unwrap();
            assert_eq!(
                session.symbol("counter").unwrap().unwrap(),
                before,
                "{entry}"
            );
            let mut mutation = finished_stream(910, "counter += 1\ncounter\n");
            session.submit_finished_stream(&mut mutation).unwrap();
            assert_eq!(
                session
                    .symbol("counter")
                    .unwrap()
                    .unwrap()
                    .format_canonical_inline(),
                "2"
            );
        }
    }

    #[test]
    fn canonical_invariants_keep_queryable_sigil_names() {
        let initial = crate::SourceDocument::parse_resolved(
            "repl://invariants",
            Revision(0),
            Arc::<str>::from("x := 1\nsafe! := x <= 2\n"),
            ParseConfig::default(),
        )
        .unwrap();
        let session = ResidentReplSession::from_document(
            CanonicalRuntimeFactory {
                activations: std::rc::Rc::new(Cell::new(0)),
            },
            initial,
        )
        .unwrap();
        for names in [vec![], vec!["safe!".to_owned()]] {
            let constraints = session.integrity_constraints(&names).unwrap();
            assert_eq!(constraints.len(), 1);
            assert_eq!(constraints[0].0, "safe!");
            assert_eq!(constraints[0].1.format_canonical_inline(), "true");
        }
        assert!(
            session
                .integrity_constraints(&["safe".to_owned()])
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn canonical_finished_stream_preserves_pending_selection() {
        for (streamed, suppress) in [(false, false), (true, false), (false, true), (true, true)] {
            let initial = crate::SourceDocument::parse_resolved(
                "repl://selected-stream",
                Revision(0),
                Arc::<str>::from("selected := 40\n~counter := 0\ncounter += 1\n1\n"),
                ParseConfig::default(),
            )
            .unwrap();
            let mut session = ResidentReplSession::from_document(
                CanonicalRuntimeFactory {
                    activations: std::rc::Rc::new(Cell::new(0)),
                },
                initial,
            )
            .unwrap();
            session.select_value("selected", session.symbol("selected").unwrap().unwrap());
            let accepted = session.source().to_owned();
            let counter = session.symbol("counter").unwrap().unwrap();
            let submit = |session: &mut ResidentReplSession<CanonicalRuntimeFactory>,
                          source: &str| {
                if streamed {
                    session.submit_finished_stream(&mut finished_stream(920, source))
                } else {
                    session.submit(source)
                }
            };
            assert!(submit(&mut session, "missing-name + ans\n").is_err());
            assert_eq!(session.source(), accepted);
            assert_eq!(session.symbol("counter").unwrap().unwrap(), counter);
            assert_eq!(
                session
                    .symbol("ans")
                    .unwrap()
                    .unwrap()
                    .format_canonical_inline(),
                "40"
            );
            session.drain_events().unwrap();
            let source = if suppress { "ans + 2;\n" } else { "ans + 2\n" };
            assert_eq!(
                submit(&mut session, source)
                    .unwrap()
                    .format_canonical_inline(),
                "42"
            );
            assert!(session.pending_selection.is_none());
            assert!(session.source_document().is_some());
            let events = session.drain_events().unwrap();
            assert_eq!(events.iter().filter(|event| matches!(&event.event, MechEvent::Repl(ReplEvent::SourceEcho {source: echo}) if echo == source.trim_end())).count(), 1);
            assert_eq!(events.iter().filter(|event| matches!(&event.event, MechEvent::Repl(ReplEvent::Response(response)) if response.kind == ReplResponseKind::ValueInspection)).count(), usize::from(!suppress));
            assert_eq!(
                submit(&mut session, "ans + 1\n")
                    .unwrap()
                    .format_canonical_inline(),
                "43"
            );
        }
    }

    #[test]
    fn canonical_interactive_lifecycle_executes_only_final_streams() {
        let activations = std::rc::Rc::new(Cell::new(0));
        let initial = crate::SourceDocument::parse_resolved(
            "repl://initial",
            Revision(0),
            Arc::<str>::from("x := 1\n"),
            ParseConfig::default(),
        )
        .unwrap();
        let mut session = ResidentReplSession::from_document(
            CanonicalRuntimeFactory {
                activations: std::rc::Rc::clone(&activations),
            },
            initial,
        )
        .unwrap();
        assert_eq!(activations.get(), 1);

        let mut open = DocumentStream::new(DocumentId(901), ParseConfig::default());
        open.append("y := x + 1\n", u64::MAX).unwrap();
        assert!(session.submit_finished_stream(&mut open).is_err());
        assert_eq!(activations.get(), 1);
        assert_eq!(session.source(), "x := 1\n");

        assert_eq!(open.finish(u64::MAX).progress, StreamProgress::Finished);
        let value = session.submit_finished_stream(&mut open).unwrap();
        assert_eq!(value.format_canonical_inline(), "2");
        assert_eq!(activations.get(), 2);
        assert!(session.source_document().is_some());

        let mut replacement = finished_stream(902, "x := 3\n");
        assert_eq!(
            session
                .replace_finished_stream(&mut replacement)
                .unwrap()
                .format_canonical_inline(),
            "3"
        );
        assert_eq!(activations.get(), 3);
        session.reset().unwrap();
        assert_eq!(session.source(), "x := 1\n");
        assert_eq!(activations.get(), 4);
        assert_eq!(session.clear_variables(&["x".to_owned()]).unwrap(), ["x"]);
        assert!(session.source().is_empty());
        assert_eq!(activations.get(), 5);

        let mut cancelled = DocumentStream::new(DocumentId(903), ParseConfig::default());
        cancelled.append("z := 9\n", u64::MAX).unwrap();
        cancelled.cancel();
        assert!(session.replace_finished_stream(&mut cancelled).is_err());
        assert_eq!(activations.get(), 5);
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
    fn standalone_reset_commits_a_clean_candidate_after_retirement_warning() {
        let factory = FailingRetirementFactory {
            activations: Cell::new(0),
        };
        let mut session = ResidentReplSession::new(factory);
        session.replace_source("baseline".to_string()).unwrap();

        session.reset().unwrap();

        assert_eq!(session.source(), "");
        assert!(session.symbols(&[]).unwrap().is_empty());
        assert!(
            !session
                .runtime()
                .expect("reset commits an empty candidate runtime")
                .ingress()
                .is_closed()
                .unwrap(),
        );
        assert!(session.drain_events().unwrap().iter().any(|event| {
            format!("{event:?}").contains("deliberate retired runtime stop failure")
        }));
    }

    #[test]
    fn canonical_interactive_turns_and_replacement_preserve_live_state() {
        let document = crate::SourceDocument::parse_resolved(
            "test:interactive-recurrence",
            Revision(0),
            "~counter := 0\ncounter += 1\ncounter\n",
            ParseConfig::default(),
        )
        .unwrap();
        let mut session = ResidentReplSession::from_document(
            CanonicalRuntimeFactory {
                activations: std::rc::Rc::new(Cell::new(0)),
            },
            document,
        )
        .unwrap();
        assert_eq!(session.symbol("counter").unwrap().unwrap().to_string(), "1");
        session.step(2).unwrap();
        assert_eq!(session.symbol("counter").unwrap().unwrap().to_string(), "3");
        session.submit("display := counter + 10").unwrap();
        assert_eq!(session.symbol("counter").unwrap().unwrap().to_string(), "3");
        assert_eq!(
            session.symbol("display").unwrap().unwrap().to_string(),
            "13"
        );
        assert_eq!(session.symbol("ans").unwrap().unwrap().to_string(), "13");
    }

    #[test]
    fn canonical_interactive_matrix_projection_and_failed_candidate_preserve_state() {
        let document = crate::SourceDocument::parse_resolved(
            "test:matrix-recurrence",
            Revision(0),
            "~values := [0f32; 1f32]\nvalues += 1f32\nvalues\n",
            ParseConfig::default(),
        )
        .unwrap();
        let mut session = ResidentReplSession::from_document(
            CanonicalRuntimeFactory {
                activations: std::rc::Rc::new(Cell::new(0)),
            },
            document,
        )
        .unwrap();
        session.step(2).unwrap();
        session.submit("display := values + 10f32").unwrap();
        for (name, values) in [("values", vec![3.0, 4.0]), ("display", vec![13.0, 14.0])] {
            assert_eq!(
                crate::RuntimeHostInputValue::from_numeric_value(
                    session.symbol(name).unwrap().unwrap().value()
                )
                .unwrap(),
                crate::RuntimeHostInputValue::F32Matrix {
                    rows: 2,
                    columns: 1,
                    values
                },
            );
        }
        let source = session.source().to_owned();
        let values = session.symbol("values").unwrap();
        let display = session.symbol("display").unwrap();
        assert!(session.submit("bad := undefined-call(values)").is_err());
        assert_eq!(session.source(), source);
        assert_eq!(session.symbol("values").unwrap(), values);
        assert_eq!(session.symbol("display").unwrap(), display);
        session.step(1).unwrap();
        assert_eq!(
            crate::RuntimeHostInputValue::from_numeric_value(
                session.symbol("values").unwrap().unwrap().value()
            )
            .unwrap(),
            crate::RuntimeHostInputValue::F32Matrix {
                rows: 2,
                columns: 1,
                values: vec![4.0, 5.0]
            },
        );
    }

    #[test]
    fn canonical_interactive_rewired_projection_uses_the_migrated_epoch() {
        let document = |source| {
            crate::SourceDocument::parse_resolved(
                "test:rewired-state",
                Revision(0),
                source,
                ParseConfig::default(),
            )
            .unwrap()
        };
        let mut session = ResidentReplSession::from_document(
            CanonicalRuntimeFactory {
                activations: std::rc::Rc::new(Cell::new(0)),
            },
            document("~a := 0\n~b := 100\na += 1\nb += 2\ndisplay := a + 10\n"),
        )
        .unwrap();
        session.step(2).unwrap();
        assert_eq!(
            session.symbol("display").unwrap().unwrap().to_string(),
            "13"
        );
        session
            .replace_document(document(
                "~a := 0\n~b := 100\na += 1\nb += 2\ndisplay := b + 10\n",
            ))
            .unwrap();
        for (name, expected) in [("a", "3"), ("b", "106"), ("display", "116"), ("ans", "116")] {
            assert_eq!(session.symbol(name).unwrap().unwrap().to_string(), expected);
        }
    }

    #[test]
    fn canonical_interactive_result_identity_is_independent_of_fence_publication() {
        let document = crate::SourceDocument::parse_resolved(
            "test:interactive-result",
            Revision(0),
            "~~~mech\n41\n~~~\n42\n",
            ParseConfig::default(),
        )
        .unwrap();
        let session = ResidentReplSession::from_document(
            CanonicalRuntimeFactory {
                activations: std::rc::Rc::new(Cell::new(0)),
            },
            document,
        )
        .unwrap();
        assert_eq!(session.symbol("ans").unwrap().unwrap().to_string(), "42");
        assert_eq!(
            session
                .runtime()
                .unwrap()
                .program_output_value()
                .unwrap()
                .unwrap()
                .to_string(),
            "42"
        );
    }

    #[test]
    fn accepted_source_preserves_compatible_live_resident_state() {
        let mut session = ResidentReplSession::new(SourceRuntimeFactory);
        session.submit("~counter := 0").unwrap();
        session.submit("counter += 1").unwrap();
        session.step(2).unwrap();
        let before = session.symbol("counter").unwrap().unwrap().to_string();
        assert_eq!(before, "3");

        session.submit("x := 1").unwrap();

        let after = session.symbol("counter").unwrap().unwrap().to_string();
        assert_eq!(after, before, "accepted source must not replay live state");
        assert_eq!(session.symbol("x").unwrap().unwrap().to_string(), "1");
    }

    #[test]
    fn accepted_source_preserves_the_published_projection_of_live_state() {
        let mut session = ResidentReplSession::new(SourceRuntimeFactory);
        session.submit("~counter := 0").unwrap();
        session.submit("counter += 1").unwrap();
        session.submit("display := counter + 10").unwrap();
        session.step(2).unwrap();
        assert_eq!(session.symbol("counter").unwrap().unwrap().to_string(), "3");
        assert_eq!(
            session.symbol("display").unwrap().unwrap().to_string(),
            "13"
        );

        session.submit("~unrelated := 1").unwrap();

        assert_eq!(session.symbol("counter").unwrap().unwrap().to_string(), "3");
        assert_eq!(
            session.symbol("display").unwrap().unwrap().to_string(),
            "13",
            "replacement output projections must describe the migrated state epoch",
        );
        assert_eq!(
            session.symbol("unrelated").unwrap().unwrap().to_string(),
            "1"
        );
    }

    #[test]
    fn accepted_source_never_migrates_a_projection_across_distinct_state_identity() {
        let mut session = ResidentReplSession::new(SourceRuntimeFactory);
        session
            .replace_source(
                r#"
~a := 0
~b := 100
a += 1
b += 2
display := a + 10
"#
                .to_string(),
            )
            .unwrap();
        session.step(2).unwrap();
        assert_eq!(session.symbol("a").unwrap().unwrap().to_string(), "3");
        assert_eq!(session.symbol("b").unwrap().unwrap().to_string(), "106");
        assert_eq!(
            session.symbol("display").unwrap().unwrap().to_string(),
            "13"
        );

        session
            .replace_source(
                r#"
~a := 0
~b := 100
a += 1
b += 2
display := b + 10
"#
                .to_string(),
            )
            .unwrap();

        assert_eq!(session.symbol("a").unwrap().unwrap().to_string(), "3");
        assert_eq!(session.symbol("b").unwrap().unwrap().to_string(), "106");
        assert_eq!(
            session.symbol("display").unwrap().unwrap().to_string(),
            "116",
            "a rewired projection must be recomputed from migrated state without advancing its transition",
        );
    }

    #[test]
    fn accepted_source_returns_the_projection_refreshed_from_migrated_state() {
        let mut session = ResidentReplSession::new(SourceRuntimeFactory);
        session.submit("~counter := 0").unwrap();
        session.submit("counter += 1").unwrap();
        session.submit("display := counter + 10").unwrap();
        session.step(2).unwrap();

        let returned = session.submit("counter").unwrap();

        assert_eq!(returned.to_string(), "3");
        assert_eq!(session.symbol("ans").unwrap().unwrap().to_string(), "3");
    }

    #[test]
    fn accepted_source_resets_a_same_named_state_when_its_schema_changes() {
        let mut session = ResidentReplSession::new(SourceRuntimeFactory);
        session.submit("~x := 0").unwrap();
        session.submit("x += 7").unwrap();

        session.replace_source("~x := \"new\"".to_string()).unwrap();

        assert_eq!(session.symbol("x").unwrap().unwrap().to_string(), "\"new\"");
    }

    #[test]
    fn accepted_source_recomputes_state_explicitly_mutated_by_the_submission() {
        let mut session = ResidentReplSession::new(SourceRuntimeFactory);
        session.submit("~answer := 0").unwrap();
        session.submit("answer += 42").unwrap();

        let result = session.submit("answer += 1\nanswer").unwrap();

        assert_eq!(result.to_string(), "43");
        assert_eq!(session.symbol("answer").unwrap().unwrap().to_string(), "43");
    }

    #[test]
    fn accepted_source_recomputes_only_projections_of_mutated_state() {
        let mut session = ResidentReplSession::new(SourceRuntimeFactory);
        session.submit("~counter := 0").unwrap();
        session.submit("counter += 1").unwrap();
        session.submit("display := counter + 10").unwrap();
        session.step(2).unwrap();

        session.submit("counter += 1").unwrap();

        assert_eq!(session.symbol("counter").unwrap().unwrap().to_string(), "2");
        assert_eq!(
            session.symbol("display").unwrap().unwrap().to_string(),
            "11",
            "a projection of explicitly mutated state must use the candidate plan, not the retired value",
        );
    }

    #[test]
    fn accepted_source_never_migrates_an_output_across_a_changed_definition() {
        let mut session = ResidentReplSession::new(SourceRuntimeFactory);
        session.submit("x := 1").unwrap();

        session.replace_source("x := 2".to_string()).unwrap();

        assert_eq!(
            session.symbol("x").unwrap().unwrap().to_string(),
            "2",
            "a matching lexical name cannot make a changed computation semantically compatible",
        );
    }

    #[test]
    fn missing_symbol_is_absent_even_when_an_unrelated_runtime_is_active() {
        let mut session = ResidentReplSession::new(SourceRuntimeFactory);
        session.submit("present := 7").unwrap();

        assert!(session.symbol("missing").unwrap().is_none());
        assert_eq!(session.symbol("present").unwrap().unwrap().to_string(), "7");
    }

    #[test]
    fn every_symbol_exposed_by_whos_including_ans_is_clearable() {
        let mut session = ResidentReplSession::new(SourceRuntimeFactory);
        session.submit("x := 7").unwrap();
        assert!(
            session
                .symbols(&[])
                .unwrap()
                .iter()
                .any(|(name, _)| name == "ans")
        );

        assert_eq!(
            session.clear_variables(&["ans".to_string()]).unwrap(),
            ["ans"]
        );
        assert!(session.symbol("ans").unwrap().is_none());
        assert!(
            !session
                .symbols(&[])
                .unwrap()
                .iter()
                .any(|(name, _)| name == "ans")
        );
        assert_eq!(session.symbol("x").unwrap().unwrap().to_string(), "7");

        session.submit("y := 8").unwrap();
        assert!(session.symbol("ans").unwrap().is_some());
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

    #[test]
    fn retained_selection_tokens_preserve_snapshot_and_ans_identity() {
        let mut session = ResidentReplSession::new(NeverBuild);
        let snapshot = RuntimeValueSnapshot::from_value(
            crate::RuntimeHostInputValue::F64(42.0)
                .into_value()
                .unwrap(),
        )
        .unwrap();
        let token = session
            .retain_selection("answer", snapshot.clone(), None)
            .unwrap();
        let (source_echo, retained) = session.retained_selection(&token).unwrap();

        assert_eq!(source_echo, "answer");
        assert_eq!(retained.to_string(), "42");
        session.select_value_with_identity("answer", retained, Some(token.clone()));
        assert_eq!(session.symbol("ans").unwrap().unwrap().to_string(), "42");
        assert_eq!(
            session.symbol_selection_identity("ans"),
            Some(token.as_str())
        );
    }

    #[test]
    fn retained_selection_tokens_can_refresh_without_changing_public_identity() {
        let mut session = ResidentReplSession::new(NeverBuild);
        let initial = RuntimeValueSnapshot::from_value(
            crate::RuntimeHostInputValue::F64(1.0).into_value().unwrap(),
        )
        .unwrap();
        let refreshed = RuntimeValueSnapshot::from_value(
            crate::RuntimeHostInputValue::F64(2.0).into_value().unwrap(),
        )
        .unwrap();
        let token = session.retain_selection("ans", initial, None).unwrap();

        session
            .refresh_retained_selection(&token, "ans", refreshed)
            .unwrap();

        let (source_echo, retained) = session.retained_selection(&token).unwrap();
        assert_eq!(source_echo, "ans");
        assert_eq!(retained.to_string(), "2");
    }

    #[test]
    fn program_event_barrier_orders_publish_before_clear() {
        let captured = Arc::new(Mutex::new(None));
        let mut session = ResidentReplSession::from_source(
            CapturingProgramEventFactory {
                events: Arc::clone(&captured),
            },
            "baseline".to_string(),
        )
        .unwrap();
        session
            .publish_program_event(MechEvent::Output(crate::OutputEvent {
                source: OutputSource::program(),
                stream: crate::OutputStream::Stdout,
                display_id: Some(crate::DisplayId::new("queued")),
                operation: crate::DisplayOperation::Create,
                content: OutputContent::Text(crate::TextOutput::new("queued output")),
            }))
            .unwrap();

        session.synchronize_program_events().unwrap();
        assert_eq!(session.outputs().len(), 1);
        session.clear_outputs();
        assert!(session.outputs().is_empty());
        let events = session.drain_events().unwrap();
        let operations = events
            .iter()
            .filter_map(|event| match &event.event {
                MechEvent::Output(output) => Some(output.operation),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            operations,
            [
                crate::DisplayOperation::Create,
                crate::DisplayOperation::Clear
            ],
        );
    }
}
