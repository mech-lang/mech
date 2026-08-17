//! Interactive access to the production resident source-to-runtime route.
//!
//! The REPL deliberately owns no interpreter.  It compiles every candidate
//! session program through the same resident runtime used by `mech run`; a
//! candidate replaces the live program only after it has activated correctly.

use std::env;
use std::fs;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use chrono::{Datelike, Local, NaiveDateTime, Timelike};
use colored::Colorize;
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use include_dir::{Dir, include_dir};
#[cfg(feature = "mika")]
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use mech_core::{MResult, MechError};
use mech_runtime::{MechRuntime, ResidentDurabilityPolicy, RuntimeConfig, RuntimeValueSnapshot};
#[cfg(feature = "mika")]
use mech_syntax::MICROMIKA_WAVE;

use crate::cli::host_grants::{
    CliHostCapabilitySelection, EffectiveCliHostGrants, effective_cli_host_grants,
};
use crate::cli::outcome::CliOutcome;
use crate::cli::run::new_cli_runtime;

static DOCS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/docs");

pub(crate) const TEXT_LOGO: &str = r#"
  ┌─────────┐ ┌──────┐ ┌─┐ ┌──┐ ┌─┐  ┌─┐
  └───┐ ┌───┘ └──────┘ │ │ └┐ │ │ │  │ │
  ┌─┐ │ │ ┌─┐ ┌──────┐ │ │  └─┘ │ └─┐│ │
  │ │ │ │ │ │ │ ┌────┘ │ │  ┌─┐ │ ┌─┘│ │
  │ │ └─┘ │ │ │ └────┐ │ └──┘ │ │ │  │ │
  └─┘     └─┘ └──────┘ └──────┘ └─┘  └─┘"#;

const PROMPT: &str = ">: ";
const INPUT_POLL: Duration = Duration::from_millis(10);
const MAX_HOST_INPUTS_PER_TURN: usize = 64;
const MECH_AMBER: (u8, u8, u8) = (246, 192, 78);

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

    // The wave ends with a blank cleanup frame. Preserve Mika's resting face
    // after the progress bar is dropped instead of leaving that blank frame.
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

#[derive(Clone, Debug, PartialEq, Eq)]
enum ReplCommand {
    Help,
    Quit,
    Save(String),
    Docs(Option<String>),
    Code(String),
    Ls,
    Profile(bool),
    Cd(String),
    Step(Option<usize>, Option<u64>),
    Load(Vec<String>),
    Whos(Vec<String>),
    Plan,
    Symbols(Option<String>),
    Clear,
    Clc,
}

enum ReplControl {
    Continue,
    Quit,
}

/// A durable REPL session backed exclusively by a resident runtime.
///
/// `source` is the accepted session program.  A failed entry is compiled in a
/// separate runtime and discarded, so it cannot replace the active program.
struct ResidentRepl {
    source: String,
    runtime: Option<MechRuntime>,
    grants: EffectiveCliHostGrants,
}

impl ResidentRepl {
    fn new() -> MResult<Self> {
        Ok(Self {
            source: String::new(),
            runtime: None,
            grants: effective_cli_host_grants(None, CliHostCapabilitySelection::default())?,
        })
    }

    fn new_runtime(&self) -> MResult<MechRuntime> {
        new_cli_runtime(RuntimeConfig::new("repl"), &self.grants, &[], &[])
    }

    fn submit(&mut self, entry: &str) -> MResult<RuntimeValueSnapshot> {
        let mut candidate_source = self.source.clone();
        candidate_source.push_str(entry);
        if !candidate_source.ends_with('\n') {
            candidate_source.push('\n');
        }
        self.activate(candidate_source)
    }

    fn load(&mut self, paths: &[String]) -> MResult<RuntimeValueSnapshot> {
        let mut candidate_source = self.source.clone();
        for path in paths {
            candidate_source.push_str(&fs::read_to_string(path)?);
            if !candidate_source.ends_with('\n') {
                candidate_source.push('\n');
            }
        }
        self.activate(candidate_source)
    }

