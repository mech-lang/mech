use crate::*;
use mech_syntax::ReplCommand;
use mech_program::{MechProgram, MechProgramConfig, MechProgramEnvironment};
use std::collections::HashMap;

use bincode::serde::encode_to_vec;
use bincode::config::standard;
use include_dir::{include_dir, Dir};
use std::time::{Instant, Duration};

static DOCS_DIR: Dir = include_dir!("docs");
static EXAMPLES_DIR: Dir = include_dir!("examples/working");


pub enum ReplExecution {
  Output(String),
  Quit,
}

pub struct MechRepl {
  pub docs: Dir<'static>,
  pub examples: Dir<'static>,
  pub active: u64,
  pub programs: HashMap<u64,MechProgram>,
}


fn repl_error(msg: impl Into<String>) -> MechError {
  MechError::new(GenericError { msg: msg.into() }, None).with_compiler_loc()
}

impl MechRepl {

  pub fn new() -> MechRepl {
    let intrp_id = generate_uuid();
    let program = MechProgram::new(MechProgramConfig{
      name: format!("repl-{}", intrp_id),
      environment: MechProgramEnvironment::default(),
    });
    let mut programs = HashMap::new();
    programs.insert(intrp_id,program);
    MechRepl {
      active: intrp_id,
      programs,
      docs: DOCS_DIR.clone(),
      examples: EXAMPLES_DIR.clone(),
    }
  }

  pub fn from(program: MechProgram) -> MechRepl {
    let intrp_id = generate_uuid();
    let mut programs = HashMap::new();
    programs.insert(intrp_id,program);
    MechRepl {
      docs: DOCS_DIR.clone(),
      examples: EXAMPLES_DIR.clone(),
      active: intrp_id,
      programs,
    }
  }

  pub fn execute_repl_command_control(&mut self, repl_cmd: ReplCommand) -> MResult<ReplExecution> {
    if matches!(repl_cmd, ReplCommand::Quit) {
      return Ok(ReplExecution::Quit);
    }
    self.execute_repl_command(repl_cmd).map(ReplExecution::Output)
  }

