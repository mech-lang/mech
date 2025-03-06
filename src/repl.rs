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
//use crate::{minify_blocks, read_mech_files};

#[derive(Debug, Clone)]
pub enum ReplCommand {
  Help,
  Quit,
  //Pause,
  //Resume,
  //Stop,
  //Save(String),
  Code(Vec<(String,MechSourceCode)>),
  Ls,
  Cd(String),
  Step(Option<usize>),
  Load(Vec<String>),
  Whos(Option<String>),
  Plan,
  Symbols(Option<String>),
  Clear(Option<String>),
  Clc,
  //Error(String),
  //Empty,
}

pub struct MechRepl {
  pub active: u64,
  pub interpreters: HashMap<u64,Interpreter>,
}

impl MechRepl {

  pub fn new() -> MechRepl {
    let intrp = Interpreter::new();
    let intrp_id = generate_uuid();
    let mut interpreters = HashMap::new();
    interpreters.insert(intrp_id,intrp);
    MechRepl {
      active: intrp_id,
      interpreters,
    }
  }

  pub fn from(interpreter: Interpreter) -> MechRepl {
    let intrp_id = generate_uuid();
    let mut interpreters = HashMap::new();
    interpreters.insert(intrp_id,interpreter);
    MechRepl {
      active: intrp_id,
      interpreters,
    }
  }

  pub fn execute_repl_command(&mut self, repl_cmd: ReplCommand) -> MResult<String> {

    let mut intrp = self.interpreters.get_mut(&self.active).unwrap();

    match repl_cmd {
      ReplCommand::Help => {
        return Ok(help());
      }
      ReplCommand::Quit => {
        // exit from the program
        process::exit(0);
      }
      ReplCommand::Symbols(name) => {return Ok(pretty_print_symbols(&intrp));}
      ReplCommand::Plan => {return Ok(pretty_print_plan(&intrp));}
      ReplCommand::Whos(name) => {return Ok(whos(&intrp));}
      ReplCommand::Clear(name) => {
        // Drop the old interpreter replace it with a new one
        *intrp = Interpreter::new();
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
      ReplCommand::Clc => {
        clc();
        Ok("".to_string())
      },
      ReplCommand::Load(paths) => {
        /*let mut mechfs = MechFileSystem::new();
        let code = mechfs.read_mech_files(&paths)?;
        match run_mech_code(&mut intrp, &code, false,false,false) {
          Ok(r) => {return Ok(format!("\n{:?}\n{}\n", r.kind(), r.pretty_print()));},
          Err(err) => {return Err(err);}
        }*/
        todo!()
      }
      ReplCommand::Code(code) => {
        match run_mech_code(&mut intrp, &code, false,false,false)  {
          Ok(r) => { return Ok(format!("\n{:?}\n{}\n", r.kind(), r.pretty_print()));},
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

  pub fn parse_repl_command(input: &str) -> IResult<&str, ReplCommand> {
    let (input, _) = tag(":")(input)?;
    let (input, command) = alt((
      MechRepl::step_rpl,
      //MechRepl::save_rpl,
      MechRepl::help_rpl,
      MechRepl::quit_rpl,
      MechRepl::symbols_rpl,
      MechRepl::plan_rpl,
      MechRepl::ls_rpl,
      MechRepl::cd_rpl,
      MechRepl::whos_rpl,
      MechRepl::clear_rpl,
      MechRepl::clc_rpl,
      MechRepl::load_rpl,
    ))(input)?;
    Ok((input, command))
  }

  fn help_rpl(input: &str) -> IResult<&str, ReplCommand> {
    let (input, _) = alt((tag("h"), tag("help")))(input)?;
    Ok((input, ReplCommand::Help))
  }

  fn quit_rpl(input: &str) -> IResult<&str, ReplCommand> {
    let (input, _) = alt((tag("q"), tag("quit"), tag("exit")))(input)?;
    Ok((input, ReplCommand::Quit))
  }

  fn cd_rpl(input: &str) -> IResult<&str, ReplCommand> {
    let (input, _) = tag("cd")(input)?;
    let (input, _) = space0(input)?;
    let (input, path) = take_until("\r\n")(input)?;
    Ok((input, ReplCommand::Cd(path.to_string())))
  }

  fn symbols_rpl(input: &str) -> IResult<&str, ReplCommand> {
    let (input, _) = alt((tag("s"), tag("symbols")))(input)?;
    let (input, _) = space0(input)?;
    let (input, name) = opt(take_while(|c: char| c.is_alphanumeric()))(input)?;
    Ok((input, ReplCommand::Symbols(name.map(|s| s.to_string()))))
  }

  fn plan_rpl(input: &str) -> IResult<&str, ReplCommand> {
    let (input, _) = alt((tag("p"), tag("plan")))(input)?;
    Ok((input, ReplCommand::Plan))
  }

  fn whos_rpl(input: &str) -> IResult<&str, ReplCommand> {
    let (input, _) = alt((tag("w"), tag("whos")))(input)?;
    let (input, _) = space0(input)?;
    let (input, name) = opt(take_while(|c: char| c.is_alphanumeric()))(input)?;
    Ok((input, ReplCommand::Whos(name.map(|s| s.to_string()))))
  }

  fn clear_rpl(input: &str) -> IResult<&str, ReplCommand> {
    let (input, _) = tag("clear")(input)?;
    Ok((input, ReplCommand::Clear(None)))
  }

  fn clc_rpl(input: &str) -> IResult<&str, ReplCommand> {
    let (input, _) = alt((tag("c"), tag("clc")))(input)?;
    Ok((input, ReplCommand::Clc))
  }

  fn ls_rpl(input: &str) -> IResult<&str, ReplCommand> {
    let (input, _) = tag("ls")(input)?;
    Ok((input, ReplCommand::Ls))
  }

  fn load_rpl(input: &str) -> IResult<&str, ReplCommand> {
    let (input, _) = tag("load")(input)?;
    let (input, _) = space1(input)?;
    let (input, path_strings) = separated_list1(space1, alt((take_until(" "),take_until("\r\n"))))(input)?;
    Ok((input, ReplCommand::Load(path_strings.iter().map(|s| s.to_string()).collect())))
  }

  /*fn save_rpl(input: &str) -> IResult<&str, ReplCommand> {
    let (input, _) = tag("save")(input)?;
    let (input, _) = space1(input)?;
    let (input, path) = take_while(|c: char| c.is_alphanumeric())(input)?;
    Ok((input, ReplCommand::Save(path.to_string())))
  }*/

  fn step_rpl(input: &str) -> IResult<&str, ReplCommand> {
    let (input, _) = tag("step")(input)?;
    let (input, _) = space0(input)?;
    let (input, count) = opt(digit1)(input)?;
    Ok((input, ReplCommand::Step(count.map(|s| s.parse().unwrap()))))
  }


}