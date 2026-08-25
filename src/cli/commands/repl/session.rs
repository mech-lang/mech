use std::fs;

use mech_core::MResult;
#[cfg(test)]
use mech_runtime::OutputArtifact;
use mech_runtime::{
    DiagnosticPhase, MechEvent, MechEventBuffer, MechEventEnvelope, MechRuntime, ReplComponentKind,
    ReplDispatchControl, ReplHostAvailability, ReplHostRequirement, ReplRequest, ReplStepMode,
    ResidentReplRuntimeFactory, ResidentReplSession, RuntimeConfig, RuntimeValueSnapshot, Severity,
    dispatch_repl_request,
};

use crate::cli::host_grants::{
    CliHostCapabilitySelection, EffectiveCliHostGrants, effective_cli_host_grants,
};
use crate::cli::run::cli_runtime_builder_with_cli_host_factory;

use super::events::cli_host_factory;

struct CliReplRuntimeFactory {
    grants: EffectiveCliHostGrants,
}

impl ResidentReplRuntimeFactory for CliReplRuntimeFactory {
    fn build(&self, events: MechEventBuffer) -> MResult<MechRuntime> {
        cli_runtime_builder_with_cli_host_factory(
            RuntimeConfig::new("repl"),
            &self.grants,
            &[],
            &[],
            Vec::new(),
            Box::new(cli_host_factory(events)?),
        )?
        .build()
    }
}

/// CLI adapter around the shared durable resident REPL session.
pub(super) struct ResidentRepl {
    grants: EffectiveCliHostGrants,
    session: ResidentReplSession<CliReplRuntimeFactory>,
}

impl ResidentRepl {
    #[cfg(test)]
    pub(super) fn new() -> MResult<Self> {
        Self::new_with_quiet(false)
    }

    #[cfg(test)]
    pub(super) fn new_with_quiet(quiet: bool) -> MResult<Self> {
        Self::new_with_options(quiet, mech_runtime::DEFAULT_REPL_VALUE_ELEMENT_LIMIT)
    }

    pub(super) fn new_with_options(quiet: bool, value_element_limit: usize) -> MResult<Self> {
        let grants = effective_cli_host_grants(None, CliHostCapabilitySelection::default())?;
        let mut session = ResidentReplSession::with_quiet(
            CliReplRuntimeFactory {
                grants: grants.clone(),
            },
            quiet,
        );
        session.set_value_element_limit(value_element_limit);
        Ok(Self { session, grants })
    }

    pub(super) fn grants(&self) -> &EffectiveCliHostGrants {
        &self.grants
    }

    #[cfg(test)]
    pub(super) fn submit(&mut self, entry: &str) -> MResult<RuntimeValueSnapshot> {
        self.session.submit(entry)
    }

    pub(super) fn dispatch_request(
        &mut self,
        request: ReplRequest,
    ) -> MResult<ReplDispatchControl> {
        let availability = cli_repl_availability();
        dispatch_repl_request(
            &mut self.session,
            request,
            &availability,
            ReplStepMode::Synchronous,
        )
    }

    pub(super) fn emit_source_echo(&mut self, source: &str) {
        self.session.emit_source_echo(source);
    }

    pub(super) fn automatic_output_enabled(&self) -> bool {
        !self.session.is_quiet()
    }

    pub(super) fn load(&mut self, paths: &[String]) -> MResult<RuntimeValueSnapshot> {
        let mut appended_source = String::new();
        for path in paths {
            appended_source.push_str(&fs::read_to_string(path)?);
            if !appended_source.ends_with('\n') {
                appended_source.push('\n');
            }
        }
        self.session.submit_host_source(&appended_source)
    }

    pub(super) fn start_input_drivers(&mut self) -> MResult<()> {
        self.session.start_input_drivers()
    }

    pub(super) fn drain_pending_inputs(&mut self, max_inputs: usize) -> MResult<usize> {
        self.session.drain_pending_inputs(max_inputs)
    }

    pub(super) fn drain_all_pending_inputs(&mut self) -> MResult<usize> {
        self.session.drain_all_pending_inputs()
    }

    pub(super) fn emit(&mut self, event: MechEvent) {
        self.session.emit(event);
    }

    pub(super) fn emit_error(
        &mut self,
        error: &mech_core::MechError,
        phase: DiagnosticPhase,
        source_name: Option<&str>,
    ) {
        self.session.emit_error(error, phase, source_name);
    }

    pub(super) fn emit_message_diagnostic(
        &mut self,
        severity: Severity,
        phase: DiagnosticPhase,
        code: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.session
            .emit_message_diagnostic(severity, phase, code, message);
    }

