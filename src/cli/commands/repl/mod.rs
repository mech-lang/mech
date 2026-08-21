//! Interactive access to the production resident source-to-runtime route and
//! its portable event delivery protocol.
//!
//! The REPL deliberately owns no interpreter. It compiles every candidate
//! session program through the same resident runtime used by `mech run`; a
//! candidate replaces the live program only after it has activated correctly.

mod events;
mod presentation;
mod session;
mod terminal;
mod ui;

use std::env;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use chrono::{Datelike, Local, NaiveDateTime, Timelike};
use colored::Colorize;
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
#[cfg(feature = "mika")]
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use mech_core::{MResult, MechError};
use mech_runtime::{
    DiagnosticPhase, MechEvent, OutputContent, REPL_TEXT_LOGO, ReplDispatchControl, ReplEvent,
    ReplHostRequest, ReplResponse, ReplResponseKind, ReplResponseStatus, Severity, TextOutput,
    parse_repl_request,
};
#[cfg(feature = "mika")]
use mech_syntax::MICROMIKA_WAVE;

use self::presentation::{
    MECH_AMBER, capabilities, docs, list_directory, profiling, save_session_source, value,
};
use self::session::ResidentRepl;
use self::terminal::{render_events, render_events_collapsed};
use self::ui::ReplUi;
use crate::cli::outcome::CliOutcome;

const PROMPT: &str = ">: ";
const INPUT_POLL: Duration = Duration::from_millis(10);
const MAX_HOST_INPUTS_PER_TURN: usize = 64;

#[cfg(feature = "mika")]
const MIKA_FAREWELL_TEMPLATE: &str = "\n{spinner:.yellow} {msg}";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReplGreetingVariant {
    Standard,
    Halloween,
}

impl ReplGreetingVariant {
    const fn mika(self) -> &'static str {
        match self {
            Self::Standard => "╭◉╮",
            Self::Halloween => "ᗑ◉ᗑ",
        }
    }

    const fn interjection(self) -> Option<&'static str> {
        match self {
            Self::Standard => None,
            Self::Halloween => Some("Boo!"),
        }
    }
}

fn repl_greeting_at(now: NaiveDateTime) -> ReplGreetingVariant {
    if now.month() == 10 && now.day() == 31 && now.hour() == 0 {
        ReplGreetingVariant::Halloween
    } else {
        ReplGreetingVariant::Standard
    }
}

#[cfg(feature = "mika")]
fn play_mika_farewell(draw_target: ProgressDrawTarget, message: String, frame_delay: Duration) {
    let final_state = ProgressBar::with_draw_target(None, draw_target);
    let animation_style = ProgressStyle::with_template(MIKA_FAREWELL_TEMPLATE)
        .unwrap_or_else(|_| ProgressStyle::default_spinner())
        .tick_strings(MICROMIKA_WAVE);
    final_state.set_style(animation_style);
    final_state.set_message(message);

    for _ in 0..MICROMIKA_WAVE.len().saturating_sub(1) {
        thread::sleep(frame_delay);
        final_state.tick();
    }

    let resting_face = MICROMIKA_WAVE[0];
    let resting_style = ProgressStyle::with_template(MIKA_FAREWELL_TEMPLATE)
        .unwrap_or_else(|_| ProgressStyle::default_spinner())
        .tick_strings(&[resting_face, resting_face]);
    final_state.set_style(resting_style);
    final_state.finish();
}

#[derive(Debug)]
enum ReplInput {
    Line(String),
    EndOfInput,
    ReadFailed(String),
}

struct ReplInputWorker {
    input: Receiver<ReplInput>,
    resume: Sender<()>,
    handle: thread::JoinHandle<()>,
    waiting_for_resume: Arc<AtomicBool>,
}

enum ReplControl {
    Continue,
    Quit,
}

