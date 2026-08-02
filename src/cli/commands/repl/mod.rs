use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
#[cfg(any(feature = "mika", feature = "run"))]
use std::thread;
#[cfg(feature = "run")]
use std::thread::JoinHandle;
#[cfg(any(feature = "mika", feature = "run"))]
use std::time::Duration;

use chrono::{Datelike, Local, NaiveDateTime, Timelike};
use colored::*;
#[cfg(feature = "run")]
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use crossterm::{ExecutableCommand, cursor, style::Print};
#[cfg(feature = "mika")]
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use mech_core::*;
use mech_engine::*;
#[cfg(feature = "mika")]
use mech_syntax::MICROMIKA_WAVE;
use mech_syntax::{ReplCommand, parse_repl_command};

use crate::cli::outcome::CliOutcome;
use crate::{MechRepl, ReplExecution, clc, generate_uuid, print_prompt};

pub(crate) const TEXT_LOGO: &str = r#"
  ┌─────────┐ ┌──────┐ ┌─┐ ┌──┐ ┌─┐  ┌─┐
  └───┐ ┌───┘ └──────┘ │ │ └┐ │ │ │  │ │
  ┌─┐ │ │ ┌─┐ ┌──────┐ │ │  └─┘ │ └─┐│ │
  │ │ │ │ │ │ │ ┌────┘ │ │  ┌─┐ │ ┌─┘│ │
  │ │ └─┘ │ │ │ └────┐ │ └──┘ │ │ │  │ │
  └─┘     └─┘ └──────┘ └──────┘ └─┘  └─┘"#;

#[cfg(feature = "mika")]
const MIKA_FAREWELL_TEMPLATE: &str = "\n{spinner:.yellow} {msg}";

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

    // `indicatif` uses the last tick string after a spinner is finished.
    // MICROMIKA_WAVE ends with a blank cleanup frame, so replace the
    // style before finishing to preserve Mika's resting face.
    let resting_face = MICROMIKA_WAVE[0];
    let finished_style = ProgressStyle::with_template(MIKA_FAREWELL_TEMPLATE)
        .unwrap_or_else(|_| ProgressStyle::default_spinner())
        .tick_strings(&[resting_face, resting_face]);

    final_state.set_style(finished_style);
    final_state.finish();
}

#[cfg(all(test, feature = "mika"))]
mod tests {
    use super::*;
    use indicatif::InMemoryTerm;

    #[test]
    fn mika_farewell_remains_visible_after_drop() {
        let terminal = InMemoryTerm::new(10, 80);
        let draw_target = ProgressDrawTarget::term_like(Box::new(terminal.clone()));

        play_mika_farewell(draw_target, "⸢Okay cya!⸥\n".to_string(), Duration::ZERO);

        // The helper has returned, so its ProgressBar has already been dropped.
        let contents = terminal.contents();
        let visible_lines = contents
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>();

        assert_eq!(
            visible_lines,
            vec!["╭◉╮ ⸢Okay cya!⸥"],
            "unexpected terminal contents after farewell spinner drop:\n{contents}",
        );
    }
}

pub(crate) struct ReplStartup {
    #[cfg(feature = "run")]
    pub runtime: Option<mech_runtime::MechRuntime>,
}

