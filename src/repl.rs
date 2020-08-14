use mech_syntax::parser::{Parser, Node as ParserNode};

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
  ParsedCode(ParserNode),
  Empty,
  Error,
}