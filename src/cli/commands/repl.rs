use std::io;
use std::sync::{Arc, Mutex};
#[cfg(feature = "mika")]
use std::thread;
#[cfg(feature = "mika")]
use std::time::Duration;

use colored::*;
use crossterm::{ExecutableCommand, cursor, style::Print};
#[cfg(feature = "mika")]
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
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
    pub runtime_config: Option<RuntimeConfig>,
    #[cfg(all(feature = "run", feature = "repl"))]
    pub seed_bytecode: Option<Vec<u8>>,
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
    let ci = caught_interrupts.clone();
    ctrlc::set_handler(move || {
        println!("{}", ctrlc_cmd);
        let should_exit = {
            let mut caught_interrupts = match ci.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };

            *caught_interrupts += 1;
            *caught_interrupts >= 3
        };
        if should_exit {
            #[cfg(feature = "mika")]
            play_mika_farewell(
                ProgressDrawTarget::stderr(),
                format!("{}Okay cya!{}\n", mika_open, mika_close),
                Duration::from_millis(100),
            );

            #[cfg(not(feature = "mika"))]
            println!("Okay cya!");

            std::process::exit(0);
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
        let config = startup
            .runtime_config
            .unwrap_or_else(RuntimeConfig::default);
        config.validate()?;
        let mut repl_program = MechProgram::new(MechProgramConfig {
            name: config.name.clone(),
            environment: MechProgramEnvironment::default(),
        });
        repl_program.configure(
            config.diagnostics.debug_enabled,
            config.diagnostics.trace_enabled,
            config.diagnostics.profile_enabled,
            config.limits.max_steps_per_turn_as_usize()?,
        );
        if let Some(bytecode) = startup.seed_bytecode {
            repl_program.run_bytecode(&bytecode)?;
        }
        MechRepl::from(repl_program)
    };

    #[cfg(all(feature = "repl", not(feature = "run")))]
    let mut repl = MechRepl::from(MechProgram::new(MechProgramConfig {
        name: format!("repl-{}", generate_uuid()),
        environment: MechProgramEnvironment::default(),
    }));

    loop {
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