/// Starts the targetless resident REPL used by an argument-less `mech` command.
pub(crate) fn run(nofun: bool, quiet: bool) -> MResult<CliOutcome> {
    let ui = ReplUi::from_environment(nofun, quiet);
    #[cfg(windows)]
    if ui.color() {
        colored::control::set_virtual_terminal(true).map_err(|_| {
            MechError::new(
                mech_core::GenericError {
                    msg: "failed to enable Windows virtual terminal processing".to_string(),
                },
                None,
            )
        })?;
    }

    colored::control::set_override(ui.color());
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    if !ui.is_plain() {
        write!(stdout, "\x1B[2J\x1B[H")?;
        print_banner(&mut stdout)?;
    }

    let mut repl = ResidentRepl::new_with_quiet(ui.quiet())?;
    let worker = spawn_input_worker();
    let exit_requested = Arc::new(AtomicBool::new(false));
    let interrupt_count = Arc::new(AtomicUsize::new(0));
    install_interrupt_handler(exit_requested.clone(), interrupt_count.clone())?;

    let result = run_interactive_loop(
        &mut repl,
        &worker,
        exit_requested.as_ref(),
        interrupt_count.as_ref(),
        &mut stdout,
        &mut stderr,
        ui,
    );
    let worker_result = finish_input_worker(worker);
    let shutdown_result = repl.shutdown();

    if exit_requested.load(Ordering::Acquire) {
        print_interrupt_farewell(&mut stdout, ui)?;
    }

    result?;
    worker_result?;
    shutdown_result?;
    Ok(CliOutcome::exit(0))
}

fn run_interactive_loop(
    repl: &mut ResidentRepl,
    worker: &ReplInputWorker,
    exit_requested: &AtomicBool,
    interrupt_count: &AtomicUsize,
    output: &mut dyn Write,
    diagnostics: &mut dyn Write,
    ui: ReplUi,
) -> MResult<()> {
    repl.start_input_drivers()?;
    print_prompt(output, ui)?;
    let mut rendered_interrupts = 0;

    loop {
        render_pending_interrupts(output, interrupt_count, &mut rendered_interrupts, ui)?;
        if exit_requested.load(Ordering::Acquire) {
            return Ok(());
        }
        match worker.input.recv_timeout(INPUT_POLL) {
            Ok(ReplInput::Line(line)) => {
                repl.drain_all_pending_inputs()?;
                let control = process_input(repl, &line)?;
                render_events(output, diagnostics, &repl.drain_events()?, ui.mode())?;
                if matches!(control, ReplControl::Quit) {
                    print_interrupt_farewell(output, ui)?;
                    return Ok(());
                }
                repl.start_input_drivers()?;
                let _ = worker.resume.send(());
                print_prompt(output, ui)?;
            }
            Ok(ReplInput::EndOfInput) | Err(RecvTimeoutError::Disconnected) => return Ok(()),
            Ok(ReplInput::ReadFailed(error)) => {
                return Err(MechError::new(
                    mech_core::GenericError {
                        msg: format!("failed to read REPL input: {error}"),
                    },
                    None,
                ));
            }
            Err(RecvTimeoutError::Timeout) => {
                repl.drain_pending_inputs(MAX_HOST_INPUTS_PER_TURN)?;
                let events = repl.drain_events()?;
                if !events.is_empty() {
                    render_events(output, diagnostics, &events, ui.mode())?;
                }
            }
        }
    }
}

fn spawn_input_worker() -> ReplInputWorker {
    let (sender, receiver) = crossbeam_channel::bounded(1);
    let (resume, wait_for_resume) = crossbeam_channel::bounded(0);
    let waiting_for_resume = Arc::new(AtomicBool::new(false));
    let worker_waiting_for_resume = waiting_for_resume.clone();
    let handle = thread::spawn(move || {
        loop {
            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(0) => {
                    let _ = sender.send(ReplInput::EndOfInput);
                    break;
                }
                Ok(_) => {
                    worker_waiting_for_resume.store(true, Ordering::Release);
                    if sender.send(ReplInput::Line(input)).is_err()
                        || wait_for_resume.recv().is_err()
                    {
                        worker_waiting_for_resume.store(false, Ordering::Release);
                        break;
                    }
                    worker_waiting_for_resume.store(false, Ordering::Release);
                }
                Err(error) => {
                    let _ = sender.send(ReplInput::ReadFailed(error.to_string()));
                    break;
                }
            }
        }
    });
    ReplInputWorker {
        input: receiver,
        resume,
        handle,
        waiting_for_resume,
    }
}