    pub(super) fn drain_events(&mut self) -> MResult<Vec<MechEventEnvelope>> {
        self.session.drain_events()
    }

    #[cfg(test)]
    pub(super) fn outputs(&self) -> Vec<OutputArtifact> {
        self.session.outputs()
    }

    pub(super) fn shutdown(&mut self) -> MResult<()> {
        self.session.shutdown()
    }
}

fn cli_repl_availability() -> ReplHostAvailability {
    let mut availability = ReplHostAvailability::all_available()
        .with_product("Mech", env!("CARGO_PKG_VERSION"))
        .deny(
            ReplHostRequirement::Profiling,
            "the resident runtime does not expose a profiling control or report API",
        );
    for library in mech_stdlib::INSTALLED_LIBRARIES {
        availability =
            availability.with_component(library.name, ReplComponentKind::Library, library.version);
    }
    #[cfg(feature = "cli_host")]
    {
        availability = availability.with_component(
            "terminal",
            ReplComponentKind::Host,
            mech_terminal::VERSION,
        );
    }
    #[cfg(any(feature = "web_host", feature = "host_delegation"))]
    {
        availability =
            availability.with_component("browser", ReplComponentKind::Host, mech_browser::VERSION);
    }
    #[cfg(any(
        feature = "time_host_native",
        feature = "time_host_browser",
        feature = "web_host"
    ))]
    {
        availability =
            availability.with_component("time", ReplComponentKind::Host, mech_time::VERSION);
    }
    #[cfg(any(
        feature = "timer_host_native",
        feature = "timer_host_browser",
        feature = "web_host"
    ))]
    {
        availability =
            availability.with_component("timer", ReplComponentKind::Host, mech_timer::VERSION);
    }
    #[cfg(any(
        feature = "console_host_native",
        feature = "console_host_browser",
        feature = "web_host"
    ))]
    {
        availability =
            availability.with_component("console", ReplComponentKind::Host, mech_console::VERSION);
    }
    #[cfg(any(
        feature = "scene_host_native",
        feature = "scene_host_browser",
        feature = "web_host"
    ))]
    {
        availability =
            availability.with_component("scene", ReplComponentKind::Host, mech_scene::VERSION);
    }
    #[cfg(feature = "full-hosts")]
    {
        availability = availability.with_component(
            "robot arm",
            ReplComponentKind::Host,
            mech_robot_arm::VERSION,
        );
    }
    #[cfg(feature = "compute_backends_native")]
    {
        availability =
            availability.with_component("GPU", ReplComponentKind::Host, mech_gpu::VERSION);
    }
    availability
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_runtime::{MechEvent, OutputContent, ReplEvent};

    #[test]
    fn rejected_entry_does_not_replace_the_live_resident_session() {
        let mut repl = ResidentRepl::new().unwrap();
        repl.submit("x := 1\n").unwrap();
        assert!(repl.submit("this := (\n").is_err());

        let value = repl.submit("x\n").unwrap();
        assert_eq!(value.to_string(), "1");
        repl.shutdown().unwrap();
    }

    #[test]
    fn resident_stdout_is_captured_as_program_output() {
        let mut repl = ResidentRepl::new().unwrap();
        repl.submit("+> @out := cli/stdout\n@out/line <- \"hello from mech\"\n")
            .unwrap();

        let events = repl.drain_events().unwrap();
        assert!(events.iter().any(|envelope| {
            matches!(
                &envelope.event,
                MechEvent::Output(output)
                    if matches!(&output.content, OutputContent::Text(text) if text.text == "hello from mech\n")
            )
        }));
        let outputs = repl.outputs();
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].id.as_str(), "output-1");
        repl.shutdown().unwrap();
    }

    #[test]
    fn resident_value_events_publish_canonical_text_as_the_primary_payload() {
        let mut repl = ResidentRepl::new().unwrap();
        repl.submit("[\"a\\\"b\" \"c\\\\d\\nnext\"]\n").unwrap();

        let events = repl.drain_events().unwrap();
        let value = events
            .iter()
            .find_map(|envelope| match &envelope.event {
                MechEvent::Repl(ReplEvent::Response(response)) => match &response.content {
                    OutputContent::Value(value) => Some(value),
                    _ => None,
                },
                _ => None,
            })
            .expect("submission value event");
        assert_eq!(value.text, "[\"a\\\"b\" \"c\\\\d\\nnext\"]");
        assert_eq!(value.inline_text, value.text);
        assert!(!value.text.chars().any(char::is_control));
        repl.shutdown().unwrap();
    }
}
