use mech_syntax::*;
use mech_syntax::parser::*;
use syntax::parser::tag;
use mech_utilities::*;
use mech_core::*;
use mech_core::nodes::*;
use crate::{minify_blocks, read_mech_files};

#[macro_use]
use nom::{
  IResult,
  branch::alt,
  sequence::tuple,
  combinator::opt,
  error::{context, convert_error, ErrorKind, ParseError, VerboseError},
  multi::{many1, many0},
  character::complete::{alphanumeric1, alpha1, digit1, space0, space1},
};


#[derive(Debug, Clone)]
pub enum ReplCommand {
  Help,
  Quit,
  Pause,
  Resume,
  Stop,
  Save,
  Debug,
  Clear,
  Info,
  Core(u64),
  Table(u64),
  Code(Vec<MechCode>),
  Empty,
  Error,
}

fn core(input: ParseString) -> ParseResult<ReplCommand> {
  let (input, _) = tag("core")(input)?;
  let (input, _) = syntax::parser::skip_spaces(input)?;
  let (input, text) = syntax::parser::text(input).unwrap();
  let core_id = syntax::compiler::compile_text(&text).unwrap();
  match core_id.parse::<u64>() {
    Ok(core_id) =>  Ok((input, ReplCommand::Core(core_id))), 
    Err(err) => Ok((input, ReplCommand::Error)),
  }
}

fn mech_code(input: ParseString) -> ParseResult<ReplCommand> {
  // Try parsing mech code fragment
  let (input,parse_tree) = syntax::parser::parse_mech_fragment(input)?;
  let mut compiler = compiler::Compiler::new();
  match compiler.compile_fragment_from_parse_tree(parse_tree) {
    Ok(blocks) => {
      let mut mb = minify_blocks(&blocks);
      Ok((input, ReplCommand::Code(vec![MechCode::MiniBlocks(mb)])))
    },
    Err(_) => Ok((input, ReplCommand::Error)),
  }
}

fn load(input: ParseString) -> ParseResult<ReplCommand> {
  let (input, _) = tag("load")(input)?;
  let (input, _) = syntax::parser::skip_spaces(input)?;
  let (input, text) = syntax::parser::text(input).unwrap();
  let string = syntax::compiler::compile_text(&text).unwrap();
  match read_mech_files(&vec![string.clone()]) {
    Ok(code) => {
      Ok((input, ReplCommand::Code(code)))
    }
    Err(err) => {
      Ok((input, ReplCommand::Error))
    }
  }
}

pub fn quit(input: ParseString) -> ParseResult<ReplCommand> {
  let (input,_) = alt((tag("quit"),tag("exit")))(input)?;
  Ok((input,ReplCommand::Quit))
}

pub fn clear(input: ParseString) -> ParseResult<ReplCommand> {
  let (input,_) = tag("clear")(input)?;
  Ok((input,ReplCommand::Clear))
}

pub fn save(input: ParseString) -> ParseResult<ReplCommand> {
  let (input,_) = tag("save")(input)?;
  Ok((input,ReplCommand::Save))
}

pub fn info(input: ParseString) -> ParseResult<ReplCommand> {
  let (input,_) = tag("info")(input)?;
  Ok((input,ReplCommand::Info))
}

pub fn debug(input: ParseString) -> ParseResult<ReplCommand> {
  let (input,_) = tag("debug")(input)?;
  Ok((input,ReplCommand::Debug))
}

pub fn help(input: ParseString) -> ParseResult<ReplCommand> {
  let (input,_) = tag("help")(input)?;
  Ok((input,ReplCommand::Help))
}

pub fn pause(input: ParseString) -> ParseResult<ReplCommand> {
  let (input,_) = tag("pause")(input)?;
  Ok((input,ReplCommand::Pause))
}

pub fn resume(input: ParseString) -> ParseResult<ReplCommand> {
  let (input,_) = tag("resume")(input)?;
  Ok((input,ReplCommand::Resume))
}

fn command(input: ParseString) -> ParseResult<ReplCommand> {
  let (input, _) = tag(":")(input)?;
  let (input, command) = alt((help,quit,clear,load,save,debug,info))(input)?;
  Ok((input, command))
}

pub fn parse_repl_command(input: ParseString) -> ParseResult<ReplCommand> {
  let (input,command) = alt((command, mech_code))(input)?;
  Ok((input, command))
}

pub fn parse(text: &str) -> Result<ReplCommand, MechError> {
  let graphemes = graphemes::init_source(text);
  let mut result_node = ParserNode::Error;
  let mut error_log: Vec<(SourceRange, ParseErrorDetail)> = vec![];
  let remaining: ParseString;

  match parse_repl_command(ParseString::new(&graphemes)) {
    Ok((input,command)) => Ok(command),
    Err(_)=> Ok(ReplCommand::Error),
  }

}