fn finish_input_worker(worker: ReplInputWorker) -> MResult<()> {
    let should_join =
        worker.waiting_for_resume.load(Ordering::Acquire) || worker.handle.is_finished();
    drop(worker.resume);
    if should_join {
        worker.handle.join().map_err(|_| {
            MechError::new(
                mech_core::GenericError {
                    msg: "REPL input worker panicked".to_string(),
                },
                None,
            )
        })?;
    }
    Ok(())
}

fn install_interrupt_handler(
    exit_requested: Arc<AtomicBool>,
    interrupt_count: Arc<AtomicUsize>,
) -> MResult<()> {
    ctrlc::set_handler(move || {
        let count = interrupt_count.fetch_add(1, Ordering::AcqRel) + 1;
        if count >= 3 {
            exit_requested.store(true, Ordering::Release);
        }
    })
    .map_err(|error| {
        MechError::new(
            mech_core::GenericError {
                msg: format!("failed to install REPL Ctrl-C handler: {error}"),
            },
            None,
        )
    })
}

fn render_pending_interrupts(
    output: &mut dyn Write,
    interrupt_count: &AtomicUsize,
    rendered_interrupts: &mut usize,
    ui: ReplUi,
) -> io::Result<()> {
    let pending = interrupt_count.load(Ordering::Acquire).min(3);
    while *rendered_interrupts < pending {
        if ui.is_plain() {
            writeln!(output, ":ctrl+c")?;
        } else {
            writeln!(output, "{}", ":ctrl+c".bright_yellow())?;
        }
        *rendered_interrupts += 1;
        if *rendered_interrupts < 3 {
            if ui.is_plain() {
                writeln!(output, "Enter :quit to terminate this REPL session.")?;
            } else {
                writeln!(
                    output,
                    "\n{} {}Enter {} to terminate this REPL session.{}\n",
                    "╭◉─".truecolor(246, 192, 78),
                    "⸢".bright_yellow(),
                    ":quit".bright_yellow(),
                    "⸥".bright_yellow(),
                )?;
            }
            print_prompt(output, ui)?;
        }
    }
    Ok(())
}

fn run_with_io<R: BufRead, W: Write>(mut input: R, mut output: W) -> MResult<CliOutcome> {
    run_with_io_and_ui(&mut input, &mut output, ReplUi::rich())
}

fn run_with_io_and_ui<R: BufRead, W: Write>(
    mut input: R,
    mut output: W,
    ui: ReplUi,
) -> MResult<CliOutcome> {
    if !ui.is_plain() {
        print_banner(&mut output)?;
    }
    let mut repl = ResidentRepl::new_with_quiet(ui.quiet())?;
    loop {
        print_prompt(&mut output, ui)?;
        let mut entry = String::new();
        if input.read_line(&mut entry)? == 0 {
            repl.shutdown()?;
            return Ok(CliOutcome::exit(0));
        }
        let control = process_input(&mut repl, &entry)?;
        render_events_collapsed(&mut output, &repl.drain_events()?, ui.mode())?;
        if matches!(control, ReplControl::Quit) {
            print_farewell(&mut output, ui)?;
            repl.shutdown()?;
            return Ok(CliOutcome::exit(0));
        }
    }
}

