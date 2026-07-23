//! Parser for source activation scopes.
use crate::*;
use mech_core::nodes::*;
use nom::{branch::alt, combinator::{cut, opt}, multi::many1, sequence::preceded};

/// activation-scope := "~>" expression ("{" mech-code "}" | activation-arm+) ;
pub fn activation_scope(input: ParseString) -> ParseResult<ActivationScope> {
  let (input, operator) = async_transition_operator(input)?;
  let (input, trigger) = cut(expression)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, body) = if let Ok((input, _)) = left_brace(input.clone()) {
    let (input, _) = whitespace0(input)?;
    if let Ok((input, _)) = right_brace(input.clone()) {
      (input, ActivationBody::Block(Vec::new()))
    } else {
      let (input, parsed) = mech_code(input)?;
      let (input, _) = cut(right_brace)(input)?;
      (input, ActivationBody::Block(parsed.code))
    }
  } else {
    let (input, arms) = cut(many1(activation_arm))(input)?;
    let (input, _) = opt(period)(input)?;
    (input, ActivationBody::PatternArms(arms))
  };
  Ok((input, ActivationScope { operator, trigger, body }))
}

fn activation_arm(input: ParseString) -> ParseResult<ActivationArm> {
  let (input, _) = crate::state_machines::guard_operator(input)?;
  let (input, pattern) = crate::patterns::pattern(input)?;
  let (input, guard) = opt(preceded(list_separator, preceded(whitespace0, expression)))(input)?;
  let (input, _) = output_operator(input)?;
  let (input, _) = whitespace0(input)?;
  if let Ok((input, _)) = left_brace(input.clone()) {
    let (input, _) = whitespace0(input)?;
    if let Ok((input, _)) = right_brace(input.clone()) {
      Ok((input, ActivationArm { pattern, guard, body: ActivationArmBody::Block(Vec::new()) }))
    } else {
      let (input, parsed) = mech_code(input)?;
      let (input, _) = cut(right_brace)(input)?;
      Ok((input, ActivationArm { pattern, guard, body: ActivationArmBody::Block(parsed.code) }))
    }
  } else {
    let (input, expression) = expression(input)?;
    let (input, _) = opt(alt((whitespace1, statement_separator)))(input)?;
    Ok((input, ActivationArm { pattern, guard, body: ActivationArmBody::Expression(expression) }))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test] fn activation_scope_parses_fixed_block_body() { assert!(parse("~> tick {\n output := x + 1\n}").is_ok()); }
  #[test] fn activation_scope_parses_pattern_block_and_expression_arms() { assert!(parse("~> event\n | :pressed(x) => {\n output := x\n }\n | * => 0.").is_ok()); }
  #[test] fn activation_scope_parses_pattern_guard() { assert!(parse("~> event\n | (x, y), x > y => { output := x }\n | * => { output := 0 }").is_ok()); }
  #[test] fn activation_scope_parses_empty_fixed_and_pattern_blocks() {
    let fixed = parse("~> tick {}").unwrap();
    let SectionElement::MechCode(fixed) = &fixed.body.sections[0].elements[0] else {
      panic!("expected fixed activation code");
    };
    assert!(matches!(
      &fixed[0].0,
      MechCode::ActivationScope(ActivationScope { body: ActivationBody::Block(body), .. })
        if body.is_empty()
    ));

    let patterned = parse("~> tick\n | value, value > 0 => {\n output := value\n }\n | * => {}").unwrap();
    let SectionElement::MechCode(patterned) = &patterned.body.sections[0].elements[0] else {
      panic!("expected patterned activation code");
    };
    let MechCode::ActivationScope(ActivationScope {
      body: ActivationBody::PatternArms(arms),
      ..
    }) = &patterned[0].0 else {
      panic!("expected patterned activation");
    };
    assert!(matches!(&arms[1].body, ActivationArmBody::Block(body) if body.is_empty()));
  }
  #[test] fn activation_scope_pattern_body_does_not_parse_as_match_expression() { let p=parse("~> event | * => 0.").unwrap(); let SectionElement::MechCode(code) = &p.body.sections[0].elements[0] else { panic!("expected Mech code") }; assert!(matches!(&code[0].0, MechCode::ActivationScope(ActivationScope { body: ActivationBody::PatternArms(_), .. }))); }
  #[test] fn activation_scope_does_not_conflict_with_mutable_definition() { assert!(parse("~x := 10\n~> tick {\n output := x + 1\n}").is_ok()); }
}