enum ReplLoopControl {
    Continue,
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReplInterruptDisposition {
    Continue,
    GracefulRuntimeExit,
    ImmediateProcessExit,
}

fn repl_interrupt_disposition(
    runtime_backed: bool,
    interrupt_count: usize,
) -> ReplInterruptDisposition {
    if interrupt_count < 3 {
        ReplInterruptDisposition::Continue
    } else if runtime_backed {
        ReplInterruptDisposition::GracefulRuntimeExit
    } else {
        ReplInterruptDisposition::ImmediateProcessExit
    }
}

#[cfg(feature = "mika")]
fn print_repl_farewell() {
    play_mika_farewell(
        ProgressDrawTarget::stderr(),
        format!("{}Okay cya!{}\n", "⸢".bright_yellow(), "⸥".bright_yellow(),),
        Duration::from_millis(100),
    );
}

#[cfg(not(feature = "mika"))]
fn print_repl_farewell() {
    println!("Okay cya!");
}

fn process_repl_input(
    repl: &mut MechRepl,
    input: String,
    output: &mut dyn FnMut(String),
) -> ReplLoopControl {
    if input.chars().next() == Some(':') {
        // Path-oriented REPL commands use a CRLF delimiter in the syntax
        // parser. Normalize terminal input so Unix LF input follows the same
        // command path as Windows console input.
        let command_input = input
            .strip_suffix("\r\n")
            .or_else(|| input.strip_suffix('\n'))
            .unwrap_or(input.as_str());
        let command_input = format!("{command_input}\r\n");
        match parse_repl_command(command_input.as_str()) {
            Ok((_, repl_command)) => match repl.execute_repl_command_control(repl_command) {
                Ok(ReplExecution::Output(value)) => output(value),
                Ok(ReplExecution::Quit) => return ReplLoopControl::Quit,
                Err(error) => output(format!("!{error:?}")),
            },
            Err(error) => output(format!(
                "{} Unrecognized command: {}",
                "[Error]".truecolor(246, 98, 78),
                error,
            )),
        }
    } else if input.trim().is_empty() {
        return ReplLoopControl::Continue;
    } else {
        let command = ReplCommand::Code(vec![("repl".to_string(), MechSourceCode::String(input))]);
        match repl.execute_repl_command_control(command) {
            Ok(ReplExecution::Output(value)) => output(value),
            Ok(ReplExecution::Quit) => return ReplLoopControl::Quit,
            Err(error) => output(format!("(x)> {error:#?}")),
        }
    }

    ReplLoopControl::Continue
}

#[cfg(feature = "run")]
#[derive(Debug)]
enum RuntimeReplInput {
    Line(String),
    EndOfInput,
    ReadFailed(String),
}

#[cfg(feature = "run")]
struct RuntimeReplInputWorker {
    input: Receiver<RuntimeReplInput>,
    resume: Sender<()>,
    handle: JoinHandle<()>,
    waiting_for_resume: Arc<AtomicBool>,
}

#[cfg(feature = "run")]
const RUNTIME_REPL_INPUT_POLL: Duration = Duration::from_millis(10);

#[cfg(feature = "run")]
const MAX_REPL_HOST_INPUTS_PER_TURN: usize = 64;

#[cfg(feature = "run")]
fn spawn_runtime_repl_input_worker() -> RuntimeReplInputWorker {
    let (sender, receiver) = crossbeam_channel::bounded(1);
    let (resume, wait_for_resume) = crossbeam_channel::bounded(0);
    let waiting_for_resume = Arc::new(AtomicBool::new(false));
    let worker_waiting_for_resume = waiting_for_resume.clone();
    let handle = thread::spawn(move || {
        loop {
            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(0) => {
                    let _ = sender.send(RuntimeReplInput::EndOfInput);
                    break;
                }
                Ok(_) => {
                    worker_waiting_for_resume.store(true, Ordering::Release);
                    if sender.send(RuntimeReplInput::Line(input)).is_err() {
                        worker_waiting_for_resume.store(false, Ordering::Release);
                        break;
                    }
                    if wait_for_resume.recv().is_err() {
                        worker_waiting_for_resume.store(false, Ordering::Release);
                        break;
                    }
                    worker_waiting_for_resume.store(false, Ordering::Release);
                }
                Err(error) => {
                    let _ = sender.send(RuntimeReplInput::ReadFailed(error.to_string()));
                    break;
                }
            }
        }
    });
    RuntimeReplInputWorker {
        input: receiver,
        resume,
        handle,
        waiting_for_resume,
    }
}

#[cfg(feature = "run")]
fn runtime_repl_input_error(message: impl Into<String>) -> MechError {
    MechError::new(
        GenericError {
            msg: message.into(),
        },
        None,
    )
    .with_compiler_loc()
}

