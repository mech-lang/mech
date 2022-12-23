use mech_syntax::*;
use mech_syntax::parser::*;
use mech_utilities::*;
use mech_core::*;
use mech_core::nodes::*;
use crate::minify_blocks;

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
};


#[derive(Debug, Clone)]
pub enum ReplCommand {
  Help,
  Quit,
  Pause,
  Resume,
  Stop,
  SaveCore(u64),
  PrintCore(Option<u64>),
  Clear,
  Table(u64),
  Code(MechCode),
  //ParsedCode(ParserNode),
  Empty,
  Error,
}

/*fn mech_code(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  // Try parsing mech code fragment
  let mut compiler = compiler::Compiler::new();
  match compiler.compile_fragment(input) {
    Ok(blocks) => {
      let mut mb = minify_blocks(&blocks);
      Ok((input, ReplCommand::Code(MechCode::MiniBlocks(mb))))
    },
    Err(_) => Ok((input, ReplCommand::Error)),
  }
}

fn clear(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  let (input, _) = tag("clear")(input)?;
  Ok((input, ReplCommand::Clear))
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

fn save(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  let (input, _) = tag("save")(input)?;
  let (input, _) = space0(input)?;
  let (input, core_id) = opt(digit1)(input)?;
  let core_id = match core_id {
    Some(core_id) => core_id.parse::<u64>().unwrap(),
    None => 0,
  };
  Ok((input, ReplCommand::SaveCore(core_id)))
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

fn load(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  let (input, _) = tag("load")(input)?;
  let (input, _) = space1(input)?;
  let (input, id) = syntax::parser::text(input).unwrap();
  Ok((input, ReplCommand::Code(MechCode::String("".to_string()))))
}

fn help(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  let (input, _) = tag("help")(input)?;
  Ok((input, ReplCommand::Help))
}

fn command(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  let (input, _) = tag(":")(input)?;
  let (input, command) = alt((quit, help, pause, resume, core, clear, save))(input)?;
  Ok((input, command))
}*/

pub fn help(input: ParseString) -> ParseResult<ReplCommand> {
  let (input,_) = syntax::parser::tag("help")(input)?;
  Ok((input,ReplCommand::Help))
}

pub fn parse_repl_command(input: ParseString) -> ParseResult<ReplCommand> {
  let (input,command) = help(input)?;
  Ok((input, command))
}

pub fn parse(text: &str) -> Result<ReplCommand, MechError> {
  let graphemes = graphemes::init_source(text);
  let mut result_node = ParserNode::Error;
  let mut error_log: Vec<(SourceRange, ParseErrorDetail)> = vec![];
  let remaining: ParseString;

  match parse_repl_command(ParseString::new(&graphemes)) {
    Ok(_) => {

    }
    Err(_)=> {

    }
  }


  Ok(ReplCommand::Empty)
}