use mech_syntax::*;
use mech_syntax::parser::*;
use syntax::parser::tag;
use mech_utilities::*;
use mech_utilities::MechCode;
use mech_core::*;
use mech_core::nodes::*;
use crate::{minify_blocks, read_mech_files};

#[macro_use]
use nom::{
  IResult,
  branch::alt,
  sequence::tuple,
  combinator::opt,
  multi::{many1, many0},
  character::complete::{alphanumeric1, alpha1, digit1, space0, space1},
  Err,
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
  Reset,
  Info,
  NewCore,
  Core(u64),
  Table(u64),
  Code(Vec<MechCode>),
  Empty,
  Error(String),
}

fn core(input: ParseString) -> ParseResult<ReplCommand> {
  let (input, _) = tag("core")(input)?;
  let (input, _) = syntax::parser::skip_spaces(input)?;
  let (input, text) = syntax::parser::text(input)?;
  match syntax::compiler::compile_text(&text) {
    Ok(core_id) => {
      match core_id.parse::<u64>() {
        Ok(core_id) =>  Ok((input, ReplCommand::Core(core_id))), 
        Err(err) => Ok((input, ReplCommand::Error("".to_string()))),
      }
    }
    Err(err) => Ok((input, ReplCommand::Error(format!("{:?}",err)))),
  }
}

fn mech_code(input: ParseString) -> ParseResult<ReplCommand> {
  // Try parsing mech code fragment
  match syntax::parser::parse_mech_fragment(input) {
    Ok((input, parse_tree)) => {
      let mut compiler = compiler::Compiler::new();
      match compiler.compile_fragment_from_parse_tree(parse_tree) {
        Ok(blocks) => {
          let mut mb = minify_blocks(&blocks);
          Ok((input, ReplCommand::Code(vec![MechCode::MiniBlocks(mb)])))
        },
        Err(err) => Ok((input, ReplCommand::Error(format!("{:?}",err)))),
      }
    }
    Err(err) => Err(err),
  }
}

fn load(input: ParseString) -> ParseResult<ReplCommand> {
  let (input, _) = tag("load")(input)?;
  let (input, _) = syntax::parser::skip_spaces(input)?;
  let (input, text) = syntax::parser::text(input).unwrap();
  let string = syntax::compiler::compile_text(&text).unwrap();
  match read_mech_files(&vec![string.clone()]) {
    Ok(code) => {
      let code = code.iter().map(|(_,c)| c).cloned().collect::<Vec<MechCode>>();
      Ok((input, ReplCommand::Code(code)))
    }
    Err(err) => {
      Ok((input, ReplCommand::Error("".to_string())))
    }
  }
}

pub fn quit(input: ParseString) -> ParseResult<ReplCommand> {
  let (input,_) = alt((tag("quit"),tag("exit")))(input)?;
  Ok((input,ReplCommand::Quit))
}

pub fn reset(input: ParseString) -> ParseResult<ReplCommand> {
  let (input,_) = tag("reset")(input)?;
  Ok((input,ReplCommand::Reset))
}

pub fn save(input: ParseString) -> ParseResult<ReplCommand> {
  let (input,_) = tag("save")(input)?;
  Ok((input,ReplCommand::Save))
}

pub fn new(input: ParseString) -> ParseResult<ReplCommand> {
  let (input,_) = tag("new")(input)?;
  Ok((input,ReplCommand::NewCore))
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
  let (input, command) = alt((help,quit,reset,load,save,debug,info,core,new))(input)?;
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
    Ok((input,command)) => {
      return Ok(command);
    }
    Err(err) => match err {
      Err::Error(mut e) | Err::Failure(mut e) => {
        error_log.append(&mut e.remaining_input.error_log);
        error_log.push((e.cause_range, e.error_detail));
        remaining = e.remaining_input;
        let report: ParserErrorReport = error_log.into_iter().map(|e| ParserErrorContext {
          cause_rng: e.0,
          err_message: String::from(e.1.message),
          annotation_rngs: e.1.annotation_rngs,
        }).collect();
        let msg = TextFormatter::new(text).format_error(&report);
        Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 3392, kind: MechErrorKind::ParserError(result_node, report, msg)})
      },
      Err::Incomplete(_) => panic!("nom::Err::Incomplete is not supported!"),
    },
  }
}