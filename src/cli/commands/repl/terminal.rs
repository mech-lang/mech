use std::io::{self, Write};

use colored::Colorize;
use mech_runtime::{
    DiagnosticEvent, DisplayOperation, MechEvent, MechEventEnvelope, OutputContent, OutputEvent,
    OutputStream, REPL_TEXT_LOGO, ReplClearTarget, ReplEvent, ReplResponse, ReplResponseKind,
    ReplResponseStatus, RichOutput, Severity, TelemetryEvent,
};

use super::presentation::{MECH_AMBER, render_table};
use super::ui::ReplRenderMode;

pub(super) fn render_events(
    interaction: &mut dyn Write,
    diagnostics: &mut dyn Write,
    events: &[MechEventEnvelope],
    mode: ReplRenderMode,
) -> io::Result<()> {
    for envelope in events {
        match &envelope.event {
            MechEvent::Diagnostic(diagnostic) => render_diagnostic(diagnostics, diagnostic, mode)?,
            MechEvent::Output(event) if event.stream == OutputStream::Stderr => {
                render_program_output(diagnostics, event, mode)?
            }
            event => render_interaction(interaction, event, mode)?,
        }
    }
    Ok(())
}

pub(super) fn render_events_collapsed(
    output: &mut dyn Write,
    events: &[MechEventEnvelope],
    mode: ReplRenderMode,
) -> io::Result<()> {
    for envelope in events {
        match &envelope.event {
            MechEvent::Diagnostic(diagnostic) => render_diagnostic(output, diagnostic, mode)?,
            event => render_interaction(output, event, mode)?,
        }
    }
    Ok(())
}

fn render_interaction(
    output: &mut dyn Write,
    event: &MechEvent,
    mode: ReplRenderMode,
) -> io::Result<()> {
    match event {
        MechEvent::Repl(event) => render_repl(output, event, mode),
        MechEvent::Output(event) => render_program_output(output, event, mode),
        MechEvent::Telemetry(event) => render_telemetry(output, event, mode),
        MechEvent::Diagnostic(_) => unreachable!("diagnostics use the diagnostic sink"),
    }
}

fn render_repl(output: &mut dyn Write, event: &ReplEvent, mode: ReplRenderMode) -> io::Result<()> {
    match event {
        // Terminals already echo input. Browser/native hosts use this event to
        // populate their interaction history.
        ReplEvent::SourceEcho { .. } => Ok(()),
        ReplEvent::Response(response) => render_response(output, response, mode),
        ReplEvent::FocusDisplay {
            display_id,
            stream: _,
            content,
        } => {
            render_heading(output, &format!("Output {display_id}"), mode)?;
            render_content(output, content, mode)
        }
        ReplEvent::Clear(ReplClearTarget::Interaction) if mode == ReplRenderMode::Rich => {
            write!(output, "\x1B[2J\x1B[H")
        }
        ReplEvent::Clear(ReplClearTarget::Interaction) => Ok(()),
        ReplEvent::Clear(ReplClearTarget::Diagnostics) if mode == ReplRenderMode::Plain => {
            writeln!(output, "Diagnostic history cleared.")
        }
        ReplEvent::Clear(ReplClearTarget::Diagnostics) => writeln!(
            output,
            "{} Diagnostic history cleared.",
            "[OK]".bright_green()
        ),
    }
}

fn render_response(
    output: &mut dyn Write,
    response: &ReplResponse,
    mode: ReplRenderMode,
) -> io::Result<()> {
    if let Some(title) = &response.title {
        if mode == ReplRenderMode::Rich {
            let mut rendered = Vec::new();
            render_heading(&mut rendered, title, mode)?;
            render_response_content(&mut rendered, response, mode)?;
            return write_indented(output, &rendered, "  ");
        }
        render_heading(output, title, mode)?;
        return render_response_content(output, response, mode);
    }

    if let OutputContent::Text(text) = &response.content {
        if mode == ReplRenderMode::Plain {
            return writeln!(output, "{}", text.text);
        }
        return match response.status {
            ReplResponseStatus::Success => {
                writeln!(output, "{} {}", "[OK]".bright_green(), text.text)
            }
            ReplResponseStatus::Info => {
                writeln!(output, "{} {}", "[Info]".bright_cyan(), text.text)
            }
            ReplResponseStatus::Neutral => writeln!(output, "{}", text.text),
        };
    }

    render_content(output, &response.content, mode)
}

