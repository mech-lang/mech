
// Program
// =============================================================================

#![cfg_attr(feature = "no-std", no_std)]
#![cfg_attr(feature = "no-std", alloc)]
#![allow(dead_code)]
#![allow(warnings)]

use mech_core::*;

#[cfg(feature = "program")]
pub mod program;
//#[cfg(feature = "runloop")]
//pub mod runloop;
//#[cfg(feature = "persister")]
//pub mod persister;

#[cfg(feature = "program")]
pub use crate::program::*;
//#[cfg(feature = "runloop")]
//pub use crate::runloop::*;
//#[cfg(feature = "persister")]
//pub use crate::persister::*;

#[macro_export]
macro_rules! print_tree {
  ($tree:expr) => {
    #[cfg(feature = "pretty_print")]
    println!("{}", $tree.pretty_print());
    #[cfg(not(feature = "pretty_print"))]
    println!("{:#?}", $tree);
  };
}

#[macro_export]
macro_rules! print_symbols {
  ($intrp:expr) => {
    #[cfg(feature = "pretty_print")]
    println!("{}", $intrp.pretty_print_symbols());
    #[cfg(not(feature = "pretty_print"))]
    println!("{:#?}", $intrp.symbols());
  };
}

#[macro_export]
macro_rules! print_plan {
  ($intrp:expr) => {
    #[cfg(feature = "pretty_print")]
    println!("{}", $intrp.plan().pretty_print());
    #[cfg(not(feature = "pretty_print"))]
    println!("{:#?}", $intrp.plan());
  };
}

fn parse_program_host_calls(
  source: &str,
) -> MResult<Option<Vec<ProgramHostCall>>> {
  let mut calls = Vec::new();
  let mut saw_statement = false;

  for raw_line in source.lines() {
    let line = strip_comment(raw_line).trim();

    if line.is_empty() {
      continue;
    }

    saw_statement = true;

    let Some(call) = parse_program_host_call_statement(line)? else {
      return Ok(None);
    };

    calls.push(call);
  }

  if !saw_statement {
    return Ok(None);
  }

  Ok(Some(calls))
}

fn strip_comment(line: &str) -> &str {
  match line.find("//") {
    Some(index) => &line[..index],
    None => line,
  }
}

fn parse_program_host_call_statement(
  line: &str,
) -> MResult<Option<ProgramHostCall>> {
  if line == "actor.message.kind()" {
    return Ok(Some(ProgramHostCall {
      name: "actor.message.kind".to_string(),
      args: Vec::new(),
    }));
  }

  if line == "actor.message.payload()" {
    return Ok(Some(ProgramHostCall {
      name: "actor.message.payload".to_string(),
      args: Vec::new(),
    }));
  }

  if line == "actor.state.id()" {
    return Ok(Some(ProgramHostCall {
      name: "actor.state.id".to_string(),
      args: Vec::new(),
    }));
  }

  if line == "actor.state.get()" {
    return Ok(Some(ProgramHostCall {
      name: "actor.state.get".to_string(),
      args: Vec::new(),
    }));
  }

  if let Some(argument) = parse_actor_state_put(line)? {
    return Ok(Some(ProgramHostCall {
      name: "actor.state.put".to_string(),
      args: vec![Value::String(Ref::new(argument))],
    }));
  }

  Ok(None)
}

fn parse_actor_state_put(line: &str) -> MResult<Option<String>> {
  let prefix = "actor.state.put(";
  let suffix = ")";

  if !line.starts_with(prefix) {
    return Ok(None);
  }

  if !line.ends_with(suffix) {
    return Err(MechError::new(
      InvalidProgramHostCallError {
        message: format!("invalid actor.state.put statement: `{}`", line),
      },
      None,
    ));
  }

  let argument = &line[prefix.len()..line.len() - suffix.len()];
  let argument = argument.trim();

  let Some(text) = parse_quoted_string(argument)? else {
    return Err(MechError::new(
      InvalidProgramHostCallError {
        message: "actor.state.put expects one quoted string argument".to_string(),
      },
      None,
    ));
  };

  Ok(Some(text))
}

fn parse_quoted_string(input: &str) -> MResult<Option<String>> {
  if input.len() < 2 {
    return Ok(None);
  }

  if !input.starts_with('"') || !input.ends_with('"') {
    return Ok(None);
  }

  let inner = &input[1..input.len() - 1];
  let mut out = String::new();
  let mut chars = inner.chars();

  while let Some(ch) = chars.next() {
    if ch != '\\' {
      out.push(ch);
      continue;
    }

    let Some(escaped) = chars.next() else {
      return Err(MechError::new(
        InvalidProgramHostCallError {
          message: "unterminated escape sequence in string literal".to_string(),
        },
        None,
      ));
    };

    match escaped {
      '"' => out.push('"'),
      '\\' => out.push('\\'),
      'n' => out.push('\n'),
      'r' => out.push('\r'),
      't' => out.push('\t'),
      other => {
        return Err(MechError::new(
          InvalidProgramHostCallError {
            message: format!("unsupported escape sequence: \\{}", other),
          },
          None,
        ));
      }
    }
  }

  Ok(Some(out))
}

#[derive(Debug, Clone)]
pub struct InvalidProgramHostCallError {
  pub message: String,
}

impl MechErrorKind for InvalidProgramHostCallError {
  fn name(&self) -> &str {
    "InvalidProgramHostCall"
  }

  fn message(&self) -> String {
    self.message.clone()
  }
}