#[cfg(feature = "run")]
fn run_runtime_repl_event_loop(
    repl: &mut MechRepl,
    input: &Receiver<RuntimeReplInput>,
    poll_interval: Duration,
    exit_requested: &AtomicBool,
    before_input: &mut dyn FnMut(),
    output: &mut dyn FnMut(String),
    after_command: &mut dyn FnMut(),
    after_idle_drain: &mut dyn FnMut(),
) -> MResult<CliOutcome> {
    let loop_result = (|| {
        if exit_requested.load(Ordering::Acquire) {
            return Ok(CliOutcome::exit(0));
        }

        repl.start_runtime_input_drivers()?;

        before_input();
        loop {
            if exit_requested.load(Ordering::Acquire) {
                return Ok(CliOutcome::exit(0));
            }

            match input.recv_timeout(poll_interval) {
                Ok(RuntimeReplInput::Line(line)) => {
                    repl.drain_all_pending_runtime_host_inputs()?;
                    let control = process_repl_input(repl, line, output);
                    if matches!(control, ReplLoopControl::Quit) {
                        return Ok(CliOutcome::exit(0));
                    }

                    repl.start_runtime_input_drivers()?;
                    before_input();
                    after_command();
                }
                Ok(RuntimeReplInput::EndOfInput) => return Ok(CliOutcome::exit(0)),
                Ok(RuntimeReplInput::ReadFailed(error)) => {
                    return Err(runtime_repl_input_error(format!(
                        "failed to read runtime REPL input: {error}",
                    )));
                }
                Err(RecvTimeoutError::Timeout) => {
                    repl.drain_runtime_host_inputs(MAX_REPL_HOST_INPUTS_PER_TURN)?;
                    after_idle_drain();
                }
                Err(RecvTimeoutError::Disconnected) => return Ok(CliOutcome::exit(0)),
            }
        }
    })();
    let shutdown_result = repl.shutdown_runtime();

    match (loop_result, shutdown_result) {
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Ok(outcome), Ok(())) => Ok(outcome),
    }
}

#[cfg(feature = "run")]
fn finalize_runtime_repl_outcome(
    interrupt_requested: &AtomicBool,
    result: MResult<CliOutcome>,
    farewell: impl FnOnce(),
) -> MResult<CliOutcome> {
    // The CLI turns a returned exit outcome into `process::exit`. Complete the
    // farewell synchronously so process termination cannot cut off its frames.
    if interrupt_requested.load(Ordering::Acquire) && matches!(&result, Ok(CliOutcome::Exit(0))) {
        farewell();
    }

    result
}

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