fn write_indented(output: &mut dyn Write, rendered: &[u8], prefix: &str) -> io::Result<()> {
    for line in rendered.split_inclusive(|byte| *byte == b'\n') {
        output.write_all(prefix.as_bytes())?;
        output.write_all(line)?;
    }
    Ok(())
}

fn render_response_content(
    output: &mut dyn Write,
    response: &ReplResponse,
    mode: ReplRenderMode,
) -> io::Result<()> {
    if mode == ReplRenderMode::Plain
        && response.kind == ReplResponseKind::Help
        && let OutputContent::Fragments(fragments) = &response.content
    {
        for fragment in fragments {
            if matches!(fragment, OutputContent::Text(text) if text.text == REPL_TEXT_LOGO) {
                continue;
            }
            render_content(output, fragment, mode)?;
        }
        return Ok(());
    }
    render_content(output, &response.content, mode)
}

fn render_program_output(
    output: &mut dyn Write,
    event: &OutputEvent,
    mode: ReplRenderMode,
) -> io::Result<()> {
    if matches!(
        event.operation,
        DisplayOperation::Clear | DisplayOperation::Remove
    ) {
        return Ok(());
    }
    if let OutputContent::Text(text) = &event.content {
        output.write_all(text.text.as_bytes())?;
        return output.flush();
    }

    if let Some(display_id) = &event.display_id {
        render_heading(output, &format!("Output {display_id}"), mode)?;
    }
    render_content(output, &event.content, mode)
}

fn render_content(
    output: &mut dyn Write,
    content: &OutputContent,
    mode: ReplRenderMode,
) -> io::Result<()> {
    match content {
        OutputContent::Text(text) => writeln!(output, "{}", text.text),
        OutputContent::Value(value) if mode == ReplRenderMode::Plain => {
            let inline = if value.inline_text.is_empty() {
                &value.text
            } else {
                &value.inline_text
            };
            writeln!(output, "{}\n{inline}", value.kind)
        }
        OutputContent::Value(value) => writeln!(
            output,
            "\n{}\n{}",
            value.kind.as_str().ansi_color(218),
            value.text
        ),
        OutputContent::Table(table) => {
            render_tabular(output, &table.columns, &table.rows, &table.muted_rows, mode)
        }
        OutputContent::Matrix(matrix) => {
            let rows = matrix
                .cells
                .chunks(matrix.columns.max(1))
                .map(|row| row.to_vec())
                .collect::<Vec<_>>();
            if mode == ReplRenderMode::Plain {
                return render_inline_matrix(output, &rows);
            }
            let headers = (0..matrix.columns)
                .map(|column| column.to_string())
                .collect::<Vec<_>>();
            render_tabular(output, &headers, &rows, &[], mode)
        }
        OutputContent::Plot(plot) => render_rich(output, "plot", &plot.representations, mode),
        OutputContent::Scene(scene) => render_rich(output, "scene", &scene.representations, mode),
        OutputContent::Image(image) => {
            render_fallback(output, "image", image.text_fallback(), mode)
        }
        OutputContent::Custom(rich) => render_rich(output, "custom", rich, mode),
        OutputContent::Fragments(fragments) => {
            for fragment in fragments {
                render_content(output, fragment, mode)?;
            }
            Ok(())
        }
    }
}

fn render_inline_matrix(output: &mut dyn Write, rows: &[Vec<String>]) -> io::Result<()> {
    write!(output, "[")?;
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            write!(output, "; ")?;
        }
        write!(output, "{}", row.join(" "))?;
    }
    writeln!(output, "]")
}

fn render_rich(
    output: &mut dyn Write,
    kind: &str,
    rich: &RichOutput,
    mode: ReplRenderMode,
) -> io::Result<()> {
    render_fallback(output, kind, rich.text_fallback(), mode)
}