fn process_input(repl: &mut ResidentRepl, input: &str) -> MResult<ReplControl> {
    if input.trim().is_empty() {
        return Ok(ReplControl::Continue);
    }
    let request = match parse_repl_request(input) {
        Ok(request) => request,
        Err(message) => {
            repl.emit_source_echo(input);
            repl.emit_message_diagnostic(
                Severity::Error,
                DiagnosticPhase::Host,
                "ReplCommand",
                message,
            );
            return Ok(ReplControl::Continue);
        }
    };
    let control = match repl.dispatch_request(request) {
        Ok(control) => control,
        Err(error) => {
            repl.emit_error(&error, DiagnosticPhase::Execute, Some("<repl>"));
            return Ok(ReplControl::Continue);
        }
    };
    match control {
        ReplDispatchControl::Continue => {}
        ReplDispatchControl::Quit => return Ok(ReplControl::Quit),
        ReplDispatchControl::Host(request) => {
            if let Err(error) = execute_host_request(repl, request) {
                repl.emit_error(&error, DiagnosticPhase::Host, Some("<repl>"));
            }
        }
        ReplDispatchControl::PendingStep { .. } => {
            unreachable!("the CLI requests synchronous shared dispatch")
        }
    }
    Ok(ReplControl::Continue)
}

fn execute_host_request(repl: &mut ResidentRepl, request: ReplHostRequest) -> MResult<()> {
    match request {
        ReplHostRequest::Capabilities => {
            emit_response(
                repl,
                ReplResponseKind::Command,
                ReplResponseStatus::Neutral,
                Some("Effective REPL host capabilities"),
                capabilities(repl.grants()),
            );
            emit_info(
                repl,
                "These grants and display capabilities apply whenever the REPL activates a candidate program.",
            );
        }
        ReplHostRequest::Documentation { topic } => emit_response(
            repl,
            ReplResponseKind::Help,
            ReplResponseStatus::Neutral,
            Some("Embedded documentation"),
            OutputContent::Text(TextOutput::new(docs(topic))),
        ),
        ReplHostRequest::ReadSources { resources } => match repl.load(&resources) {
            Ok(result) => {
                emit_success(
                    repl,
                    &format!("Loaded {} source file(s) transactionally.", resources.len()),
                );
                emit_value(repl, &result);
            }
            Err(error) => repl.emit_error(&error, DiagnosticPhase::Compile, None),
        },
        ReplHostRequest::WriteSource { resource, source } => {
            match save_session_source(Path::new(&resource), &source) {
                Ok(()) => emit_success(
                    repl,
                    &format!("Saved accepted session source to {resource}"),
                ),
                Err(error) => emit_host_error(repl, "Unable to save session source", &error),
            }
        }
        ReplHostRequest::ListResources { resource } => match list_directory(resource.as_deref()) {
            Ok((path, content)) => emit_response(
                repl,
                ReplResponseKind::Command,
                ReplResponseStatus::Neutral,
                Some(&format!("Directory: {}", path.display())),
                content,
            ),
            Err(error) => emit_host_error(repl, "Unable to list directory", &error),
        },
        ReplHostRequest::ChangeWorkingResource { resource } => {
            match env::set_current_dir(Path::new(&resource)) {
                Ok(()) => emit_success(
                    repl,
                    &format!("Working directory: {}", env::current_dir()?.display()),
                ),
                Err(error) => emit_host_error(repl, "Unable to change directory", &error),
            }
        }
        ReplHostRequest::Profile { enabled } => emit_response(
            repl,
            ReplResponseKind::Command,
            ReplResponseStatus::Neutral,
            Some("Resident profiling"),
            profiling(enabled),
        ),
    }
    Ok(())
}

fn emit_response(
    repl: &mut ResidentRepl,
    kind: ReplResponseKind,
    status: ReplResponseStatus,
    title: Option<&str>,
    content: OutputContent,
) {
    repl.emit(MechEvent::Repl(ReplEvent::Response(ReplResponse::new(
        kind,
        status,
        title.map(str::to_string),
        content,
    ))));
}

fn emit_value(repl: &mut ResidentRepl, result: &mech_runtime::RuntimeValueSnapshot) {
    if !repl.automatic_output_enabled() {
        return;
    }
    if let Some(content) = value(result) {
        emit_response(
            repl,
            ReplResponseKind::ValueInspection,
            ReplResponseStatus::Neutral,
            None,
            content,
        );
    }
}

fn emit_success(repl: &mut ResidentRepl, message: &str) {
    emit_response(
        repl,
        ReplResponseKind::Command,
        ReplResponseStatus::Success,
        None,
        OutputContent::Text(TextOutput::new(message)),
    );
}