    fn activate(&mut self, candidate_source: String) -> MResult<RuntimeValueSnapshot> {
        let mut candidate = self.new_runtime()?;
        let outcome = match candidate
            .load_source_program(&candidate_source, ResidentDurabilityPolicy::Volatile)
        {
            Ok(outcome) => outcome,
            Err(error) => {
                let _ = candidate.shutdown();
                return Err(error);
            }
        };

        // Do not disturb the currently published program until the candidate
        // has passed parsing, lowering, validation, and resident activation.
        if let Some(mut previous) = self.runtime.take() {
            previous.shutdown()?;
        }
        self.runtime = Some(candidate);
        self.source = candidate_source;
        Ok(outcome.initial_value)
    }

    fn reset(&mut self) -> MResult<()> {
        if let Some(mut runtime) = self.runtime.take() {
            runtime.shutdown()?;
        }
        self.source.clear();
        Ok(())
    }

    fn start_input_drivers(&mut self) -> MResult<()> {
        if let Some(runtime) = self.runtime.as_mut() {
            runtime.start_input_drivers()?;
        }
        Ok(())
    }

    fn drain_pending_inputs(&mut self, max_inputs: usize) -> MResult<usize> {
        let Some(runtime) = self.runtime.as_mut() else {
            return Ok(0);
        };
        runtime
            .drain_host_inputs(max_inputs)
            .map(|outcomes| outcomes.len())
    }

    fn drain_all_pending_inputs(&mut self) -> MResult<usize> {
        let Some(runtime) = self.runtime.as_mut() else {
            return Ok(0);
        };
        let pending = runtime.pending_host_input_count()?;
        runtime
            .drain_host_inputs(pending)
            .map(|outcomes| outcomes.len())
    }

    fn symbols(&self, names: &[String]) -> MResult<Vec<(String, RuntimeValueSnapshot)>> {
        let Some(runtime) = self.runtime.as_ref() else {
            return Ok(Vec::new());
        };
        if names.is_empty() {
            runtime.root_symbol_values_all()
        } else {
            runtime.root_symbol_values(&names.iter().map(String::as_str).collect::<Vec<_>>())
        }
    }

    fn plan(&self) -> String {
        match self.runtime.as_ref() {
            Some(runtime) => {
                let info = runtime.program_execution_info();
                format!(
                    "resident plan: {} nodes, {:?}, {} accepted turns",
                    runtime.root_plan_len(),
                    info.route,
                    info.resident_accepted_turns
                )
            }
            None => "No resident program is active.".to_string(),
        }
    }

    fn step(&mut self, count: u64) -> MResult<Vec<(String, RuntimeValueSnapshot)>> {
        let runtime = self.runtime.as_mut().ok_or_else(|| {
            MechError::new(
                mech_core::GenericError {
                    msg: "no resident program is active".to_string(),
                },
                None,
            )
        })?;
        for _ in 0..count {
            runtime.step_active_program()?;
        }
        runtime.root_symbol_values_all()
    }

    fn shutdown(&mut self) -> MResult<()> {
        if let Some(mut runtime) = self.runtime.take() {
            runtime.shutdown()?;
        }
        Ok(())
    }
}

