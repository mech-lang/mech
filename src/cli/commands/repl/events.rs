use std::env::VarError;

use mech_core::{GenericError, MResult, MechError};
use mech_runtime::{MechEvent, MechEventBuffer, OutputEvent, OutputStream};
use mech_terminal::{CliBackend, CliHostFactory};

#[derive(Clone, Debug)]
pub(super) struct EventCliBackend {
    events: MechEventBuffer,
}

impl EventCliBackend {
    pub(super) fn new(events: MechEventBuffer) -> Self {
        Self { events }
    }

    fn emit(&self, event: MechEvent) -> MResult<()> {
        self.events.emit(event)
    }
}

impl CliBackend for EventCliBackend {
    fn env_var(&self, name: &str) -> MResult<Option<String>> {
        match std::env::var(name) {
            Ok(value) => Ok(Some(value)),
            Err(VarError::NotPresent) => Ok(None),
            Err(VarError::NotUnicode(_)) => Err(event_error(format!(
                "environment variable `{name}` exists but is not valid Unicode"
            ))),
        }
    }

    fn write_stdout(&mut self, text: &str) -> MResult<()> {
        self.emit(MechEvent::Output(OutputEvent::text(text)))
    }

    fn write_stderr(&mut self, text: &str) -> MResult<()> {
        self.emit(MechEvent::Output(OutputEvent::stream_text(
            OutputStream::Stderr,
            text,
        )))
    }
}

pub(super) fn cli_host_factory(
    events: MechEventBuffer,
) -> MResult<CliHostFactory<EventCliBackend>> {
    CliHostFactory::with_backend(EventCliBackend::new(events))
}

fn event_error(message: impl Into<String>) -> MechError {
    MechError::new(
        GenericError {
            msg: message.into(),
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use mech_runtime::{OutputContent, OutputStream};

    use super::*;

    #[test]
    fn stdout_and_stderr_are_distinct_streams_with_exact_framing() {
        fn output(event: &MechEvent) -> &OutputEvent {
            match event {
                MechEvent::Output(output) => output,
                event => panic!("expected output, got {event:?}"),
            }
        }

        let events = MechEventBuffer::default();
        let mut backend = EventCliBackend::new(events.clone());
        backend.write_stdout("hello").unwrap();
        backend.write_stderr("warn").unwrap();
        backend.write_stderr("ing\n").unwrap();

        let events = events.drain().unwrap();
        assert_eq!(output(&events[0]).stream, OutputStream::Stdout);
        assert_eq!(output(&events[1]).stream, OutputStream::Stderr);
        assert_eq!(output(&events[2]).stream, OutputStream::Stderr);
        assert_eq!(
            events
                .iter()
                .map(output)
                .map(|output| match &output.content {
                    OutputContent::Text(text) => text.text.as_str(),
                    content => panic!("expected text output, got {content:?}"),
                })
                .collect::<String>(),
            "hellowarning\n"
        );
    }
}
