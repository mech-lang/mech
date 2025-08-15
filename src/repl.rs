use crate::*;
use mech_syntax::*;
use mech_core::*;
use mech_interpreter::*;
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

static DOCS_DIR: Dir = include_dir!("docs");
static EXAMPLES_DIR: Dir = include_dir!("examples/working");

pub struct MechRepl {
  pub docs: Dir<'static>,
  pub examples: Dir<'static>,
  pub active: u64,
  pub interpreters: HashMap<u64,Interpreter>,
}

impl MechRepl {

  pub fn new() -> MechRepl {
    let intrp_id = generate_uuid();
    let intrp = Interpreter::new(intrp_id);
    let mut interpreters = HashMap::new();
    interpreters.insert(intrp_id,intrp);
    MechRepl {
      active: intrp_id,
      interpreters,
      docs: DOCS_DIR.clone(),
      examples: EXAMPLES_DIR.clone(),
    }
  }

  pub fn from(interpreter: Interpreter) -> MechRepl {
    let intrp_id = generate_uuid();
    let mut interpreters = HashMap::new();
    interpreters.insert(intrp_id,interpreter);
    MechRepl {
      docs: DOCS_DIR.clone(),
      examples: EXAMPLES_DIR.clone(),
      active: intrp_id,
      interpreters,
    }
  }

  pub fn execute_repl_command(&mut self, repl_cmd: ReplCommand) -> MResult<String> {

    let mut intrp = self.interpreters.get_mut(&self.active).unwrap();
    let mut mechfs = MechFileSystem::new();

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
      ReplCommand::Symbols(name) => {return Ok(pretty_print_symbols(&intrp));}
      ReplCommand::Plan => {return Ok(pretty_print_plan(&intrp));}
      ReplCommand::Whos(names) => {return Ok(whos(&intrp,names));}
      ReplCommand::Clear(name) => {
        // Drop the old interpreter replace it with a new one
        let id = intrp.id;
        *intrp = Interpreter::new(id);
        return Ok("".to_string());
      }
      ReplCommand::Ls => {
        return Ok(ls());
      }
      ReplCommand::Cd(path) => {
        let path = PathBuf::from(path);
        env::set_current_dir(&path).unwrap();
        return Ok("".to_string());
      }
      ReplCommand::Save(path) => {
        let path = PathBuf::from(path);
        let intrp = self.interpreters.get(&self.active).unwrap();
        let encoded = encode_to_vec(&MechSourceCode::Program(intrp.code.clone()), standard()).unwrap();
        let mut file = File::create(&path)?;
        file.write_all(&encoded)?;
        return Ok(format!("Saved interpreter state to {}", path.display()));
      }
      ReplCommand::Clc => {
        clc();
        Ok("".to_string())
      },
      ReplCommand::Load(paths) => {
        for source in paths {
          mechfs.watch_source(&source)?;
        }
        match run_mech_code(&mut intrp, &mechfs, false,false,false) {
          Ok(r) => {return Ok(format!("\n{}\n{}\n", r.kind(), r.pretty_print()));},
          Err(err) => {return Err(err);}
        }
      }
      ReplCommand::Code(code) => {
        for (_,src) in code {
          mechfs.add_code(&src)?;
        }
        match run_mech_code(&mut intrp, &mechfs, false,false,false)  {
          Ok(r) => { return Ok(format!("\n{}\n{}\n", r.kind(), r.pretty_print()));},
          Err(err) => { return Err(err); }
        }
      }
      ReplCommand::Step(count) => {
        let n = match count {
          Some(n) => n,
          None => 1,
        };
        let now = Instant::now();
        intrp.step(n as u64);
        let elapsed_time = now.elapsed();
        let cycle_duration = elapsed_time.as_nanos() as f64;
        return Ok(format!("{} cycles in {:0.2?} ns\n", n, cycle_duration));
      }
    }
  }

}