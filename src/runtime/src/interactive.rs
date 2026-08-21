//! Shared resident session state for interactive hosts.

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
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
                let _ = runtime.shutdown();
                return Err(error);
            }
        };
        Ok((runtime, outcome))
    }

    /// Build and activate an already parsed candidate tree.
    ///
    /// Document hosts override this boundary so their decoded program remains
    /// authoritative even when no lossless source text is available. Source-
    /// backed hosts retain the default behavior.
    fn activate_tree(
        &self,
        events: MechEventBuffer,
        source: &str,
        _tree: mech_core::nodes::Program,
    ) -> MResult<(MechRuntime, RuntimeProgramLoadOutcome)> {
        self.activate(events, source)
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
    initial_source: Option<String>,
    initial_tree: Option<mech_core::nodes::Program>,
    source: String,
    source_tree: Option<mech_core::nodes::Program>,
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
            initial_source: None,
            initial_tree: None,
            source: String::new(),
            source_tree: None,
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
        let mut session = Self {
            factory,
            initial_source: Some(source.clone()),
            initial_tree: None,
            source: String::new(),
            source_tree: None,
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
        session.replace_source(source)?;
        Ok(session)
    }

    /// Construct a session around an authoritative decoded program tree.
    ///
    /// The source remains available for persistence and transcript behavior,
    /// but interactive mutations extend and edit the tree directly instead of
    /// reparsing a potentially lossy formatted projection.
    pub fn from_tree(factory: F, source: String, tree: mech_core::nodes::Program) -> MResult<Self> {
        let mut session = Self {
            factory,
            initial_source: Some(source.clone()),
            initial_tree: Some(tree.clone()),
            source: String::new(),
            source_tree: None,
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
        session.replace_source_tree(source, tree)?;
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

    /// Return the accepted semantic program tree when this session was built
    /// from an authoritative decoded document.
    ///
    /// Rich document hosts must use this tree for output identity and document
    /// projections. Formatting `source()` and parsing it again is deliberately
    /// not equivalent: the textual projection is for persistence and echoing,
    /// while this tree remains the accepted semantic authority.
    pub fn source_tree(&self) -> Option<&mech_core::nodes::Program> {
        self.source_tree.as_ref()
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
        let overlay = mech_syntax::parser::parse(appended_source.trim())?;
        let changed_state_names = resident_state_mutations(&overlay);
        let value = if let Some(mut tree) = self.source_tree.clone() {
            tree.body.sections.extend(overlay.body.sections);
            self.replace_source_tree_preserving(candidate_source, tree, &changed_state_names)?
        } else {
            self.replace_source_preserving(candidate_source, &changed_state_names)?
        };
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

    fn replace_source_preserving(
        &mut self,
        candidate_source: String,
        changed_state_names: &std::collections::BTreeSet<String>,
    ) -> MResult<RuntimeValueSnapshot> {
        if self.source_tree.is_some() {
            let tree = mech_syntax::parser::parse(candidate_source.trim())?;
            return self.replace_source_tree_preserving(
                candidate_source,
                tree,
                changed_state_names,
            );
        }
        self.replace_source_candidate(candidate_source, None, Some(changed_state_names))
    }

    fn replace_source_tree(
        &mut self,
        candidate_source: String,
        candidate_tree: mech_core::nodes::Program,
    ) -> MResult<RuntimeValueSnapshot> {
        self.replace_source_tree_preserving(
            candidate_source,
            candidate_tree,
            &std::collections::BTreeSet::new(),
        )
    }

    fn replace_source_tree_preserving(
        &mut self,
        candidate_source: String,
        candidate_tree: mech_core::nodes::Program,
        changed_state_names: &std::collections::BTreeSet<String>,
    ) -> MResult<RuntimeValueSnapshot> {
        self.replace_source_candidate(
            candidate_source,
            Some(candidate_tree),
            Some(changed_state_names),
        )
    }

    fn replace_source_candidate(
        &mut self,
        candidate_source: String,
        candidate_tree: Option<mech_core::nodes::Program>,
        changed_state_names: Option<&std::collections::BTreeSet<String>>,
    ) -> MResult<RuntimeValueSnapshot> {
        let candidate_events = MechEventBuffer::default();
        let activated = match candidate_tree.clone() {
            Some(tree) => {
                self.factory
                    .activate_tree(candidate_events.clone(), &candidate_source, tree)
            }
            None => self
                .factory
                .activate(candidate_events.clone(), &candidate_source),
        };
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
                    let _ = candidate.shutdown();
                    self.factory.abort();
                    return Err(error);
                }
            }
        }

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
        self.source_tree = candidate_tree;
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
        Ok(outcome.initial_value)
    }

    /// Remove resident variables by rebuilding the complete accepted source.
    ///
    /// The candidate runtime is activated before the current runtime is
    /// retired, so a dependency or activation failure leaves the workspace
    /// unchanged. With no names, the complete resident workspace is removed.
    pub fn clear_variables(&mut self, names: &[String]) -> MResult<Vec<String>> {
        if names.is_empty() {
            if self.source_tree.is_some() {
                self.replace_source_candidate(
                    String::new(),
                    Some(mech_syntax::parser::parse("")?),
                    None,
                )?;
            } else {
                self.replace_source_candidate(String::new(), None, None)?;
            }
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
        let mut tree = match &self.source_tree {
            Some(tree) => tree.clone(),
            None => mech_syntax::parser::parse(self.source.trim())?,
        };
        let mut removed = std::collections::BTreeSet::new();
        for section in &mut tree.body.sections {
            for element in &mut section.elements {
                match element {
                    mech_core::nodes::SectionElement::MechCode(code) => {
                        remove_resident_definitions(code, &requested, &mut removed)?;
                    }
                    mech_core::nodes::SectionElement::FencedMechCode(fenced) => {
                        remove_resident_definitions(&mut fenced.code, &requested, &mut removed)?;
                    }
                    _ => {}
                }
            }
        }
        let missing = requested.difference(&removed).cloned().collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(interactive_error(format!(
                "resident variable{} {} not found",
                if missing.len() == 1 { "" } else { "s" },
                missing
                    .iter()
                    .map(|name| format!("`{name}`"))
                    .collect::<Vec<_>>()
                    .join(", "),
            )));
        }

        let candidate_source = mech_syntax::formatter::Formatter::new().format(&tree);
        if self.source_tree.is_some() {
            self.replace_source_tree(candidate_source, tree)?;
        } else {
            self.replace_source(candidate_source)?;
        }
        if clear_ans {
            self.cleared_synthetic_symbols.insert("ans".to_string());
            removed.insert("ans".to_string());
        }
        Ok(removed.into_iter().collect())
    }

    pub fn reset(&mut self) -> MResult<()> {
        if let Some(initial_source) = self.initial_source.clone() {
            if let Some(initial_tree) = self.initial_tree.clone() {
                self.replace_source_candidate(initial_source, Some(initial_tree), None)?;
            } else {
                self.replace_source_candidate(initial_source, None, None)?;
            }
            return Ok(());
        }
        if self.source_tree.is_some() {
            self.replace_source_candidate(
                String::new(),
                Some(mech_syntax::parser::parse("")?),
                None,
            )?;
        } else {
            self.replace_source_candidate(String::new(), None, None)?;
        }
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
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
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

fn remove_resident_definitions(
    code: &mut Vec<(
        mech_core::nodes::MechCode,
        Option<mech_core::nodes::Comment>,
    )>,
    requested: &std::collections::BTreeSet<String>,
    removed: &mut std::collections::BTreeSet<String>,
) -> MResult<()> {
    let mut retained = Vec::with_capacity(code.len());
    for entry in code.drain(..) {
        let (node, _) = &entry;
        let mech_core::nodes::MechCode::Statement(statement) = node else {
            retained.push(entry);
            continue;
        };
        let targets = match statement {
            mech_core::nodes::Statement::VariableDefine(definition) => {
                vec![definition.var.name.to_string()]
            }
            mech_core::nodes::Statement::VariableAssign(assignment) => {
                vec![assignment.target.name.to_string()]
            }
            mech_core::nodes::Statement::OpAssign(assignment) => {
                vec![assignment.target.name.to_string()]
            }
            mech_core::nodes::Statement::TupleDestructure(destructure) => destructure
                .vars
                .iter()
                .map(|variable| variable.to_string())
                .collect(),
            _ => {
                retained.push(entry);
                continue;
            }
        };
        let matched = targets
            .iter()
            .filter(|target| requested.contains(*target))
            .cloned()
            .collect::<Vec<_>>();
        if matched.is_empty() {
            retained.push(entry);
            continue;
        }
        if targets.len() > 1 && matched.len() != targets.len() {
            return Err(interactive_error(format!(
                "cannot clear {} independently because {} are defined by the same tuple destructure; clear all of them together",
                matched
                    .iter()
                    .map(|name| format!("`{name}`"))
                    .collect::<Vec<_>>()
                    .join(", "),
                targets
                    .iter()
                    .map(|name| format!("`{name}`"))
                    .collect::<Vec<_>>()
                    .join(", "),
            )));
        }
        removed.extend(matched);
    }
    *code = retained;
    Ok(())
}

fn resident_state_mutations(
    tree: &mech_core::nodes::Program,
) -> std::collections::BTreeSet<String> {
    use mech_core::nodes::{MechCode, SectionElement, Statement};

    let mut names = std::collections::BTreeSet::new();
    let mut collect = |code: &[(MechCode, Option<mech_core::nodes::Comment>)]| {
        for (node, _) in code {
            let MechCode::Statement(statement) = node else {
                continue;
            };
            match statement {
                Statement::VariableDefine(definition) if definition.mutable => {
                    names.insert(definition.var.name.to_string());
                }
                Statement::VariableAssign(assignment) => {
                    names.insert(assignment.target.name.to_string());
                }
                Statement::OpAssign(assignment) => {
                    names.insert(assignment.target.name.to_string());
                }
                Statement::TupleDestructure(destructure) => {
                    names.extend(
                        destructure
                            .vars
                            .iter()
                            .map(|variable| variable.name.to_string()),
                    );
                }
                _ => {}
            }
        }
    };

    for section in &tree.body.sections {
        for element in &section.elements {
            match element {
                SectionElement::MechCode(code) => collect(code),
                SectionElement::FencedMechCode(fenced) => collect(&fenced.code),
                _ => {}
            }
        }
    }
    names
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

    #[test]
    fn resident_state_mutations_include_compound_assignments() {
        let tree = mech_syntax::parser::parse("answer += 1\nanswer").unwrap();

        assert_eq!(
            resident_state_mutations(&tree),
            std::collections::BTreeSet::from(["answer".to_string()]),
        );
    }
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

    impl ResidentReplRuntimeFactory for SourceRuntimeFactory {
        fn build(&self, _events: MechEventBuffer) -> MResult<MechRuntime> {
            MechRuntime::builder()
                .function_catalog(mech_stdlib::source_catalog())
                .build()
        }
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
    fn accepted_source_recomputes_state_explicitly_mutated_by_the_submission() {
        let mut session = ResidentReplSession::new(SourceRuntimeFactory);
        session.submit("~answer := 0").unwrap();
        session.submit("answer += 42").unwrap();

        let result = session.submit("answer += 1\nanswer").unwrap();

        assert_eq!(result.to_string(), "43");
        assert_eq!(session.symbol("answer").unwrap().unwrap().to_string(), "43");
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
        let snapshot =
            RuntimeValueSnapshot::try_from(mech_core::LegacyValue::F64(mech_core::Ref::new(42.0)))
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