pub(crate) fn run(startup: ReplStartup) -> MResult<CliOutcome> {
    let text_logo = TEXT_LOGO.truecolor(246, 192, 78);
    let greeting = repl_greeting_at(Local::now().naive_local());
    let micromika = greeting.mika().truecolor(246, 192, 78);
    let micromika_point = "╭◉─".truecolor(246, 192, 78);
    let help_cmd = ":help".bright_yellow();
    let quit_cmd = ":quit".bright_yellow();
    let ctrlc_cmd = ":ctrl+c".bright_yellow();
    let mika_open = "⸢".bright_yellow();
    let mika_close = "⸥".bright_yellow();

    #[cfg(windows)]
    control::set_virtual_terminal(true)
        .map_err(|_| io::Error::other("failed to enable Windows virtual terminal processing"))?;
    clc();
    let mut stdo = std::io::stdout();
    stdo.execute(Print(text_logo))?;
    stdo.execute(cursor::MoveToNextLine(1))?;
    println!(
        "\n                {}                ",
        format!("v{}", env!("CARGO_PKG_VERSION")).truecolor(246, 192, 78)
    );
    println!("           {}           \n", "www.mech-lang.org");
    let interjection = greeting
        .interjection()
        .map(|text| format!("{} ", text.bright_magenta()))
        .unwrap_or_default();
    let intro_message = format!(
        "{}{}Enter {} for a list of all commands.{}\n",
        mika_open, interjection, help_cmd, mika_close
    );
    println!("{} {}", micromika, intro_message);

    let runtime_backed_startup = {
        #[cfg(feature = "run")]
        {
            startup.runtime.is_some()
        }

        #[cfg(not(feature = "run"))]
        {
            false
        }
    };
    let runtime_exit_requested = Arc::new(AtomicBool::new(false));
    let caught_interrupts = Arc::new(Mutex::new(0));
    let ci = caught_interrupts.clone();
    let handler_exit_requested = runtime_exit_requested.clone();
    ctrlc::set_handler(move || {
        println!("{}", ctrlc_cmd);
        let interrupt_count = {
            let mut caught_interrupts = match ci.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };

            *caught_interrupts += 1;
            *caught_interrupts
        };

        match repl_interrupt_disposition(runtime_backed_startup, interrupt_count) {
            ReplInterruptDisposition::Continue => {
                println!(
                    "\n{} {}Enter {} to terminate this REPL session.{}\n",
                    micromika_point, mika_open, quit_cmd, mika_close
                );
                print_prompt();
            }
            ReplInterruptDisposition::GracefulRuntimeExit => {
                handler_exit_requested.store(true, Ordering::Release);
            }
            ReplInterruptDisposition::ImmediateProcessExit => {
                print_repl_farewell();
                std::process::exit(0);
            }
        }
    })
    .map_err(|error| {
        MechError::new(
            GenericError {
                msg: format!("Error setting Ctrl+C handler: {error}"),
            },
            None,
        )
        .with_compiler_loc()
    })?;

    #[cfg(all(feature = "repl", feature = "run"))]
    let mut repl = match startup.runtime {
        Some(runtime) => MechRepl::from_runtime(runtime),
        None => MechRepl::from(MechProgram::with_function_catalog(
            MechProgramConfig {
                name: format!("repl-{}", generate_uuid()),
                environment: MechProgramEnvironment::default(),
            },
            mech_stdlib::source_catalog(),
        )),
    };

    #[cfg(all(feature = "repl", not(feature = "run")))]
    let mut repl = MechRepl::from(MechProgram::with_function_catalog(
        MechProgramConfig {
            name: format!("repl-{}", generate_uuid()),
            environment: MechProgramEnvironment::default(),
        },
        mech_stdlib::source_catalog(),
    ));

    #[cfg(feature = "run")]
    if repl.is_runtime_backed() {
        let RuntimeReplInputWorker {
            input,
            resume,
            handle,
            waiting_for_resume,
        } = spawn_runtime_repl_input_worker();
        let mut before_input = || {
            if let Ok(mut interrupts) = caught_interrupts.lock() {
                *interrupts = 0;
            }
            print_prompt();
        };
        let mut output = |value: String| println!("{value}");
        let mut after_idle_drain = || {};
        let loop_result = {
            let mut after_command = || {
                let _ = resume.send(());
            };
            run_runtime_repl_event_loop(
                &mut repl,
                &input,
                RUNTIME_REPL_INPUT_POLL,
                runtime_exit_requested.as_ref(),
                &mut before_input,
                &mut output,
                &mut after_command,
                &mut after_idle_drain,
            )
        };
        let worker_was_waiting = waiting_for_resume.load(Ordering::Acquire);
        drop(resume);
        let worker_result = if worker_was_waiting || handle.is_finished() {
            handle
                .join()
                .map_err(|_| runtime_repl_input_error("runtime REPL input worker panicked"))
        } else {
            // A portable blocking stdin read cannot be cancelled. On an
            // idle-loop failure, detach that read rather than blocking
            // runtime cleanup; normal submitted-line and :quit paths join.
            Ok(())
        };
        let result = match (loop_result, worker_result) {
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
            (Ok(outcome), Ok(())) => Ok(outcome),
        };
        return finalize_runtime_repl_outcome(
            runtime_exit_requested.as_ref(),
            result,
            print_repl_farewell,
        );
    }

    loop {
        {
            if let Ok(mut ci) = caught_interrupts.lock() {
                *ci = 0;
            }
        }
        print_prompt();
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let mut output = |value: String| println!("{value}");
        if matches!(
            process_repl_input(&mut repl, input, &mut output),
            ReplLoopControl::Quit
        ) {
            return Ok(CliOutcome::exit(0));
        }
    }
}

#[cfg(all(test, feature = "run"))]
#[path = "tests/runtime_live.rs"]
mod runtime_live_tests;
