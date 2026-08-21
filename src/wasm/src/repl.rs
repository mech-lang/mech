use mech_runtime::{MAX_RESIDENT_STEP_COUNT, ReplInputGesture};
use wasm_bindgen::prelude::*;

#[cfg(feature = "browser_project_core")]
use js_sys::{Array, Object, Reflect};
#[cfg(feature = "browser_project_core")]
use mech_console::{ConsoleBackend, ConsoleHostFactory};
#[cfg(feature = "browser_project_core")]
use mech_core::{GenericError, MResult, MechError};
#[cfg(feature = "browser_project_core")]
use mech_runtime::{
    ConfigValue, DiagnosticPhase, HostInstanceConfig, MechEvent, MechEventBuffer, MechRuntime,
    OutputContent, OutputEvent, REPL_COMMAND_SPECS, ReplDispatchControl, ReplEvent,
    ReplHostAvailability, ReplHostRequest, ReplHostRequirement, ReplResponse, ReplResponseKind,
    ReplResponseStatus, ReplStepMode, ResidentReplRuntimeFactory, ResidentReplSession,
    RunResourceGrantConfig, RuntimeConfig, TableOutput, TextOutput, ValueOutput,
    dispatch_repl_request, emit_host_response, emit_step_complete, parse_repl_request,
};

#[cfg(feature = "browser_project_core")]
use crate::project::{
    browser_runtime_builder, rendered_symbol_names_from_js, rendered_symbol_row, rendered_value,
};

/// Resolve browser keyboard events using the portable REPL input contract.
///
/// A browser terminal should prevent the event's default behavior whenever
/// this function returns an action. `submit` sends the complete editor value
/// as one source entry; `insert_line_break` inserts `\n` at the current caret.
#[wasm_bindgen(js_name = replInputAction)]
pub fn repl_input_action(
    key: &str,
    control: bool,
    alt: bool,
    shift: bool,
    meta: bool,
) -> Option<String> {
    let mut gesture = ReplInputGesture::new(key);
    gesture.control = control;
    gesture.alt = alt;
    gesture.shift = shift;
    gesture.meta = meta;
    gesture.action().map(|action| action.as_str().to_string())
}

/// Return the portable synchronous step ceiling for host-side validation.
#[wasm_bindgen(js_name = replStepLimit)]
pub fn repl_step_limit() -> u64 {
    MAX_RESIDENT_STEP_COUNT
}

#[cfg(feature = "browser_project_core")]
#[derive(Clone, Debug)]
pub(crate) struct ReplConsoleBackend {
    events: MechEventBuffer,
}

#[cfg(feature = "browser_project_core")]
impl ReplConsoleBackend {
    pub(crate) fn new(events: MechEventBuffer) -> Self {
        Self { events }
    }
}

#[cfg(feature = "browser_project_core")]
impl ConsoleBackend for ReplConsoleBackend {
    fn write_line(&mut self, text: &str) -> MResult<()> {
        self.events
            .emit(MechEvent::Output(OutputEvent::text(format!("{text}\n"))))
    }
}

#[cfg(feature = "browser_project_core")]
pub(crate) enum WasmReplRuntimeFactory {
    Standalone,
    Document(crate::project::WasmDocumentBootstrap),
}

#[cfg(feature = "browser_project_core")]
impl ResidentReplRuntimeFactory for WasmReplRuntimeFactory {
    fn build(&self, events: MechEventBuffer) -> MResult<MechRuntime> {
        match self {
            Self::Standalone => browser_runtime_builder()
                .config(RuntimeConfig::new("wasm-repl"))
                .host_factory(Box::new(ConsoleHostFactory::with_backend(
                    ReplConsoleBackend::new(events),
                )?))?
                .host_instance(HostInstanceConfig {
                    name: "repl".to_string(),
                    provider: "console".to_string(),
                    settings: ConfigValue::Map(Default::default()),
                })
                .run_resource_grant(RunResourceGrantConfig {
                    target: "repl/output".to_string(),
                    operations: vec!["write".to_string()],
                    paths: vec!["line".to_string()],
                })
                .build(),
            Self::Document(bootstrap) => {
                crate::project::build_document_repl_runtime(bootstrap, events)
            }
        }
    }

    fn activate(
        &self,
        events: MechEventBuffer,
        source: &str,
    ) -> MResult<(MechRuntime, mech_runtime::RuntimeProgramLoadOutcome)> {
        match self {
            Self::Standalone => {
                let mut runtime = self.build(events)?;
                let outcome = match runtime.load_interactive_source_program(
                    source,
                    mech_runtime::ResidentDurabilityPolicy::Volatile,
                ) {
                    Ok(outcome) => outcome,
                    Err(error) => {
                        let _ = runtime.shutdown();
                        return Err(error);
                    }
                };
                Ok((runtime, outcome))
            }
            Self::Document(bootstrap) => {
                crate::project::activate_document_repl_runtime(bootstrap, events, source)
            }
        }
    }