fn emit_info(repl: &mut ResidentRepl, message: &str) {
    emit_response(
        repl,
        ReplResponseKind::Command,
        ReplResponseStatus::Info,
        None,
        OutputContent::Text(TextOutput::new(message)),
    );
}

fn emit_host_error(repl: &mut ResidentRepl, context: &str, error: &io::Error) {
    repl.emit_message_diagnostic(
        Severity::Error,
        DiagnosticPhase::Host,
        "IoError",
        format!("{context}: {error}"),
    );
}

fn print_banner(output: &mut dyn Write) -> io::Result<()> {
    let greeting = repl_greeting_at(Local::now().naive_local());
    writeln!(
        output,
        "{}",
        REPL_TEXT_LOGO.truecolor(MECH_AMBER.0, MECH_AMBER.1, MECH_AMBER.2)
    )?;
    writeln!(
        output,
        "\n                {}",
        format!("v{}", env!("CARGO_PKG_VERSION")).truecolor(
            MECH_AMBER.0,
            MECH_AMBER.1,
            MECH_AMBER.2
        )
    )?;
    writeln!(output, "           www.mech-lang.org\n")?;
    let interjection = greeting
        .interjection()
        .map(|text| format!("{} ", text.bright_magenta()))
        .unwrap_or_default();
    writeln!(
        output,
        "{} {}{}Enter {} for a list of all commands.{}\n",
        greeting
            .mika()
            .truecolor(MECH_AMBER.0, MECH_AMBER.1, MECH_AMBER.2),
        "⸢".bright_yellow(),
        interjection,
        ":help".bright_yellow(),
        "⸥".bright_yellow(),
    )
}

fn print_prompt(output: &mut dyn Write, ui: ReplUi) -> io::Result<()> {
    if ui.is_plain() {
        write!(output, "{PROMPT}")?;
        return output.flush();
    }
    write!(
        output,
        "{}",
        PROMPT.truecolor(MECH_AMBER.0, MECH_AMBER.1, MECH_AMBER.2)
    )?;
    output.flush()
}

fn print_farewell(output: &mut dyn Write, ui: ReplUi) -> io::Result<()> {
    if ui.is_plain() {
        return Ok(());
    }
    writeln!(
        output,
        "{} {}Okay cya!{}",
        "╭◉╮".truecolor(246, 192, 78),
        "⸢".bright_yellow(),
        "⸥".bright_yellow()
    )
}

