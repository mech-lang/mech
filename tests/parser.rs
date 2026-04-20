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

#[test]
fn parser_rejects_pattern_rest_syntax_on_rhs_matrix_construction() {
  let source = "#demo(xs<[u64]>) => <[u64]>\n  ├ :Pass(xs<[u64]>)\n  └ :Done(out<[u64]>).\n\n#demo(xs) -> :Pass(xs)\n  :Pass([x | tail]) -> :Done([x | tail])\n  :Done(out) => out.\n\n#demo([1u64 2u64])";
  let result = parse(source);
  assert!(result.is_err(), "RHS [x | tail] should not parse as matrix construction");
}

#[test]
fn parser_accepts_mech_fenced_code_block_as_program_source() {
  let source = "```mech:pattern-matching\n<result> := :ok<u64> | :err<u64>\n<option> := :some<result> | :none\nx<option> := :some(:ok(42u64))\n```";
  let result = parse(source);
  assert!(result.is_ok(), "fenced mech code block should parse as mech program source");
}

#[test]
fn parser_accepts_mech_fenced_code_blocks_embedded_in_markdown() {
  let source = "Mech v0.3 — Program Specification\n\n- Item one\n- Item two\n\n```mech:enums\n<color> := :red | :green | :blue\nx<color> := :red\n```\n";
  let result = parse(source);
  assert!(result.is_ok(), "embedded fenced mech code block should parse even with surrounding markdown");
}

#[test]
fn parser_accepts_markdown_with_bullets_links_and_fenced_mech() {
  let source = "Mech v0.3 — Program Specification\n\n- **Enums and tagged unions** for modeling categorical data and optional values (§2)\n- [Expression broadcasting](https://docs.mech-lang.org/reference/broadcasting.html): applying operations.\n\n```mech:enums\n<color> := :red | :green | :blue\nx<color> := :red\n```\n";
  let result = parse(source);
  assert!(result.is_ok(), "markdown prose and list content should parse around fenced mech blocks");
}