    fn prepare_commit(&self, runtime: &mut MechRuntime) -> MResult<()> {
        match self {
            Self::Standalone => Ok(()),
            Self::Document(bootstrap) => bootstrap.prepare_commit(runtime),
        }
    }

    fn commit(&self) {
        if let Self::Document(bootstrap) = self {
            bootstrap.commit();
        }
    }

    fn abort(&self) {
        if let Self::Document(bootstrap) = self {
            bootstrap.abort();
        }
    }
}

/// Exclusive lifecycle state for the public browser REPL embedding API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WasmReplState {
    Ready,
    AwaitingHost,
    Stepping { remaining: u64, total: u64 },
    Terminated,
}

impl WasmReplState {
    fn pending(self) -> bool {
        matches!(self, Self::Stepping { .. })
    }

    fn remaining(self) -> u64 {
        match self {
            Self::Stepping { remaining, .. } => remaining,
            Self::Ready | Self::AwaitingHost | Self::Terminated => 0,
        }
    }

    fn terminated(self) -> bool {
        matches!(self, Self::Terminated)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WasmReplTransition {
    Invoke,
    Submit,
    SetQuiet,
    Reset,
    StartStep { count: u64 },
    ContinueStep,
    StepChunkSucceeded { count: u64 },
    StepChunkFailed,
    Interrupt,
    FinishHostRequest,
    ClearOutputs,
    ClearDiagnostics,
    Shutdown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WasmReplTransitionResult {
    Allowed,
    Rejected,
    ContinueStep { remaining: u64 },
    StepCompleted { total: u64 },
    Interrupted { was_stepping: bool },
}

#[cfg(feature = "browser_project_core")]
#[wasm_bindgen]
/// Resident browser REPL used by the relocatable `{{REPL}}` document panel.
pub struct WasmRepl {
    pub(crate) session: ResidentReplSession<WasmReplRuntimeFactory>,
    availability: ReplHostAvailability,
    pub(crate) state: WasmReplState,
    pending_host_request: Option<ReplHostRequest>,
    console_output_context: String,
}

#[cfg(feature = "browser_project_core")]
#[wasm_bindgen]
impl WasmRepl {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmRepl {
        Self {
            session: ResidentReplSession::new(WasmReplRuntimeFactory::Standalone),
            availability: browser_repl_availability(),
            state: WasmReplState::Ready,
            pending_host_request: None,
            console_output_context: "console://repl/output".to_string(),
        }
    }

    /// Parse and dispatch one request through the portable command layer.
    pub fn invoke(&mut self, source: &str) -> Result<JsValue, JsValue> {
        if self.transition(WasmReplTransition::Invoke) == WasmReplTransitionResult::Rejected {
            return self.response(None);
        }
        match parse_repl_request(source) {
            Ok(request) => {
                match dispatch_repl_request(
                    &mut self.session,
                    request,
                    &self.availability,
                    ReplStepMode::Cooperative,
                ) {
                    Ok(ReplDispatchControl::PendingStep { count }) => {
                        self.transition(WasmReplTransition::StartStep { count });
                    }
                    Ok(ReplDispatchControl::Host(request)) => self.handle_host_request(request),
                    Ok(ReplDispatchControl::Quit) => {
                        self.transition(WasmReplTransition::Shutdown);
                        if let Err(error) = self.session.shutdown() {
                            self.session
                                .emit_error(&error, DiagnosticPhase::Host, Some("<repl>"));
                        }
                    }
                    Ok(ReplDispatchControl::Continue) => {}
                    Err(error) => {
                        self.session
                            .emit_error(&error, DiagnosticPhase::Execute, Some("<repl>"))
                    }
                }
            }
            Err(message) => {
                self.session.emit_source_echo(source);
                self.session.emit_message_diagnostic(
                    mech_runtime::Severity::Error,
                    DiagnosticPhase::Host,
                    "ReplCommand",
                    message,
                );
            }
        }
        self.response(None)
    }

    /// Submit one complete source entry. Embedded newlines remain part of the
    /// same transactional candidate.
    pub fn submit(&mut self, source: &str) -> Result<JsValue, JsValue> {
        if self.transition(WasmReplTransition::Submit) == WasmReplTransitionResult::Rejected {
            return self.response(None);
        }
        let display_result = self.session.submission_displays_result(source);
        let result = match self.session.submit(source) {
            Ok(snapshot) => {
                if snapshot.is_empty() || !display_result {
                    None
                } else {
                    Some(rendered_value(snapshot)?)
                }
            }
            Err(error) => {
                self.session
                    .emit_error(&error, DiagnosticPhase::Compile, Some("<repl>"));
                None
            }
        };
        self.response(result)
    }

    #[wasm_bindgen(js_name = setQuiet)]
    pub fn set_quiet(&mut self, quiet: bool) -> Result<JsValue, JsValue> {
        if self.transition(WasmReplTransition::SetQuiet) == WasmReplTransitionResult::Rejected {
            return self.response(None);
        }
        self.session.set_quiet(quiet);
        self.response(None)
    }

    pub fn reset(&mut self) -> Result<JsValue, JsValue> {
        if self.transition(WasmReplTransition::Reset) == WasmReplTransitionResult::Rejected {
            return self.response(None);
        }
        match self.session.reset() {
            Ok(()) => self
                .session
                .emit(MechEvent::Repl(ReplEvent::Response(ReplResponse::new(
                    ReplResponseKind::Command,
                    ReplResponseStatus::Success,
                    None,
                    OutputContent::Text(TextOutput::new("Resident REPL state cleared.")),
                )))),
            Err(error) => {
                self.session
                    .emit_error(&error, DiagnosticPhase::Host, Some("<repl>"));
            }
        }
        self.response(None)
    }

    pub fn step(&mut self, count: u64) -> Result<JsValue, JsValue> {
        self.transition(WasmReplTransition::StartStep { count });
        self.response(None)
    }

    /// Execute one bounded piece of a pending step request. Browser adapters
    /// yield to the event loop between calls so a legal large request cannot
    /// monopolize the UI thread.
    #[wasm_bindgen(js_name = continueStep)]
    pub fn continue_step(&mut self, max_steps: u32) -> Result<JsValue, JsValue> {
        let remaining = match self.transition(WasmReplTransition::ContinueStep) {
            WasmReplTransitionResult::Rejected => return self.response(None),
            WasmReplTransitionResult::ContinueStep { remaining } => remaining,
            _ => return self.response(None),
        };
        if max_steps == 0 {
            return Err(JsValue::from_str("max_steps must be greater than zero"));
        }
        if remaining == 0 {
            return self.response(None);
        }
        let count = remaining.min(u64::from(max_steps));
        match self.session.step_chunk(count) {
            Ok(()) => {
                if let WasmReplTransitionResult::StepCompleted { total } =
                    self.transition(WasmReplTransition::StepChunkSucceeded { count })
                    && let Err(error) = emit_step_complete(&mut self.session, total)
                {
                    self.session
                        .emit_error(&error, DiagnosticPhase::Execute, Some("<repl>"));
                }
            }
            Err(error) => {
                self.transition(WasmReplTransition::StepChunkFailed);
                self.session
                    .emit_error(&error, DiagnosticPhase::Execute, Some("<repl>"));
            }
        }
        self.response(None)
    }

    pub fn interrupt(&mut self) -> Result<JsValue, JsValue> {
        match self.transition(WasmReplTransition::Interrupt) {
            WasmReplTransitionResult::Rejected => return self.response(None),
            WasmReplTransitionResult::Interrupted { was_stepping: true } => {
                self.session.emit_message_diagnostic(
                    mech_runtime::Severity::Info,
                    DiagnosticPhase::Host,
                    "Interrupted",
                    "Cooperative resident step request interrupted.",
                );
            }
            _ => {}
        }
        self.response(None)
    }

    #[wasm_bindgen(js_name = commandIds)]
    pub fn command_ids(&self) -> Array {
        REPL_COMMAND_SPECS
            .iter()
            .map(|spec| JsValue::from_str(spec.id.as_str()))
            .collect()
    }

    #[wasm_bindgen(js_name = renderedSymbols)]
    pub fn rendered_symbols(&mut self, names: JsValue) -> Result<Array, JsValue> {
        if self.state.terminated() {
            return Err(JsValue::from_str("REPL session terminated"));
        }
        let names = rendered_symbol_names_from_js(names)?;
        let requested = names.unwrap_or_default();
        let mut rows = self.session.symbols(&requested).map_err(to_js_error)?;
        rows.sort_by(|left, right| left.0.cmp(&right.0));
        let out = Array::new();
        for (name, snapshot) in rows {
            out.push(&rendered_symbol_row(&name, snapshot)?);
        }
        Ok(out)
    }

    #[wasm_bindgen(js_name = clearOutputs)]
    pub fn clear_outputs(&mut self) -> Result<JsValue, JsValue> {
        if self.transition(WasmReplTransition::ClearOutputs) == WasmReplTransitionResult::Rejected {
            return self.response(None);
        }
        self.session.clear_outputs();
        self.response(None)
    }

    #[wasm_bindgen(js_name = clearDiagnostics)]
    pub fn clear_diagnostics(&mut self) -> Result<JsValue, JsValue> {
        if self.transition(WasmReplTransition::ClearDiagnostics)
            == WasmReplTransitionResult::Rejected
        {
            return self.response(None);
        }
        self.session.clear_diagnostics();
        self.response(None)
    }

    pub fn source(&self) -> String {
        self.session.source().to_string()
    }

    pub fn shutdown(&mut self) -> Result<JsValue, JsValue> {
        if self.transition(WasmReplTransition::Shutdown) == WasmReplTransitionResult::Rejected {
            return self.response(None);
        }
        self.session.shutdown().map_err(to_js_error)?;
        self.response(None)
    }

    pub(crate) fn response(&mut self, result: Option<JsValue>) -> Result<JsValue, JsValue> {
        let response = Object::new();
        let events = self.session.drain_events().map_err(to_js_error)?;
        Reflect::set(
            &response,
            &JsValue::from_str("events"),
            &serde_wasm_bindgen::to_value(&events)?,
        )?;
        Reflect::set(
            &response,
            &JsValue::from_str("result"),
            &result.unwrap_or(JsValue::NULL),
        )?;
        Reflect::set(
            &response,
            &JsValue::from_str("pending"),
            &JsValue::from_bool(self.state.pending()),
        )?;
        Reflect::set(
            &response,
            &JsValue::from_str("remaining"),
            &JsValue::from_f64(self.state.remaining() as f64),
        )?;
        Reflect::set(
            &response,
            &JsValue::from_str("terminated"),
            &JsValue::from_bool(self.state.terminated()),
        )?;
        Reflect::set(
            &response,
            &JsValue::from_str("hostRequest"),
            &self
                .pending_host_request
                .take()
                .map(|request| serde_wasm_bindgen::to_value(&request))
                .transpose()?
                .unwrap_or(JsValue::NULL),
        )?;
        Reflect::set(
            &response,
            &JsValue::from_str("hostPending"),
            &JsValue::from_bool(matches!(self.state, WasmReplState::AwaitingHost)),
        )?;
        Ok(response.into())
    }

    fn handle_host_request(&mut self, request: ReplHostRequest) {
        match request {
            ReplHostRequest::Capabilities => emit_host_response(
                &mut self.session,
                Some("Effective REPL host capabilities"),
                ReplResponseStatus::Neutral,
                OutputContent::Table(TableOutput::new(
                    vec![
                        "Context".to_string(),
                        "Operation".to_string(),
                        "Status".to_string(),
                        "Paths / fallback".to_string(),
                    ],
                    vec![
                        vec![
                            self.console_output_context.clone(),
                            "write".to_string(),
                            "granted".to_string(),
                            "line".to_string(),
                        ],
                        vec![
                            "browser output".to_string(),
                            "render".to_string(),
                            "granted".to_string(),
                            "text, value, table, matrix, scene".to_string(),
                        ],
                        vec![
                            "host resources".to_string(),
                            "load/save/list/cd".to_string(),
                            "unavailable".to_string(),
                            "no resource provider installed".to_string(),
                        ],
                    ],
                )),
            ),
            request @ ReplHostRequest::Documentation { .. } => {
                self.state = WasmReplState::AwaitingHost;
                self.pending_host_request = Some(request);
            }
            _ => unreachable!("unavailable browser host requests are rejected before dispatch"),
        }
    }

    fn transition(&mut self, transition: WasmReplTransition) -> WasmReplTransitionResult {
        use WasmReplState::{AwaitingHost, Ready, Stepping, Terminated};
        use WasmReplTransition as Transition;
        use WasmReplTransitionResult as TransitionResult;

        match (self.state, transition) {
            (Terminated, _) => {
                self.session.emit_message_diagnostic(
                    mech_runtime::Severity::Error,
                    DiagnosticPhase::Host,
                    "ReplTerminated",
                    "This REPL session has terminated. Create a new session to evaluate another request.",
                );
                TransitionResult::Rejected
            }
            (AwaitingHost, Transition::FinishHostRequest) => {
                self.state = Ready;
                TransitionResult::Allowed
            }
            (AwaitingHost, Transition::Shutdown) => {
                self.state = Terminated;
                TransitionResult::Allowed
            }
            (AwaitingHost, _) => {
                self.session.emit_message_diagnostic(
                    mech_runtime::Severity::Error,
                    DiagnosticPhase::Host,
                    "ReplBusy",
                    "A browser host request is still pending. Finish it before mutating the resident session.",
                );
                TransitionResult::Rejected
            }
            (Stepping { remaining, .. }, Transition::ContinueStep) => {
                TransitionResult::ContinueStep { remaining }
            }
            (Stepping { total, .. }, Transition::StepChunkSucceeded { count }) => {
                let remaining = self
                    .state
                    .remaining()
                    .checked_sub(count)
                    .expect("a cooperative step chunk cannot exceed the pending request");
                if remaining == 0 {
                    self.state = Ready;
                    TransitionResult::StepCompleted { total }
                } else {
                    self.state = Stepping { remaining, total };
                    TransitionResult::Allowed
                }
            }
            (Stepping { .. }, Transition::StepChunkFailed) => {
                self.state = Ready;
                TransitionResult::Allowed
            }
            (Stepping { .. }, Transition::Interrupt) => {
                self.state = Ready;
                TransitionResult::Interrupted { was_stepping: true }
            }
            (Stepping { .. }, Transition::Shutdown) => {
                self.state = Terminated;
                TransitionResult::Allowed
            }
            (Stepping { .. }, _) => {
                self.session.emit_message_diagnostic(
                    mech_runtime::Severity::Error,
                    DiagnosticPhase::Host,
                    "ReplBusy",
                    "A cooperative :step request is still running. Interrupt it before submitting another request.",
                );
                TransitionResult::Rejected
            }
            (Ready, Transition::StartStep { count }) => {
                if let Err(error) = mech_runtime::validate_resident_step_count(count) {
                    self.session
                        .emit_error(&error, DiagnosticPhase::Execute, Some("<repl>"));
                    TransitionResult::Rejected
                } else {
                    self.state = Stepping {
                        remaining: count,
                        total: count,
                    };
                    TransitionResult::Allowed
                }
            }
            (Ready, Transition::ContinueStep) => TransitionResult::ContinueStep { remaining: 0 },
            (Ready, Transition::Interrupt) => TransitionResult::Interrupted {
                was_stepping: false,
            },
            (Ready, Transition::Shutdown) => {
                self.state = Terminated;
                TransitionResult::Allowed
            }
            (Ready, Transition::StepChunkSucceeded { .. } | Transition::StepChunkFailed) => {
                unreachable!("step completion requires a pending cooperative request")
            }
            (Ready, _) => TransitionResult::Allowed,
        }
    }
}

#[cfg(feature = "browser_project_core")]
impl WasmRepl {
    pub(crate) fn step_immediate(&mut self, count: u64) -> MResult<()> {
        if !matches!(self.state, WasmReplState::Ready) {
            return Err(MechError::new(
                GenericError {
                    msg: "cannot run a synchronous document step while the REPL is busy or terminated"
                        .to_string(),
                },
                None,
            ));
        }
        mech_runtime::validate_resident_step_count(count)?;
        self.session.step_chunk(count)
    }

    pub(crate) fn from_document(bootstrap: crate::project::WasmDocumentBootstrap) -> MResult<Self> {
        let console_output_context = bootstrap.console_output_context();
        Ok(Self {
            session: ResidentReplSession::from_source(
                WasmReplRuntimeFactory::Document(bootstrap),
                String::new(),
            )?,
            availability: browser_repl_availability(),
            state: WasmReplState::Ready,
            pending_host_request: None,
            console_output_context,
        })
    }

    pub(crate) fn finish_host_request(&mut self) -> Result<JsValue, JsValue> {
        self.pending_host_request = None;
        self.transition(WasmReplTransition::FinishHostRequest);
        self.response(None)
    }

    pub(crate) fn begin_selection(&mut self) -> Result<Option<JsValue>, JsValue> {
        if self.transition(WasmReplTransition::Submit) == WasmReplTransitionResult::Rejected {
            return self.response(None).map(Some);
        }
        Ok(None)
    }

    pub(crate) fn publish_selection(
        &mut self,
        source_echo: &str,
        value: mech_runtime::RuntimeValueSnapshot,
    ) -> Result<(JsValue, Option<ValueOutput>), JsValue> {
        let presentation = self.session.select_value(source_echo, value);
        self.response(None).map(|response| (response, presentation))
    }

    pub(crate) fn host_request_pending(&self) -> bool {
        matches!(self.state, WasmReplState::AwaitingHost)
    }
}

#[cfg(feature = "browser_project_core")]
fn browser_repl_availability() -> ReplHostAvailability {
    ReplHostAvailability::all_available()
        .deny(
            ReplHostRequirement::ReadableResources,
            "this host did not provide a readable resource provider",
        )
        .deny(
            ReplHostRequirement::WritableResources,
            "this host did not provide a writable resource provider",
        )
        .deny(
            ReplHostRequirement::WorkingDirectory,
            "browser hosts do not expose a process working directory",
        )
        .deny(
            ReplHostRequirement::Profiling,
            "the resident runtime does not expose profiling controls",
        )
}

#[cfg(feature = "browser_project_core")]
fn to_js_error(error: mech_core::MechError) -> JsValue {
    JsValue::from_str(&format!("{:?}", error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(feature = "browser_project_core", target_arch = "wasm32"))]
    fn assert_terminated_response(response: JsValue) {
        assert_eq!(
            Reflect::get(&response, &JsValue::from_str("terminated"))
                .unwrap()
                .as_bool(),
            Some(true)
        );
        let events: Vec<mech_runtime::MechEventEnvelope> = serde_wasm_bindgen::from_value(
            Reflect::get(&response, &JsValue::from_str("events")).unwrap(),
        )
        .unwrap();
        assert!(events.iter().any(|event| matches!(
            &event.event,
            MechEvent::Diagnostic(diagnostic)
                if diagnostic.code.as_deref() == Some("ReplTerminated")
        )));
    }

    #[cfg(all(feature = "browser_project_core", target_arch = "wasm32"))]
    fn assert_busy_response(response: JsValue, expected_remaining: u64) {
        assert_eq!(
            Reflect::get(&response, &JsValue::from_str("pending"))
                .unwrap()
                .as_bool(),
            Some(true)
        );
        assert_eq!(
            Reflect::get(&response, &JsValue::from_str("remaining"))
                .unwrap()
                .as_f64(),
            Some(expected_remaining as f64)
        );
        assert_eq!(
            Reflect::get(&response, &JsValue::from_str("terminated"))
                .unwrap()
                .as_bool(),
            Some(false)
        );
        let events: Vec<mech_runtime::MechEventEnvelope> = serde_wasm_bindgen::from_value(
            Reflect::get(&response, &JsValue::from_str("events")).unwrap(),
        )
        .unwrap();
        assert!(events.iter().any(|event| matches!(
            &event.event,
            MechEvent::Diagnostic(diagnostic)
                if diagnostic.code.as_deref() == Some("ReplBusy")
        )));
    }

    #[test]
    fn wasm_uses_the_shared_enter_and_control_enter_contract() {
        assert_eq!(
            repl_input_action("Enter", false, false, false, false),
            Some("submit".to_string())
        );
        assert_eq!(
            repl_input_action("Enter", true, false, false, false),
            Some("insert_line_break".to_string())
        );
        assert_eq!(repl_input_action("a", false, false, false, false), None);
        assert_eq!(repl_step_limit(), MAX_RESIDENT_STEP_COUNT);
    }

    #[cfg(feature = "browser_project_core")]
    #[test]
    fn browser_repl_is_transactional_and_routes_program_output_to_its_event_bus() {
        let mut semicolon = ResidentReplSession::new(WasmReplRuntimeFactory::Standalone);
        semicolon.submit("1 + 1;\n").unwrap();
        assert!(!semicolon.source().contains(';'));
        let events = semicolon.drain_events().unwrap();
        assert!(
            events
                .iter()
                .any(|event| matches!(event.event, MechEvent::Repl(ReplEvent::SourceEcho { .. })))
        );
        assert!(!events.iter().any(|event| matches!(
            event.event,
            MechEvent::Repl(ReplEvent::Response(ReplResponse {
                kind: ReplResponseKind::ValueInspection,
                ..
            }))
        )));
        semicolon.shutdown().unwrap();

        let mut quiet = ResidentReplSession::with_quiet(WasmReplRuntimeFactory::Standalone, true);
        quiet.submit("1 + 1\n").unwrap();
        assert!(quiet.drain_events().unwrap().is_empty());
        quiet.shutdown().unwrap();

        let mut live = ResidentReplSession::new(WasmReplRuntimeFactory::Standalone);
        live.submit("~counter := 0\ncounter += 1\nanswer := 42\nanswer\n")
            .unwrap();
        assert_eq!(
            live.symbols(&["counter".to_string()]).unwrap()[0]
                .1
                .to_string(),
            "1"
        );
        live.step(1).unwrap();
        assert_eq!(
            live.symbols(&["counter".to_string()]).unwrap()[0]
                .1
                .to_string(),
            "2",
            "named inspection must read the stepped resident instance"
        );
        live.shutdown().unwrap();

        let mut aliases = ResidentReplSession::new(WasmReplRuntimeFactory::Standalone);
        aliases.submit("a := 1\nb := a\n").unwrap();
        let alias_values = aliases
            .symbols(&["a".to_string(), "b".to_string()])
            .unwrap();
        assert_eq!(
            alias_values
                .iter()
                .map(|(name, value)| (name.as_str(), value.to_string()))
                .collect::<Vec<_>>(),
            [("a", "1".to_string()), ("b", "1".to_string())],
            "interactive projection must retain every name for an aliased register"
        );
        aliases.shutdown().unwrap();

        let mut lexical_names = ResidentReplSession::new(WasmReplRuntimeFactory::Standalone);
        lexical_names
            .submit("odd/name := 1\nodd\\name := 2\nsafe := odd/name + odd\\name\n")
            .unwrap();
        let lexical_values = lexical_names
            .symbols(&[
                "odd/name".to_string(),
                r"odd\name".to_string(),
                "safe".to_string(),
            ])
            .unwrap();
        assert_eq!(
            lexical_values
                .iter()
                .map(|(name, value)| (name.as_str(), value.to_string()))
                .collect::<Vec<_>>(),
            [
                ("odd/name", "1".to_string()),
                (r"odd\name", "2".to_string()),
                ("safe", "3".to_string()),
            ],
            "lexical symbol names must survive the canonical artifact interface"
        );
        assert_eq!(
            lexical_names
                .symbols(&[])
                .unwrap()
                .into_iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>(),
            ["ans", "odd/name", r"odd\name", "safe"],
            "unfiltered symbol inspection must decode only lexical symbol outputs"
        );
        lexical_names.shutdown().unwrap();

        let mut rich_document = ResidentReplSession::new(WasmReplRuntimeFactory::Standalone);
        rich_document
            .replace_source(include_str!("../../../examples/working/fizzbuzz.mec").to_string())
            .unwrap();
        dispatch_repl_request(
            &mut rich_document,
            parse_repl_request(":code 1 + 1").unwrap(),
            &browser_repl_availability(),
            ReplStepMode::Cooperative,
        )
        .unwrap();
        let submitted_value = rich_document
            .drain_events()
            .unwrap()
            .into_iter()
            .find_map(|event| match event.event {
                MechEvent::Repl(ReplEvent::Response(ReplResponse {
                    kind: ReplResponseKind::ValueInspection,
                    content: OutputContent::Value(value),
                    ..
                })) => Some(value.text),
                _ => None,
            })
            .expect("`:code` value response after a rich document load");
        assert_eq!(submitted_value, "2");
        rich_document.shutdown().unwrap();

        let mut repl = ResidentReplSession::new(WasmReplRuntimeFactory::Standalone);
        repl.submit("answer := 1\n").unwrap();
        assert_eq!(repl.symbols(&["answer".to_string()]).unwrap().len(), 1);
        assert!(repl.submit("broken := (\n").is_err());
        assert_eq!(repl.submit("answer + 1\n").unwrap().to_string(), "2");
        assert_eq!(repl.symbols(&["answer".to_string()]).unwrap().len(), 1);

        repl.submit(
            "@out := console://repl/output{:write(line)}\n@out/line <- \"browser-output\"\n",
        )
        .unwrap();
        let events = repl.drain_events().unwrap();
        assert!(events.iter().any(|envelope| {
            matches!(
                &envelope.event,
                MechEvent::Output(output)
                    if matches!(
                        &output.content,
                        OutputContent::Text(text) if text.text == "browser-output\n"
                    )
            )
        }));
        repl.shutdown().unwrap();
    }

    #[cfg(feature = "browser_project_core")]
    #[test]
    fn cooperative_state_rejects_every_overlapping_mutation() {
        let mut repl = WasmRepl::new();
        assert_eq!(
            repl.transition(WasmReplTransition::StartStep { count: 1_000 }),
            WasmReplTransitionResult::Allowed
        );
        let pending = repl.state;

        for operation in [
            WasmReplTransition::Invoke,
            WasmReplTransition::Submit,
            WasmReplTransition::SetQuiet,
            WasmReplTransition::Reset,
            WasmReplTransition::StartStep { count: 2 },
            WasmReplTransition::ClearOutputs,
            WasmReplTransition::ClearDiagnostics,
        ] {
            assert_eq!(
                repl.transition(operation),
                WasmReplTransitionResult::Rejected
            );
            assert_eq!(
                repl.state, pending,
                "a rejected operation changed the pending request: {operation:?}"
            );
            let events = repl.session.drain_events().unwrap();
            assert!(events.iter().any(|event| matches!(
                &event.event,
                MechEvent::Diagnostic(diagnostic)
                    if diagnostic.code.as_deref() == Some("ReplBusy")
            )));
        }

        assert_eq!(
            repl.transition(WasmReplTransition::ContinueStep),
            WasmReplTransitionResult::ContinueStep { remaining: 1_000 }
        );
        assert_eq!(
            repl.transition(WasmReplTransition::Interrupt),
            WasmReplTransitionResult::Interrupted { was_stepping: true }
        );
        assert_eq!(repl.state, WasmReplState::Ready);

        assert_eq!(
            repl.transition(WasmReplTransition::StartStep { count: 2 }),
            WasmReplTransitionResult::Allowed
        );
        assert_eq!(
            repl.transition(WasmReplTransition::Shutdown),
            WasmReplTransitionResult::Allowed
        );
        assert_eq!(repl.state, WasmReplState::Terminated);
    }

    #[cfg(feature = "browser_project_core")]
    #[test]
    fn pending_host_request_owns_mutation_and_capabilities_use_the_active_console() {
        let mut repl = WasmRepl::new();
        repl.console_output_context = "console://repl-console-2/output".to_string();
        repl.handle_host_request(ReplHostRequest::Capabilities);
        let events = repl.session.drain_events().unwrap();
        assert!(events.iter().any(|event| matches!(
            &event.event,
            MechEvent::Repl(ReplEvent::Response(ReplResponse {
                content: OutputContent::Table(table),
                ..
            })) if table.rows.iter().flatten().any(|cell| cell == "console://repl-console-2/output")
        )));

        repl.handle_host_request(ReplHostRequest::Documentation {
            topic: Some("browser/value".to_string()),
        });
        assert_eq!(repl.state, WasmReplState::AwaitingHost);
        assert_eq!(
            repl.transition(WasmReplTransition::Submit),
            WasmReplTransitionResult::Rejected,
        );
        assert_eq!(repl.state, WasmReplState::AwaitingHost);
        assert_eq!(
            repl.transition(WasmReplTransition::FinishHostRequest),
            WasmReplTransitionResult::Allowed,
        );
        assert_eq!(repl.state, WasmReplState::Ready);
    }

    #[cfg(feature = "browser_project_core")]
    #[test]
    fn immediate_document_step_obeys_the_cooperative_state_guard() {
        let mut repl = WasmRepl::new();
        repl.session
            .submit("~counter := 0\ncounter += 1\n")
            .unwrap();

        repl.state = WasmReplState::Stepping {
            remaining: 1_000,
            total: 1_000,
        };
        assert!(repl.step_immediate(1).is_err());

        repl.state = WasmReplState::Ready;
        assert!(repl.step_immediate(1).is_ok());

        repl.state = WasmReplState::Terminated;
        assert!(repl.step_immediate(1).is_err());
    }

    #[cfg(all(feature = "browser_project_core", target_arch = "wasm32"))]
    #[wasm_bindgen_test::wasm_bindgen_test]
    fn direct_wasm_exports_preserve_a_pending_step_request() {
        let mut repl = WasmRepl::new();
        repl.submit("~counter := 0\ncounter += 1\n").unwrap();
        let source = repl.source();

        let started = repl.step(1_000).unwrap();
        assert_eq!(
            Reflect::get(&started, &JsValue::from_str("pending"))
                .unwrap()
                .as_bool(),
            Some(true)
        );
        assert_eq!(
            Reflect::get(&started, &JsValue::from_str("remaining"))
                .unwrap()
                .as_f64(),
            Some(1_000.0)
        );

        assert_busy_response(repl.invoke("other := 1\n").unwrap(), 1_000);
        assert_eq!(repl.source(), source);

        assert_busy_response(repl.submit("other := 1\n").unwrap(), 1_000);
        assert_eq!(repl.source(), source);
        assert_busy_response(repl.set_quiet(true).unwrap(), 1_000);
        assert!(!repl.session.is_quiet());
        assert_busy_response(repl.reset().unwrap(), 1_000);
        assert_eq!(
            repl.session.symbols(&["counter".to_string()]).unwrap()[0]
                .1
                .to_string(),
            "1"
        );

        assert_busy_response(repl.step(2).unwrap(), 1_000);
        assert_eq!(repl.source(), source);
        repl.session
            .emit(MechEvent::Output(mech_runtime::OutputEvent {
                source: mech_runtime::OutputSource::program(),
                stream: mech_runtime::OutputStream::Stdout,
                display_id: Some(mech_runtime::DisplayId::new("busy-retained")),
                operation: mech_runtime::DisplayOperation::Create,
                content: OutputContent::Text(TextOutput::new("retained while busy")),
            }));
        let retained_outputs = repl.session.outputs().len();
        assert_busy_response(repl.clear_outputs().unwrap(), 1_000);
        assert_eq!(repl.session.outputs().len(), retained_outputs);
        assert_busy_response(repl.clear_diagnostics().unwrap(), 1_000);
        assert_eq!(
            repl.state,
            WasmReplState::Stepping {
                remaining: 1_000,
                total: 1_000,
            }
        );

        let continued = repl.continue_step(1).unwrap();
        assert_eq!(
            Reflect::get(&continued, &JsValue::from_str("remaining"))
                .unwrap()
                .as_f64(),
            Some(999.0)
        );
        assert_eq!(
            repl.session.symbols(&["counter".to_string()]).unwrap()[0]
                .1
                .to_string(),
            "2"
        );

        let interrupted = repl.interrupt().unwrap();
        assert_eq!(
            Reflect::get(&interrupted, &JsValue::from_str("pending"))
                .unwrap()
                .as_bool(),
            Some(false)
        );
        repl.submit("other := 1\n").unwrap();
        assert!(repl.source().contains("other := 1"));
        repl.step(2).unwrap();
        let shutdown = repl.shutdown().unwrap();
        assert_eq!(
            Reflect::get(&shutdown, &JsValue::from_str("pending"))
                .unwrap()
                .as_bool(),
            Some(false)
        );
        assert_eq!(
            Reflect::get(&shutdown, &JsValue::from_str("terminated"))
                .unwrap()
                .as_bool(),
            Some(true)
        );
    }

    #[cfg(all(feature = "browser_project_core", target_arch = "wasm32"))]
    #[wasm_bindgen_test::wasm_bindgen_test]
    fn terminated_browser_repl_rejects_auxiliary_mutating_exports() {
        let mut repl = WasmRepl::new();
        repl.session
            .emit(MechEvent::Output(mech_runtime::OutputEvent {
                source: mech_runtime::OutputSource::program(),
                stream: mech_runtime::OutputStream::Stdout,
                display_id: Some(mech_runtime::DisplayId::new("retained")),
                operation: mech_runtime::DisplayOperation::Create,
                content: OutputContent::Text(TextOutput::new("retained output")),
            }));
        assert_eq!(repl.session.outputs().len(), 1);
        assert!(!repl.session.is_quiet());

        let quit = repl.invoke(":quit").unwrap();
        assert_eq!(
            Reflect::get(&quit, &JsValue::from_str("terminated"))
                .unwrap()
                .as_bool(),
            Some(true)
        );

        assert_terminated_response(repl.clear_outputs().unwrap());
        assert_eq!(
            repl.session.outputs().len(),
            1,
            "post-termination clearOutputs must not mutate retained output"
        );

        assert_terminated_response(repl.clear_diagnostics().unwrap());
        assert_terminated_response(repl.set_quiet(true).unwrap());
        assert!(
            !repl.session.is_quiet(),
            "post-termination setQuiet must not mutate session configuration"
        );
        assert_terminated_response(repl.shutdown().unwrap());
    }
}
