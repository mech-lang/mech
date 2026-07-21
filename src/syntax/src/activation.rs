//! Parser for the source activation block (`~> trigger { ... }`).
use crate::*;
use mech_core::nodes::*;
use nom::combinator::cut;

/// activation-scope := "~>", whitespace0, expression, whitespace0, "{", mech-code, "}" ;
/// The header deliberately uses the ordinary expression parser; semantic validation
/// of the stable-reference restriction belongs to elaboration.
pub fn activation_scope(input: ParseString) -> ParseResult<ActivationScope> {
  let (input, operator) = async_transition_operator(input)?;
  let (input, trigger) = cut(expression)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = cut(left_brace)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, parsed) = mech_code(input)?;
  let (input, _) = cut(right_brace)(input)?;
  Ok((input, ActivationScope { operator, trigger, body: parsed.code }))
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn activation_scope_parses_wildcard_block_form() {
    let program = parse("~> tick {\n  output := x + 1\n}").expect("activation scope parses");
    let code = &program.body.sections[0].elements;
    assert!(format!("{:?}", code).contains("ActivationScope"));
  }
  #[test]
  fn activation_scope_does_not_conflict_with_mutable_definition() {
    let program = parse("~x := 10\n~> tick {\n output := x + 1\n}").expect("parse");
    assert!(format!("{:?}", program).contains("ActivationScope"));
  }
  #[test]
  fn activation_scope_parses_nested_record_literal() {
    assert!(parse("~> tick {\n point := { x: x, y: y }\n}").is_ok());
  }
}
