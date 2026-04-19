use mech_syntax::{parse, ParserErrorReport};

#[test]
fn parser_match_arm_reports_transition_arrow_mismatch() {
  let source = "y := x?\n  | * -> a\n  | * => 0.";
  let result = parse(source);
  assert!(result.is_err());
  let err = result.unwrap_err();
  let report = err
    .kind_as::<ParserErrorReport>()
    .expect("expected parser error report");

  let has_specific_message = report
    .1
    .iter()
    .any(|ctx| ctx.err_message.contains("Match arm: expected `=>` but found `->`"));
  let all_messages = report
    .1
    .iter()
    .map(|ctx| ctx.err_message.clone())
    .collect::<Vec<String>>()
    .join(" | ");
  assert!(has_specific_message, "report should mention => vs -> mismatch; got: {all_messages}");
}

#[test]
fn parser_match_arm_accepts_output_arrow() {
  let source = "y := x?\n  | * => a\n  | * => 0.";
  let result = parse(source);
  assert!(result.is_ok(), "valid match arms should parse");
}

#[test]
fn parser_match_arm_error_does_not_fall_back_to_paragraph_noise() {
  let source = "x := [1 2 3 4]\n\ny := x?\n  | [h a ... t] -> a\n  | * => 0.";
  let err = parse(source).unwrap_err();
  let report = err
    .kind_as::<ParserErrorReport>()
    .expect("expected parser error report");
  let has_paragraph_noise = report
    .1
    .iter()
    .any(|ctx| ctx.err_message.contains("Unexpected paragraph element"));
  assert!(!has_paragraph_noise, "should stay in mech parser context");
}

#[test]
fn parser_match_expression_requires_terminating_period() {
  let source = "x := [2 2 3 4]\n\ny := x?\n  | [h | t] => t\n  | * => 0\n\nz := [7 8 9]";
  let err = parse(source).unwrap_err();
  let report = err
    .kind_as::<ParserErrorReport>()
    .expect("expected parser error report");
  let has_period_message = report
    .1
    .iter()
    .any(|ctx| ctx.err_message.contains("Match expression expects terminating period `.`"));
  assert!(has_period_message, "missing-period error should be explicit");
  let has_unexpected_char = report
    .1
    .iter()
    .any(|ctx| ctx.err_message == "Unexpected character");
  assert!(!has_unexpected_char, "missing-period case should avoid generic unexpected-character noise");
}