fn print_interrupt_farewell(output: &mut dyn Write, ui: ReplUi) -> io::Result<()> {
    if ui.is_plain() {
        return Ok(());
    }
    output.flush()?;
    #[cfg(feature = "mika")]
    {
        if ui.animation() && io::stderr().is_terminal() {
            play_mika_farewell(
                ProgressDrawTarget::stderr(),
                format!("{}Okay cya!{}\n", "⸢".bright_yellow(), "⸥".bright_yellow()),
                Duration::from_millis(100),
            );
            Ok(())
        } else {
            print_farewell(output, ui)
        }
    }
    #[cfg(not(feature = "mika"))]
    {
        print_farewell(output, ui)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    #[cfg(feature = "mika")]
    use std::time::Instant;

    use super::*;

    #[test]
    fn resident_repl_preserves_the_product_interface_and_evaluates_source() {
        let mut output = Vec::new();
        let outcome =
            run_with_io(Cursor::new(":help\n1 + 1\n[1 1 2]\n:quit\n"), &mut output).unwrap();

        assert!(matches!(outcome, CliOutcome::Exit(0)));
        let output = String::from_utf8(output).unwrap();
        assert!(
            output.contains("www.mech-lang.org"),
            "missing REPL banner: {output}"
        );
        assert!(output.contains(":help"), "missing REPL greeting: {output}");
        for logo_line in REPL_TEXT_LOGO.lines().filter(|line| !line.is_empty()) {
            assert_eq!(
                output
                    .lines()
                    .filter(|rendered| rendered.trim_start() == logo_line.trim_start())
                    .count(),
                2,
                "the rich REPL should show its full logo at startup and indented in :help: {output}",
            );
        }
        assert!(output.contains("f64"), "missing scalar type: {output}");
        assert!(output.contains('2'), "missing scalar value: {output}");
        assert!(output.contains("Okay cya!"), "missing farewell: {output}");
    }

    #[test]
    fn command_failure_does_not_terminate_the_session() {
        let mut output = Vec::new();
        let outcome = run_with_io(
            Cursor::new(":cd definitely-not-a-real-directory\n1 + 1\n:quit\n"),
            &mut output,
        )
        .unwrap();

        assert!(matches!(outcome, CliOutcome::Exit(0)));
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("Unable to change directory"));
        assert!(output.contains("f64"));
        assert!(output.contains('2'));
    }

    #[test]
    fn halloween_greeting_keeps_its_original_date_gate() {
        let halloween =
            NaiveDateTime::parse_from_str("2026-10-31 00:30:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let ordinary =
            NaiveDateTime::parse_from_str("2026-10-31 01:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        assert_eq!(repl_greeting_at(halloween), ReplGreetingVariant::Halloween);
        assert_eq!(repl_greeting_at(ordinary), ReplGreetingVariant::Standard);
    }

    #[cfg(feature = "mika")]
    #[test]
    fn ctrl_c_farewell_animation_leaves_mika_visible() {
        use indicatif::InMemoryTerm;

        let terminal = InMemoryTerm::new(10, 80);
        let started = Instant::now();
        play_mika_farewell(
            ProgressDrawTarget::term_like(Box::new(terminal.clone())),
            "⸢Okay cya!⸥\n".to_string(),
            Duration::from_millis(10),
        );

        assert!(
            started.elapsed() >= Duration::from_millis(50),
            "Mika farewell did not advance through its wave frames"
        );
        assert!(terminal.contents().contains("╭◉╮ ⸢Okay cya!⸥"));
    }

    #[test]
    fn third_interrupt_finishes_the_prompt_line_without_another_warning() {
        let interrupt_count = AtomicUsize::new(3);
        let mut rendered_interrupts = 0;
        let mut output = Vec::new();

        render_pending_interrupts(
            &mut output,
            &interrupt_count,
            &mut rendered_interrupts,
            ReplUi::rich(),
        )
        .unwrap();

        let output = String::from_utf8(output).unwrap();
        assert_eq!(rendered_interrupts, 3);
        assert_eq!(output.matches(":ctrl+c").count(), 3);
        assert_eq!(output.matches("to terminate this REPL session.").count(), 2);
        assert_eq!(output.matches("\n\n").count(), 4);
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn nofun_repl_keeps_the_prompt_and_typed_value_without_decoration() {
        let mut output = Vec::new();
        let outcome = run_with_io_and_ui(
            Cursor::new(":help\n1 + 1\n:quit\n"),
            &mut output,
            ReplUi::plain(),
        )
        .unwrap();

        assert!(matches!(outcome, CliOutcome::Exit(0)));
        let output = String::from_utf8(output).unwrap();
        assert!(output.starts_with(">: "));
        assert!(!output.contains("www.mech-lang.org"));
        assert!(!output.contains("╭◉╮"));
        assert!(!output.contains("Okay cya!"));
        assert!(!output.contains(REPL_TEXT_LOGO));
        assert!(!output.contains("\u{1b}["));
        assert!(output.contains(">: f64\n2\n>: "));
    }

    #[test]
    fn code_command_has_one_causal_source_echo() {
        let mut repl = ResidentRepl::new().unwrap();
        assert!(matches!(
            process_input(&mut repl, ":code 1 + 1\n").unwrap(),
            ReplControl::Continue
        ));
        let echoes = repl
            .drain_events()
            .unwrap()
            .into_iter()
            .filter_map(|envelope| match envelope.event {
                MechEvent::Repl(ReplEvent::SourceEcho { source }) => Some(source),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(echoes, [":code 1 + 1"]);
        repl.shutdown().unwrap();
    }
}
