use mech_syntax::*;

/*
#[macro_use]
use nom::{
  IResult,
  branch::alt,
  sequence::tuple,
  combinator::opt,
  error::{context, convert_error, ErrorKind, ParseError, VerboseError},
  multi::{many1, many0},
  bytes::complete::{tag},
  character::complete::{alphanumeric1, alpha1, digit1, space0, space1},
};*/


#[derive(Debug, Clone)]
pub enum ReplCommand {
  Help,
  Quit,
  Pause,
  Resume,
  Stop,
  PrintCore(Option<u64>),
  PrintRuntime,
  Clear,
  Table(u64),
  Code(String),
  EchoCode(String),
  //ParsedCode(ParserNode),
  Empty,
  Error,
}

/*
fn mech_code(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  // Try parsing mech code
  let mut parser = Parser::new();
  match parser.parse_fragment(input) {
    Ok(_) => Ok((input, ReplCommand::Code(input.to_string()))),
    Err(_) => {
      // Try parsing it as an anonymous statement
      let command = format!("#ans = {}", input.trim());
      let mut parser = Parser::new();
      match parser.parse_fragment(&command) { 
        Ok(_) => Ok((input, ReplCommand::EchoCode(command.to_string()))),
        Err(_) => Ok((input, ReplCommand::Error)),
      }
    }
  }
}

fn clear(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  let (input, _) = tag("clear")(input)?;
  Ok((input, ReplCommand::Clear))
}

fn runtime(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  let (input, _) = tag("runtime")(input)?;
  Ok((input, ReplCommand::PrintRuntime))
}

fn core(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  let (input, _) = tag("core")(input)?;
  let (input, _) = space0(input)?;
  let (input, core_id) = opt(digit1)(input)?;
  let core_id = match core_id {
    Some(core_id) => Some(core_id.parse::<u64>().unwrap()),
    None => None,
  };
  Ok((input, ReplCommand::PrintCore(core_id)))
}

fn quit(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  let (input, _) = alt((tag("quit"),tag("exit")))(input)?;
  Ok((input, ReplCommand::Quit))
}

fn resume(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  let (input, _) = tag("resume")(input)?;
  Ok((input, ReplCommand::Resume))
}

fn pause(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  let (input, _) = tag("pause")(input)?;
  Ok((input, ReplCommand::Pause))
}

fn help(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  let (input, _) = tag("help")(input)?;
  Ok((input, ReplCommand::Help))
}

fn command(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  let (input, _) = tag(":")(input)?;
  let (input, command) = alt((quit, help, pause, resume, core, runtime, clear))(input)?;
  Ok((input, command))
}

pub fn parse_repl_command(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  let (input, command) = alt((command, mech_code))(input)?;
  Ok((input, command))
}*/