use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
#[cfg(feature = "mika")]
use std::thread;
#[cfg(feature = "mika")]
use std::time::Duration;

use colored::*;
use crossterm::{ExecutableCommand, cursor, style::Print};
#[cfg(feature = "mika")]
use indicatif::{ProgressBar, ProgressStyle};
use mech_core::*;
use mech_program::*;
#[cfg(feature = "run")]
use mech_runtime::RuntimeConfig;
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

pub(crate) struct ReplStartup {
    #[cfg(feature = "run")]
    pub runtime_config: Option<RuntimeConfig>,
    #[cfg(all(feature = "run", feature = "repl"))]
    pub seed_program: Option<MechProgram>,
}

pub(crate) fn run(startup: ReplStartup) -> MResult<CliOutcome> {
    let text_logo = TEXT_LOGO.truecolor(246, 192, 78);
    let micromika = "╭◉╮".truecolor(246, 192, 78);
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
    let intro_message = format!(
        "{}Enter {} for a list of all commands.{}\n",
        mika_open, help_cmd, mika_close
    );
    println!("{} {}", micromika, intro_message);

    let caught_interrupts = Arc::new(Mutex::new(0));
    let exit_requested = Arc::new(AtomicBool::new(false));
    let ci = caught_interrupts.clone();
    let exit_requested_for_handler = exit_requested.clone();
    ctrlc::set_handler(move || {
        println!("{}", ctrlc_cmd);
        let Ok(mut caught_interrupts) = ci.lock() else {
            exit_requested_for_handler.store(true, Ordering::SeqCst);
            return;
        };
        *caught_interrupts += 1;
        if *caught_interrupts >= 3 {
            #[cfg(feature = "mika")]
            {
                let final_state = ProgressBar::new_spinner();
                let completed_style = ProgressStyle::with_template("\n{spinner:.yellow} {msg}")
                    .unwrap_or_else(|_| ProgressStyle::default_spinner())
                    .tick_strings(MICROMIKA_WAVE);
                final_state.set_style(completed_style);
                final_state.set_message(format!("{}Okay cya!{}\n", mika_open, mika_close));

                for _ in 0..MICROMIKA_WAVE.len().saturating_sub(1) {
                    thread::sleep(Duration::from_millis(100));
                    final_state.tick();
                }
            }

            #[cfg(not(feature = "mika"))]
            println!("Okay cya!");

            exit_requested_for_handler.store(true, Ordering::SeqCst);
            return;
        }
        println!(
            "\n{} {}Enter {} to terminate this REPL session.{}\n",
            micromika_point, mika_open, quit_cmd, mika_close
        );
        print_prompt();
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
    let mut repl = {
        if let Some(program) = startup.seed_program {
            MechRepl::from(program)
        } else {
            let config = startup
                .runtime_config
                .unwrap_or_else(RuntimeConfig::default);
            let mut repl_program = MechProgram::new(MechProgramConfig {
                name: config.name.clone(),
                environment: MechProgramEnvironment::default(),
            });
            repl_program.configure(
                config.diagnostics.debug_enabled,
                config.diagnostics.trace_enabled,
                config.diagnostics.profile_enabled,
                config.limits.max_steps_per_turn.unwrap_or(10_000) as usize,
            );
            MechRepl::from(repl_program)
        }
    };

    #[cfg(all(feature = "repl", not(feature = "run")))]
    let mut repl = MechRepl::from(MechProgram::new(MechProgramConfig {
        name: format!("repl-{}", generate_uuid()),
        environment: MechProgramEnvironment::default(),
    }));

    loop {
        if exit_requested.load(Ordering::SeqCst) {
            return Ok(CliOutcome::exit(0));
        }
        {
            if let Ok(mut ci) = caught_interrupts.lock() {
                *ci = 0;
            }
        }
        print_prompt();
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if input.chars().next() == Some(':') {
            match parse_repl_command(input.as_str()) {
                Ok((_, repl_command)) => match repl.execute_repl_command_control(repl_command) {
                    Ok(ReplExecution::Output(output)) => println!("{}", output),
                    Ok(ReplExecution::Quit) => return Ok(CliOutcome::exit(0)),
                    Err(err) => println!("!{:?}", err),
                },
                Err(x) => println!(
                    "{} Unrecognized command: {}",
                    "[Error]".truecolor(246, 98, 78),
                    x
                ),
            }
        } else if input.trim().is_empty() {
            continue;
        } else {
            let cmd = ReplCommand::Code(vec![("repl".to_string(), MechSourceCode::String(input))]);
            match repl.execute_repl_command_control(cmd) {
                Ok(ReplExecution::Output(output)) => println!("{}", output),
                Ok(ReplExecution::Quit) => return Ok(CliOutcome::exit(0)),
                Err(err) => println!("(x)> {:#?}", err),
            }
        }
    }
}