fn render_fallback(
    output: &mut dyn Write,
    kind: &str,
    fallback: Option<&str>,
    mode: ReplRenderMode,
) -> io::Result<()> {
    match fallback {
        Some(text) => writeln!(output, "{text}"),
        None if mode == ReplRenderMode::Plain => {
            writeln!(output, "{kind} output has no terminal representation.")
        }
        None => writeln!(
            output,
            "{} {kind} output has no terminal representation.",
            "[Info]".bright_cyan()
        ),
    }
}

fn render_tabular(
    output: &mut dyn Write,
    columns: &[String],
    rows: &[Vec<String>],
    muted_rows: &[usize],
    mode: ReplRenderMode,
) -> io::Result<()> {
    if mode == ReplRenderMode::Plain {
        write!(output, "|")?;
        for (index, column) in columns.iter().enumerate() {
            if index > 0 {
                write!(output, " ")?;
            }
            write!(output, "{column}<*>")?;
        }
        write!(output, "|")?;
        for row in rows {
            write!(output, " {} |", row.join(" "))?;
        }
        return writeln!(output);
    }
    let headers = columns.iter().map(String::as_str).collect::<Vec<_>>();
    let table = render_table(&headers, rows);
    for (line_index, line) in table.lines().enumerate() {
        let row_index = line_index.checked_sub(1);
        if row_index.is_some_and(|index| muted_rows.contains(&index)) {
            writeln!(output, "{}", line.dimmed())?;
        } else {
            writeln!(output, "{line}")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use mech_runtime::{
        ImageOutput, MatrixOutput, MechEventEnvelope, OutputContent, OutputEvent, OutputStream,
        Representation, RichOutput, TableOutput,
    };

    use super::*;

    #[test]
    fn stderr_output_uses_the_diagnostic_sink_without_changing_its_bytes() {
        let events = [
            MechEventEnvelope::new(0, MechEvent::Output(OutputEvent::text("out"))),
            MechEventEnvelope::new(
                1,
                MechEvent::Output(OutputEvent::stream_text(OutputStream::Stderr, "warn")),
            ),
            MechEventEnvelope::new(
                2,
                MechEvent::Output(OutputEvent::stream_text(OutputStream::Stderr, "ing\n")),
            ),
        ];
        let mut interaction = Vec::new();
        let mut diagnostics = Vec::new();

        render_events(
            &mut interaction,
            &mut diagnostics,
            &events,
            ReplRenderMode::Rich,
        )
        .unwrap();

        assert_eq!(String::from_utf8(interaction).unwrap(), "out");
        assert_eq!(String::from_utf8(diagnostics).unwrap(), "warning\n");
    }

    #[test]
    fn image_alt_text_is_the_terminal_fallback_for_binary_images() {
        let image = OutputContent::Image(ImageOutput {
            alt_text: Some("five bodies orbiting a barycenter".to_string()),
            representations: RichOutput::new(vec![Representation::binary(
                "image/png",
                vec![0, 1, 2],
            )]),
        });
        let mut output = Vec::new();

        render_content(&mut output, &image, ReplRenderMode::Rich).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "five bodies orbiting a barycenter\n"
        );
    }

    #[test]
    fn plain_structured_values_use_single_line_mech_syntax() {
        let table = OutputContent::Table(TableOutput::new(
            vec!["foo".to_string(), "bar".to_string()],
            vec![
                vec!["1".to_string(), "2".to_string()],
                vec!["3".to_string(), "4".to_string()],
            ],
        ));
        let matrix = OutputContent::Matrix(MatrixOutput {
            element_kind: "f64".to_string(),
            rows: 2,
            columns: 3,
            cells: ["1", "2", "3", "4", "5", "6"]
                .into_iter()
                .map(str::to_string)
                .collect(),
        });
        let mut output = Vec::new();

        render_content(&mut output, &table, ReplRenderMode::Plain).unwrap();
        render_content(&mut output, &matrix, ReplRenderMode::Plain).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "|foo<*> bar<*>| 1 2 | 3 4 |\n[1 2 3; 4 5 6]\n"
        );
    }

    #[test]
    fn plain_values_keep_unstyled_kind_and_value_on_separate_lines() {
        let value = OutputContent::Value(
            mech_runtime::ValueOutput::new("[f64]:1,3", "[1 2 3]").with_inline_text("[1 2 3]"),
        );
        let mut output = Vec::new();

        render_content(&mut output, &value, ReplRenderMode::Plain).unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "[f64]:1,3\n[1 2 3]\n");
    }

    #[test]
    fn titled_command_responses_indent_only_in_rich_mode() {
        let response = ReplResponse::new(
            ReplResponseKind::Command,
            ReplResponseStatus::Neutral,
            Some("Resident values".to_string()),
            OutputContent::Table(TableOutput::new(
                vec!["Name".to_string(), "Value".to_string()],
                vec![vec!["x".to_string(), "1".to_string()]],
            )),
        );
        let mut rich = Vec::new();
        let mut plain = Vec::new();

        render_response(&mut rich, &response, ReplRenderMode::Rich).unwrap();
        render_response(&mut plain, &response, ReplRenderMode::Plain).unwrap();

        let rich = String::from_utf8(rich).unwrap();
        assert!(rich.lines().all(|line| line.starts_with("  ")), "{rich:?}",);
        assert_eq!(
            String::from_utf8(plain).unwrap(),
            "Resident values\n|Name<*> Value<*>| x 1 |\n",
        );
    }
}