/// Starts the targetless resident REPL used by an argument-less `mech` command.
pub(crate) fn run() -> MResult<CliOutcome> {
    #[cfg(windows)]
    colored::control::set_virtual_terminal(true).map_err(|_| {
        MechError::new(
            mech_core::GenericError {
                msg: "failed to enable Windows virtual terminal processing".to_string(),
            },
            None,
        )
    })?;

    // The product REPL owns a styled terminal surface. Do not let shell or
    // pseudo-terminal detection silently turn that surface monochrome.
    colored::control::set_override(true);
    let mut stdout = io::stdout().lock();
    write!(stdout, "\x1B[2J\x1B[H")?;
    print_banner(&mut stdout)?;

    let mut repl = ResidentRepl::new()?;
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
    );
    let worker_result = finish_input_worker(worker);
    let shutdown_result = repl.shutdown();

    if exit_requested.load(Ordering::Acquire) {
        print_interrupt_farewell(&mut stdout)?;
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
) -> MResult<()> {
    repl.start_input_drivers()?;
    print_prompt(output)?;
    let mut rendered_interrupts = 0;

    loop {
        render_pending_interrupts(output, interrupt_count, &mut rendered_interrupts)?;
        if exit_requested.load(Ordering::Acquire) {
            return Ok(());
        }
        match worker.input.recv_timeout(INPUT_POLL) {
            Ok(ReplInput::Line(line)) => {
                repl.drain_all_pending_inputs()?;
                if matches!(process_input(repl, &line, output)?, ReplControl::Quit) {
                    print_interrupt_farewell(output)?;
                    return Ok(());
                }
                repl.start_input_drivers()?;
                let _ = worker.resume.send(());
                print_prompt(output)?;
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
) -> io::Result<()> {
    let pending = interrupt_count.load(Ordering::Acquire).min(3);
    while *rendered_interrupts < pending {
        writeln!(output, "{}", ":ctrl+c".bright_yellow())?;
        *rendered_interrupts += 1;
        if *rendered_interrupts < 3 {
            writeln!(
                output,
                "\n{} {}Enter {} to terminate this REPL session.{}\n",
                "╭◉─".truecolor(246, 192, 78),
                "⸢".bright_yellow(),
                ":quit".bright_yellow(),
                "⸥".bright_yellow(),
            )?;
            print_prompt(output)?;
        }
    }
    Ok(())
}

fn run_with_io<R: BufRead, W: Write>(mut input: R, mut output: W) -> MResult<CliOutcome> {
    print_banner(&mut output)?;
    let mut repl = ResidentRepl::new()?;
    loop {
        print_prompt(&mut output)?;
        let mut entry = String::new();
        if input.read_line(&mut entry)? == 0 {
            repl.shutdown()?;
            return Ok(CliOutcome::exit(0));
        }
        if matches!(
            process_input(&mut repl, &entry, &mut output)?,
            ReplControl::Quit
        ) {
            print_farewell(&mut output)?;
            repl.shutdown()?;
            return Ok(CliOutcome::exit(0));
        }
    }
}

fn process_input(
    repl: &mut ResidentRepl,
    input: &str,
    output: &mut dyn Write,
) -> MResult<ReplControl> {
    if input.trim().is_empty() {
        return Ok(ReplControl::Continue);
    }
    if input.trim_start().starts_with(':') {
        match parse_command(input) {
            Some(ReplCommand::Quit) => return Ok(ReplControl::Quit),
            Some(command) => execute_command(repl, command, output)?,
            None => writeln!(
                output,
                "{} Unrecognized command.",
                "[Error]".truecolor(246, 98, 78)
            )?,
        }
    } else {
        match repl.submit(input) {
            Ok(value) => print_value(output, &value)?,
            Err(error) => render_error(output, &error)?,
        }
    }
    Ok(ReplControl::Continue)
}

fn parse_command(input: &str) -> Option<ReplCommand> {
    let command = input.trim();
    let body = command.strip_prefix(':')?;
    let (name, arguments) = body.split_once(char::is_whitespace).unwrap_or((body, ""));
    let arguments = arguments.trim();
    match name {
        "help" | "h" if arguments.is_empty() => Some(ReplCommand::Help),
        "quit" | "exit" | "q" if arguments.is_empty() => Some(ReplCommand::Quit),
        "clear" | "reset" if arguments.is_empty() => Some(ReplCommand::Clear),
        "clc" if arguments.is_empty() => Some(ReplCommand::Clc),
        "ls" if arguments.is_empty() => Some(ReplCommand::Ls),
        "plan" | "p" if arguments.is_empty() => Some(ReplCommand::Plan),
        "docs" | "d" => Some(ReplCommand::Docs(
            (!arguments.is_empty()).then(|| arguments.to_string()),
        )),
        "symbols" | "s" => Some(ReplCommand::Symbols(
            (!arguments.is_empty()).then(|| arguments.to_string()),
        )),
        "whos" | "w" => Some(ReplCommand::Whos(
            arguments.split_whitespace().map(str::to_string).collect(),
        )),
        "load" if !arguments.is_empty() => Some(ReplCommand::Load(
            arguments.split_whitespace().map(str::to_string).collect(),
        )),
        "save" if !arguments.is_empty() => Some(ReplCommand::Save(arguments.to_string())),
        "cd" if !arguments.is_empty() => Some(ReplCommand::Cd(arguments.to_string())),
        "code" | "c" => Some(ReplCommand::Code(arguments.to_string())),
        "profile" => match arguments {
            "on" => Some(ReplCommand::Profile(true)),
            "off" => Some(ReplCommand::Profile(false)),
            _ => None,
        },
        "step" => parse_step(arguments),
        _ => None,
    }
}

fn parse_step(arguments: &str) -> Option<ReplCommand> {
    if arguments.is_empty() {
        return Some(ReplCommand::Step(None, None));
    }
    let mut pieces = arguments.split_whitespace();
    let first = pieces.next()?;
    if let Some(id) = first.strip_prefix('#') {
        let id = id.parse().ok()?;
        let count = pieces.next().map(str::parse).transpose().ok()?;
        if pieces.next().is_none() {
            return Some(ReplCommand::Step(Some(id), count));
        }
        return None;
    }
    let count = first.parse().ok()?;
    (pieces.next().is_none()).then_some(ReplCommand::Step(None, Some(count)))
}

fn execute_command(
    repl: &mut ResidentRepl,
    command: ReplCommand,
    output: &mut dyn Write,
) -> MResult<()> {
    match command {
        ReplCommand::Help => writeln!(output, "{}", help())?,
        ReplCommand::Docs(name) => writeln!(output, "{}", docs(name))?,
        ReplCommand::Code(source) => match repl.submit(&source) {
            Ok(value) => print_value(output, &value)?,
            Err(error) => render_error(output, &error)?,
        },
        ReplCommand::Ls => writeln!(output, "{}", list_current_directory()?)?,
        ReplCommand::Cd(path) => {
            env::set_current_dir(Path::new(&path))?;
            writeln!(output, "{}", env::current_dir()?.display())?;
        }
        ReplCommand::Save(path) => {
            fs::write(&path, &repl.source)?;
            writeln!(output, "Saved session source to {path}")?;
        }
        ReplCommand::Load(paths) => match repl.load(&paths) {
            Ok(value) => print_value(output, &value)?,
            Err(error) => render_error(output, &error)?,
        },
        ReplCommand::Whos(names) => print_symbols(output, repl.symbols(&names)?)?,
        ReplCommand::Symbols(name) => {
            let names = name.into_iter().collect::<Vec<_>>();
            print_symbols(output, repl.symbols(&names)?)?;
        }
        ReplCommand::Plan => writeln!(output, "{}", repl.plan())?,
        ReplCommand::Profile(enabled) => {
            writeln!(
                output,
                "Profiling {} was requested, but resident profiling is not yet exposed by the runtime.",
                if enabled { "enabled" } else { "disabled" }
            )?;
        }
        ReplCommand::Step(step_id, count) => {
            if let Some(step_id) = step_id {
                writeln!(
                    output,
                    "Stepping resident program; step selector #{step_id} is informational."
                )?;
            }
            print_symbols(output, repl.step(count.unwrap_or(1))?)?;
        }
        ReplCommand::Clear => {
            repl.reset()?;
            writeln!(output, "Session source cleared.")?;
        }
        ReplCommand::Clc => write!(output, "\x1B[2J\x1B[H")?,
        ReplCommand::Quit => unreachable!("quit is handled before execution"),
    }
    Ok(())
}

fn print_banner(output: &mut dyn Write) -> io::Result<()> {
    let greeting = repl_greeting_at(Local::now().naive_local());
    writeln!(
        output,
        "{}",
        TEXT_LOGO.truecolor(MECH_AMBER.0, MECH_AMBER.1, MECH_AMBER.2)
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

fn print_prompt(output: &mut dyn Write) -> io::Result<()> {
    write!(
        output,
        "{}",
        PROMPT.truecolor(MECH_AMBER.0, MECH_AMBER.1, MECH_AMBER.2)
    )?;
    output.flush()
}

fn print_farewell(output: &mut dyn Write) -> io::Result<()> {
    writeln!(
        output,
        "{} {}Okay cya!{}",
        "╭◉╮".truecolor(246, 192, 78),
        "⸢".bright_yellow(),
        "⸥".bright_yellow()
    )
}

fn print_interrupt_farewell(output: &mut dyn Write) -> io::Result<()> {
    output.flush()?;
    #[cfg(feature = "mika")]
    {
        if io::stderr().is_terminal() {
            play_mika_farewell(
                ProgressDrawTarget::stderr(),
                format!("{}Okay cya!{}\n", "⸢".bright_yellow(), "⸥".bright_yellow()),
                Duration::from_millis(100),
            );
            Ok(())
        } else {
            // Progress bars deliberately suppress themselves when redirected.
            // Keep the farewell in CI logs, pipes, and captured transcripts.
            print_farewell(output)
        }
    }
    #[cfg(not(feature = "mika"))]
    {
        print_farewell(output)
    }
}

fn print_value(output: &mut dyn Write, value: &RuntimeValueSnapshot) -> io::Result<()> {
    if !value.is_empty() {
        writeln!(
            output,
            "\n{}\n{}",
            value.kind().to_string().ansi_color(218),
            value
        )?;
    }
    Ok(())
}

fn print_symbols(
    output: &mut dyn Write,
    rows: Vec<(String, RuntimeValueSnapshot)>,
) -> io::Result<()> {
    for (name, value) in rows {
        writeln!(output, "{name} = {value}")?;
    }
    Ok(())
}

fn render_error(output: &mut dyn Write, error: &MechError) -> io::Result<()> {
    writeln!(output, "{} {error:?}", "[Error]".truecolor(246, 98, 78))
}

fn list_current_directory() -> io::Result<String> {
    let mut entries = fs::read_dir(env::current_dir()?)?
        .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();
    Ok(entries.join("\n"))
}

fn docs(name: Option<String>) -> String {
    let Some(name) = name else {
        return "Enter a document name to search.".to_string();
    };
    let glob = format!("*{name}*");
    match DOCS_DIR.find(&glob) {
        Ok(entries) => entries
            .filter_map(|entry| entry.as_file())
            .find_map(|file| file.contents_utf8().map(str::to_string))
            .unwrap_or_else(|| format!("No documentation found for {name}")),
        Err(error) => format!("Unable to search documentation: {error}"),
    }
}

fn help() -> &'static str {
    ":help              show this help\n\
     :docs <name>       search embedded documentation\n\
     :symbols [name]    show resident output symbols\n\
     :whos [names...]   show selected resident values\n\
     :plan              show the active resident execution plan\n\
     :step [count]      step a pure resident program\n\
     :load <paths...>   append source files to the session\n\
     :save <path>       save accepted session source\n\
     :code <source>     evaluate source without leaving the prompt\n\
     :ls | :cd <path>   inspect or change the working directory\n\
     :clear             clear the accepted resident session\n\
     :clc               clear the terminal\n\
     :profile on|off    toggle REPL profiling state\n\
     :quit              terminate this REPL session"
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
        let outcome = run_with_io(Cursor::new("1 + 1\n[1 1 2]\n:quit\n"), &mut output).unwrap();

        assert!(matches!(outcome, CliOutcome::Exit(0)));
        let output = String::from_utf8(output).unwrap();
        assert!(
            output.contains("www.mech-lang.org"),
            "missing REPL banner: {output}"
        );
        assert!(output.contains(":help"), "missing REPL greeting: {output}");
        assert!(output.contains("f64"), "missing scalar type: {output}");
        assert!(output.contains('2'), "missing scalar value: {output}");
        assert!(output.contains("Okay cya!"), "missing farewell: {output}");
    }

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
    fn command_parser_retains_the_original_command_surface() {
        assert_eq!(parse_command(":help"), Some(ReplCommand::Help));
        assert_eq!(
            parse_command(":w x y"),
            Some(ReplCommand::Whos(vec!["x".into(), "y".into()]))
        );
        assert_eq!(
            parse_command(":step #2 4"),
            Some(ReplCommand::Step(Some(2), Some(4)))
        );
        assert_eq!(
            parse_command(":profile on"),
            Some(ReplCommand::Profile(true))
        );
        assert_eq!(
            parse_command(":load one.mec two.mec"),
            Some(ReplCommand::Load(vec!["one.mec".into(), "two.mec".into()]))
        );
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

        render_pending_interrupts(&mut output, &interrupt_count, &mut rendered_interrupts).unwrap();

        let output = String::from_utf8(output).unwrap();
        assert_eq!(rendered_interrupts, 3);
        assert_eq!(output.matches(":ctrl+c").count(), 3);
        assert_eq!(output.matches("to terminate this REPL session.").count(), 2);
        assert_eq!(output.matches("\n\n").count(), 4);
        assert!(output.ends_with('\n'));
    }
}
