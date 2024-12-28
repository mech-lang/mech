use mech_syntax::*;
use nom::{
  IResult,
  bytes::complete::tag,
  branch::alt,
  bytes::complete::take_while,
  combinator::opt,
  character::complete::{space0,space1,digit1},
};
//use crate::{minify_blocks, read_mech_files};

#[derive(Debug, Clone)]
pub enum ReplCommand {
  Help,
  Quit,
  Pause,
  Resume,
  Stop,
  Step(Option<usize>),
  Load(String),
  Save(String),
  Whos(Option<String>),
  Plan,
  Symbols(Option<String>),
  Empty,
  Clear(Option<String>),
  Clc,
  Error(String),
}

pub fn parse_repl_command(input: &str) -> IResult<&str, ReplCommand> {
  let (input, _) = tag(":")(input)?;
  let (input, command) = alt((
    step_rpl,
    save_rpl,
    help_rpl,
    quit_rpl,
    symbols_rpl,
    plan_rpl,
    whos_rpl,
    clear_rpl,
    clc_rpl,
    load_rpl,
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
  let (input, _) = tag("clc")(input)?;
  Ok((input, ReplCommand::Clc))
}

fn load_rpl(input: &str) -> IResult<&str, ReplCommand> {
  let (input, _) = tag("load")(input)?;
  let (input, _) = space1(input)?;
  Ok((input, ReplCommand::Load(input.to_string())))
}

fn save_rpl(input: &str) -> IResult<&str, ReplCommand> {
  let (input, _) = tag("save")(input)?;
  let (input, _) = space1(input)?;
  let (input, path) = take_while(|c: char| c.is_alphanumeric())(input)?;
  Ok((input, ReplCommand::Save(path.to_string())))
}

fn step_rpl(input: &str) -> IResult<&str, ReplCommand> {
  let (input, _) = tag("step")(input)?;
  let (input, _) = space0(input)?;
  let (input, count) = opt(digit1)(input)?;
  Ok((input, ReplCommand::Step(count.map(|s| s.parse().unwrap()))))
}
