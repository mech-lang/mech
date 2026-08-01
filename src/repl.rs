use crate::*;
use mech_core::*;
use mech_program::{MechProgram, MechProgramConfig, MechProgramEnvironment};
use std::collections::HashMap;
use std::process;
use nom::{
  IResult,
  bytes::complete::tag,
  branch::alt,
  bytes::complete::{take_while, take_until},
  combinator::{opt, not},
  multi::separated_list1,
  character::complete::{space0,space1,digit1},
};

use bincode::serde::encode_to_vec;
use bincode::config::standard;
use include_dir::{include_dir, Dir};
use std::time::{Instant, Duration};

static DOCS_DIR: Dir = include_dir!("docs");
static EXAMPLES_DIR: Dir = include_dir!("examples/working");

pub struct MechRepl {
  pub docs: Dir<'static>,
  pub examples: Dir<'static>,
  pub active: u64,
  pub programs: HashMap<u64,MechProgram>,
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

  pub fn execute_repl_command(&mut self, repl_cmd: ReplCommand) -> MResult<String> {

    let mut prgrm = self.programs.get_mut(&self.active).unwrap();

    match repl_cmd {
      ReplCommand::Help => {
        return Ok(help());
      }
      ReplCommand::Quit => {
        // exit from the program
        process::exit(0);
      }
      ReplCommand::Docs(name) => {
        if let Some(name) = name {
          let glob = format!("*{}*",name);
          for entry in self.docs.find(&glob).unwrap() {
            println!("Found {}", entry.path().display());
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
        let out = prgrm.interpreter().pretty_print_symbols();
        #[cfg(not(feature = "pretty_print"))]
        let out = format!("{:#?}", prgrm.state.borrow().symbols());
        return Ok(out);
      }
      ReplCommand::Plan => {
        #[cfg(feature = "pretty_print")]
        let out = prgrm.interpreter().plan().pretty_print();
        #[cfg(not(feature = "pretty_print"))]
        let out = format!("{:#?}", prgrm.interpreter().plan());
        return Ok(out);
      }
      ReplCommand::Whos(names) => {return Ok(whos(prgrm,names));}
      ReplCommand::Clear(name) => {
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
              Err(e) => {
                return Err(MechError::new(PathNotFound{ file_path: path.display().to_string() }, None).with_compiler_loc());
              }
            }
          }
          Err(e) => {
            return Err(MechError::new(PathNotFound{ file_path: path.display().to_string() }, None).with_compiler_loc());
          }
        }
      }
      #[cfg(feature = "serde")]
      ReplCommand::Save(path) => {
        let path = PathBuf::from(path);
        let intrp = self.programs.get(&self.active).unwrap();
        let encoded = encode_to_vec(&MechSourceCode::String(format!("{:#?}", intrp.interpreter().plan())), standard()).unwrap();
        let mut file = File::create(&path)?;
        file.write_all(&encoded)?;
        return Ok(format!("Saved program state to {}", path.display()));
      }
      ReplCommand::Clc => {
        clc();
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
        return Ok(format!("\n{}\n{}\n", r.kind(), r));
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
        return Ok(format!("\n{}\n{}\n", kind_formatted, r));
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
      x => {
        return Err(MechError::new(FeatureNotEnabledError, None).with_compiler_loc());
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