fn render_diagnostic(
    output: &mut dyn Write,
    diagnostic: &DiagnosticEvent,
    mode: ReplRenderMode,
) -> io::Result<()> {
    if mode == ReplRenderMode::Plain {
        let code = diagnostic
            .code
            .as_deref()
            .map(|code| format!(" {code}"))
            .unwrap_or_default();
        writeln!(
            output,
            "{}{code}: {}",
            severity_name(diagnostic.severity),
            diagnostic.message
        )?;
        return render_diagnostic_details(output, diagnostic);
    }
    let label = match diagnostic.severity {
        Severity::Info => "[Info]".bright_cyan().to_string(),
        Severity::Warning => "[Warning]".bright_yellow().to_string(),
        Severity::Error => "[Error]".truecolor(246, 98, 78).to_string(),
        Severity::Fatal => "[Fatal]".bright_red().bold().to_string(),
    };
    let code = diagnostic
        .code
        .as_deref()
        .map(|code| format!(" {code}"))
        .unwrap_or_default();
    writeln!(output, "{label}{code}: {}", diagnostic.message)?;
    render_diagnostic_details(output, diagnostic)
}

const fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Error => "error",
        Severity::Fatal => "fatal",
    }
}

fn render_diagnostic_details(
    output: &mut dyn Write,
    diagnostic: &DiagnosticEvent,
) -> io::Result<()> {
    if let Some(source) = &diagnostic.source {
        let name = source.source.as_deref().unwrap_or("<source>");
        writeln!(
            output,
            "  {name}:{}:{} ({:?})",
            source.start.line, source.start.column, diagnostic.phase
        )?;
    }
    for note in &diagnostic.notes {
        writeln!(output, "  note: {}", note.message)?;
    }
    Ok(())
}

fn render_telemetry(
    output: &mut dyn Write,
    event: &TelemetryEvent,
    mode: ReplRenderMode,
) -> io::Result<()> {
    let prefix = |rich: &'static str, plain: &'static str| {
        if mode == ReplRenderMode::Plain {
            plain
        } else {
            rich
        }
    };
    match event {
        TelemetryEvent::Profile { name, value } => {
            writeln!(
                output,
                "{} {name}: {value}",
                prefix("[Profile]", "profile:")
            )
        }
        TelemetryEvent::Trace { message } => {
            writeln!(output, "{} {message}", prefix("[Trace]", "trace:"))
        }
        TelemetryEvent::Timing { name, duration_ns } => {
            writeln!(
                output,
                "{} {name}: {duration_ns} ns",
                prefix("[Timing]", "timing:")
            )
        }
        TelemetryEvent::Debug { message } => {
            writeln!(output, "{} {message}", prefix("[Debug]", "debug:"))
        }
    }
}

fn render_heading(output: &mut dyn Write, title: &str, mode: ReplRenderMode) -> io::Result<()> {
    if mode == ReplRenderMode::Plain {
        return writeln!(output, "{title}");
    }
    writeln!(
        output,
        "{}",
        title
            .truecolor(MECH_AMBER.0, MECH_AMBER.1, MECH_AMBER.2)
            .bold()
    )
}