  pub fn execute_repl_command(&mut self, repl_cmd: ReplCommand) -> MResult<String> {

    let prgrm = self
      .programs
      .get_mut(&self.active)
      .ok_or_else(|| repl_error(format!("active REPL program not found: {}", self.active)))?;

    match repl_cmd {
      ReplCommand::Help => {
        return Ok(help());
      }
      ReplCommand::Quit => {
        return Ok(String::new());
      }
      ReplCommand::Docs(name) => {
        if let Some(name) = name {
          let glob = format!("*{}*",name);
          let entries = self
            .docs
            .find(&glob)
            .map_err(|error| repl_error(format!("failed to search documentation: {error}")))?;
          for entry in entries {
            // print out hte contents of hte file
            match entry.as_file() {
              Some(file) => {
                match file.contents_utf8() {
                  Some(doc_content) => {
                    return Ok(format!("{}", doc_content));
                  },
                  None => {
                    return Ok(format!("No documentation found for {}", name));
                  }
                }
              },
              None => {
                return Ok(format!("No documentation found for {}", name));
              }
            }
          }
          Ok(format!("No documentation found for {}", name))
        } else {
          Ok("Enter a doc to search for.".to_string())
        }
      }
      ReplCommand::Symbols(name) => {
        #[cfg(feature = "pretty_print")]
        let all_symbols = prgrm.interpreter().pretty_print_symbols();
        #[cfg(not(feature = "pretty_print"))]
        let all_symbols = format!("{:#?}", prgrm.interpreter().symbols());
        if let Some(name) = name {
          let matches = all_symbols.lines().filter(|line| line.contains(&name)).collect::<Vec<_>>();
          if matches.is_empty() { return Ok(format!("No symbols matched '{name}'.")); }
          return Ok(matches.join("\n"));
        }
        return Ok(all_symbols);
      }
      ReplCommand::Plan => {
        #[cfg(feature = "pretty_print")]
        let out = prgrm.interpreter().plan().pretty_print();
        #[cfg(not(feature = "pretty_print"))]
        let out = format!("{:#?}", prgrm.interpreter().plan());
        return Ok(out);
      }
      ReplCommand::Whos(names) => {
        #[cfg(feature = "whos")]
        {
          return Ok(whos(prgrm,names));
        }
        #[cfg(not(feature = "whos"))]
        {
          let _ = names;
          return Ok("The :whos command requires the whos feature.".to_string());
        }
      }
      ReplCommand::Clear => {
        // Drop the old program and replace it with a new one
        let id = self.active;
        *prgrm = MechProgram::new(MechProgramConfig{
          name: format!("repl-{}", id),
          environment: MechProgramEnvironment::default(),
        });
        return Ok("".to_string());
      }
      ReplCommand::Ls => {
        return Ok(ls());
      }
      ReplCommand::Cd(path) => {
        let path = PathBuf::from(path);
        match env::set_current_dir(&path) {
          Ok(_) => {
            match env::current_dir() {
              Ok(current_path) => {
                return Ok(format!("{}", current_path.display()));
              }
              Err(error) => {
                return Err(repl_error(format!("failed to read current directory after changing to {}: {error}", path.display())));
              }
            }
          }
          Err(error) => {
            return Err(repl_error(format!("failed to change directory to {}: {error}", path.display())));
          }
        }
      }
      #[cfg(feature = "serde")]
      ReplCommand::Save(path) => {
        let path = PathBuf::from(path);
        let intrp = self
          .programs
          .get(&self.active)
          .ok_or_else(|| repl_error(format!("active REPL program not found: {}", self.active)))?;
        let encoded = encode_to_vec(&MechSourceCode::String(format!("{:#?}", intrp.interpreter().plan())), standard())
          .map_err(|error| repl_error(format!("failed to encode REPL program state: {error}")))?;
        let mut file = File::create(&path)?;
        file.write_all(&encoded)?;
        return Ok(format!("Saved program state to {}", path.display()));
      }
      ReplCommand::Clc => {
        clc()?;
        Ok("".to_string())
      },
      ReplCommand::Load(paths) => {
        let mut result = Value::Empty;
        for source_path in paths {
          let source = std::fs::read_to_string(&source_path)?;
          result = prgrm.run_string(&source)?;
        }
        let r = result;
        #[cfg(feature = "pretty_print")]
        let out = r.pretty_print();
        #[cfg(not(feature = "pretty_print"))]
        let out = format!("{:#?}", r);
        return Ok(format!("\n{}\n{}\n", r.kind(), out));
      }
      ReplCommand::Code(code) => {
        let mut result = Value::Empty;
        for (_,src) in code {
          result = prgrm.run_string(&src.to_string())?;
        }
        let r = result;
        #[cfg(feature = "pretty_print")]
        let out = r.pretty_print();
        #[cfg(not(feature = "pretty_print"))]
        let out = format!("{:#?}", r);
        let kind_formatted = format!("{}", r.kind()).ansi_color(218);
        return Ok(format!("\n{}\n{}\n", kind_formatted, out));
      }
      ReplCommand::Profile(on) => {
        let _ = on;
        Ok("Profiling is not currently supported in Program.".to_string())
      }
      ReplCommand::Step(step_id, step_count) => {
        let n: u64 = match step_count {
          Some(n) => n,
          None => 1,
        };
        let step_id: usize = match step_id {
          Some(id) => id,
          None => 0,
        };
        let now = Instant::now();
        let _ = (step_id, n);
        let elapsed_time = now.elapsed();
        return Ok(format!("Stepping is not currently supported in Program ({})", format_cycles(1, elapsed_time)));      
      }
    }
  }

}

fn format_cycles(n: u64, total_duration: Duration) -> String {
  let total_ns = total_duration.as_nanos() as f64;
  let total_s = total_ns / 1_000_000_000.0;

  // Human-friendly total duration
  let formatted_total = if total_ns >= 1_000_000_000.0 {
    format!("{:.3} s", total_s)
  } else if total_ns >= 1_000_000.0 {
    format!("{:.3} ms", total_ns / 1_000_000.0)
  } else if total_ns >= 1_000.0 {
    format!("{:.3} µs", total_ns / 1_000.0)
  } else {
    format!("{:.3} ns", total_ns)
  };

  // Per-cycle duration
  let cycle_ns = total_ns / n as f64;
  let cycle_s = cycle_ns / 1_000_000_000.0;

  let formatted_cycle = if cycle_ns >= 1_000_000_000.0 {
    format!("{:.3} s", cycle_s)
  } else if cycle_ns >= 1_000_000.0 {
    format!("{:.3} ms", cycle_ns / 1_000_000.0)
  } else if cycle_ns >= 1_000.0 {
    format!("{:.3} µs", cycle_ns / 1_000.0)
  } else {
    format!("{:.3} ns", cycle_ns)
  };

  // Cycle frequency
  let freq_hz = 1.0 / cycle_s;
  let formatted_freq = if freq_hz >= 1_000_000_000.0 {
    format!("{:.3} GHz", freq_hz / 1_000_000_000.0)
  } else if freq_hz >= 1_000_000.0 {
    format!("{:.3} MHz", freq_hz / 1_000_000.0)
  } else if freq_hz >= 1_000.0 {
    format!("{:.3} kHz", freq_hz / 1_000.0)
  } else {
    format!("{:.3} Hz", freq_hz)
  };

  format!(
    "{} cycles in {} ({} per cycle, {})",
    n, formatted_total, formatted_cycle, formatted_freq
  )
}
