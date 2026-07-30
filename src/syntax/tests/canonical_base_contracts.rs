use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use mech_syntax::document::parser::canonical::{
  FIXED_TERMINAL_COUNT, FIXED_TERMINALS, FixedTerminalSpec, TerminalSpacing,
  canonical_base_rule_supported, parse_canonical_base_rule_for_test,
  parse_canonical_tag_for_test,
};
use mech_syntax::document::parser::{
  CANONICAL_PORTS, PortPhase, RuleFamily, canonical_rule_name, rules,
};
use mech_syntax::document::{
  DocumentId, ParseConfig, Revision, SyntaxKind, TextRange, TextSize, TextSnapshot,
  lower_legacy_escaped_character, reconstruct_source_range, validate_lossless_range,
};

fn source(text: &str) -> TextSnapshot {
  TextSnapshot::new(DocumentId(31), Revision(0), text).unwrap()
}

fn parse(text: &str, rule: mech_syntax::document::RuleId) -> mech_syntax::document::parser::canonical::CanonicalRuleSnapshot {
  parse_canonical_base_rule_for_test(source(text), rule, ParseConfig::default())
    .expect("Phase 2A base rule must have a canonical port")
}

fn legacy_extent<Output>(
  input: &str,
  parser: for<'source> fn(
    mech_syntax::ParseString<'source>,
  ) -> mech_syntax::ParseResult<'source, Output>,
) -> Option<TextSize> {
  let graphemes = mech_syntax::graphemes::init_tag(input);
  parser(mech_syntax::ParseString::new(&graphemes))
    .ok()
    .map(|(remaining, _)| {
      TextSize(
        graphemes[..remaining.cursor]
          .iter()
          .map(|grapheme| grapheme.len() as u32)
          .sum(),
      )
    })
}

fn assert_legacy_boundary_parity<Output>(
  rule: mech_syntax::document::RuleId,
  parser: for<'source> fn(
    mech_syntax::ParseString<'source>,
  ) -> mech_syntax::ParseResult<'source, Output>,
  inputs: &[&str],
) {
  for input in inputs {
    let canonical = parse(input, rule);
    let canonical_extent = canonical.matched.then_some(canonical.consumed.end);
    assert_eq!(
      canonical_extent,
      legacy_extent(input, parser),
      "{} boundary mismatch for {input:?}",
      canonical_rule_name(rule).unwrap_or("<unknown>")
    );
  }
}

#[derive(Debug, Eq, PartialEq)]
struct LegacyFixedTerminalContract {
  literal: String,
  legacy_kind: String,
  spacing: TerminalSpacing,
}

fn split_macro_arguments(arguments: &str) -> Vec<&str> {
  let mut result = Vec::new();
  let mut start = 0;
  let mut in_string = false;
  let mut escaped = false;
  for (index, character) in arguments.char_indices() {
    if in_string {
      if escaped {
        escaped = false;
      } else if character == '\\' {
        escaped = true;
      } else if character == '"' {
        in_string = false;
      }
    } else if character == '"' {
      in_string = true;
    } else if character == ',' {
      result.push(arguments[start..index].trim());
      start = index + character.len_utf8();
    }
  }
  result.push(arguments[start..].trim());
  result
}

fn decode_rust_string_literal(literal: &str) -> String {
  let body = literal
    .strip_prefix('"')
    .and_then(|literal| literal.strip_suffix('"'))
    .unwrap_or_else(|| panic!("expected a Rust string literal, found {literal:?}"));
  let mut decoded = String::new();
  let mut characters = body.chars();
  while let Some(character) = characters.next() {
    if character != '\\' {
      decoded.push(character);
      continue;
    }
    match characters.next().expect("unterminated string escape") {
      '\\' => decoded.push('\\'),
      '"' => decoded.push('"'),
      'n' => decoded.push('\n'),
      'r' => decoded.push('\r'),
      't' => decoded.push('\t'),
      'u' => {
        assert_eq!(characters.next(), Some('{'), "malformed Unicode escape");
        let mut digits = String::new();
        loop {
          let character = characters.next().expect("unterminated Unicode escape");
          if character == '}' {
            break;
          }
          digits.push(character);
        }
        let value = u32::from_str_radix(&digits, 16)
          .unwrap_or_else(|error| panic!("invalid Unicode escape {digits:?}: {error}"));
        decoded.push(char::from_u32(value).expect("Unicode escape must be a scalar"));
      }
      escape => panic!("unsupported string escape \\{escape}"),
    }
  }
  decoded
}

fn legacy_fixed_terminal_contracts() -> BTreeMap<String, LegacyFixedTerminalContract> {
  let source = fs::read_to_string(
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/base.rs"),
  )
  .expect("read legacy base terminal declarations");
  let mut contracts = BTreeMap::new();
  for line in source.lines() {
    let line = line.trim();
    let (spacing, invocation) = if let Some(invocation) = line.strip_prefix("leaf!") {
      (TerminalSpacing::Exact, invocation)
    } else if let Some(invocation) = line.strip_prefix("ws0_leaf!") {
      (TerminalSpacing::Whitespace0Both, invocation)
    } else {
      continue;
    };
    let open = invocation
      .find(['{', '('])
      .unwrap_or_else(|| panic!("malformed fixed-terminal declaration: {line}"));
    let close_delimiter = match invocation.as_bytes()[open] {
      b'{' => '}',
      b'(' => ')',
      _ => unreachable!(),
    };
    let close = invocation
      .rfind(close_delimiter)
      .unwrap_or_else(|| panic!("unclosed fixed-terminal declaration: {line}"));
    let fields = split_macro_arguments(&invocation[open + 1..close]);
    assert_eq!(fields.len(), 3, "malformed fixed-terminal declaration: {line}");
    let rule = fields[0].replace('_', "-");
    let contract = LegacyFixedTerminalContract {
      literal: decode_rust_string_literal(fields[1]),
      legacy_kind: fields[2]
        .strip_prefix("TokenKind::")
        .unwrap_or_else(|| panic!("missing legacy TokenKind in {line}"))
        .to_owned(),
      spacing,
    };
    assert!(
      contracts.insert(rule.clone(), contract).is_none(),
      "duplicate legacy fixed-terminal rule {rule}"
    );
  }
  contracts
}

#[test]
fn every_fixed_terminal_executes_its_exact_phase_0_contract() {
  assert_eq!(FIXED_TERMINAL_COUNT, 108);
  assert_eq!(FIXED_TERMINALS.len(), FIXED_TERMINAL_COUNT);

  let mut legacy = legacy_fixed_terminal_contracts();
  assert_eq!(
    legacy.len(),
    FIXED_TERMINAL_COUNT,
    "legacy source must declare the complete Phase 0 fixed-terminal set"
  );

  for spec in FIXED_TERMINALS {
    let name = canonical_rule_name(spec.rule).expect("registered terminal RuleId");
    let contract = legacy
      .remove(name)
      .unwrap_or_else(|| panic!("canonical terminal {name} has no legacy declaration"));
    assert_eq!(
      spec.literal, contract.literal,
      "fixed-terminal literal drift for {name}"
    );
    assert_eq!(
      expected_legacy_kind(spec.kind),
      contract.legacy_kind,
      "fixed-terminal legacy TokenKind drift for {name}"
    );
    assert_eq!(
      spec.spacing, contract.spacing,
      "fixed-terminal spacing drift for {name}"
    );
    assert_fixed_terminal(spec);
  }
  assert!(
    legacy.is_empty(),
    "legacy fixed terminals lack canonical rules: {:?}",
    legacy.keys().collect::<Vec<_>>()
  );
}

#[test]
fn fixed_terminals_do_not_match_inside_an_extended_grapheme() {
  let input = ".\u{301}";
  let parsed = parse(input, rules::PERIOD);
  assert!(!parsed.matched);
  assert_eq!(parsed.consumed, TextRange::empty(TextSize::ZERO));
  assert!(parsed.syntax().tokens().is_empty());

  let graphemes = mech_syntax::graphemes::init_source(input);
  assert!(mech_syntax::period(mech_syntax::ParseString::new(&graphemes)).is_err());
}

fn expected_legacy_kind(kind: SyntaxKind) -> &'static str {
  match kind {
    SyntaxKind::AbstractSigil => "AbstractSigil",
    SyntaxKind::Alpha => "Alpha",
    SyntaxKind::Ampersand => "Ampersand",
    SyntaxKind::Apostrophe => "Apostrophe",
    SyntaxKind::AssignOperator => "AssignOperator",
    SyntaxKind::Asterisk => "Asterisk",
    SyntaxKind::AsyncTransitionOperator => "AsyncTransitionOperator",
    SyntaxKind::At => "At",
    SyntaxKind::Backslash => "Backslash",
    SyntaxKind::Bar => "Bar",
    SyntaxKind::BoxDrawing => "BoxDrawing",
    SyntaxKind::Caret => "Caret",
    SyntaxKind::CarriageReturn => "CarriageReturn",
    SyntaxKind::Colon => "Colon",
    SyntaxKind::Comma => "Comma",
    SyntaxKind::Dash => "Dash",
    SyntaxKind::DefineOperatorToken => "DefineOperator",
    SyntaxKind::Digit => "Digit",
    SyntaxKind::Dollar => "Dollar",
    SyntaxKind::Emoji => "Emoji",
    SyntaxKind::EmphasisSigil => "EmphasisSigil",
    SyntaxKind::EquationSigil => "EquationSigil",
    SyntaxKind::Equal => "Equal",
    SyntaxKind::ErrorSigil => "ErrorSigil",
    SyntaxKind::EscapedChar => "EscapedChar",
    SyntaxKind::Exclamation => "Exclamation",
    SyntaxKind::False => "False",
    SyntaxKind::FloatLeft => "FloatLeft",
    SyntaxKind::FloatRight => "FloatRight",
    SyntaxKind::FootnotePrefix => "FootnotePrefix",
    SyntaxKind::GenOperator => "GenOperator",
    SyntaxKind::GeneratorArrow => "GeneratorArrow",
    SyntaxKind::Grave => "Grave",
    SyntaxKind::GraveCodeBlockSigil => "GraveCodeBlockSigil",
    SyntaxKind::HashTag => "HashTag",
    SyntaxKind::HighlightSigil => "HighlightSigil",
    SyntaxKind::HttpPrefix => "HttpPrefix",
    SyntaxKind::IdeaSigil => "IdeaSigil",
    SyntaxKind::ImgPrefix => "ImgPrefix",
    SyntaxKind::InfoSigil => "InfoSigil",
    SyntaxKind::LeftAngle => "LeftAngle",
    SyntaxKind::LeftBrace => "LeftBrace",
    SyntaxKind::LeftBracket => "LeftBracket",
    SyntaxKind::LeftParen => "LeftParenthesis",
    SyntaxKind::MikaSectionClose => "MikaSectionClose",
    SyntaxKind::MikaSectionOpen => "MikaSectionOpen",
    SyntaxKind::ModuleExportSigil => "ModuleExportSigil",
    SyntaxKind::ModuleImportSigil => "ModuleImportSigil",
    SyntaxKind::Newline => "Newline",
    SyntaxKind::Not => "Not",
    SyntaxKind::OutputOperator => "OutputOperator",
    SyntaxKind::Percent => "Percent",
    SyntaxKind::Period => "Period",
    SyntaxKind::Plus => "Plus",
    SyntaxKind::PromptSigil => "PromptSigil",
    SyntaxKind::Question => "Question",
    SyntaxKind::QuestionSigil => "QuestionSigil",
    SyntaxKind::Quote => "Quote",
    SyntaxKind::QuoteSigil => "QuoteSigil",
    SyntaxKind::RightAngle => "RightAngle",
    SyntaxKind::RightBrace => "RightBrace",
    SyntaxKind::RightBracket => "RightBracket",
    SyntaxKind::RightParen => "RightParenthesis",
    SyntaxKind::SectionSigil => "SectionSigil",
    SyntaxKind::Semicolon => "Semicolon",
    SyntaxKind::Slash => "Slash",
    SyntaxKind::SpreadOperator => "SpreadOperator",
    SyntaxKind::StrikeSigil => "StrikeSigil",
    SyntaxKind::StrongSigil => "StrongSigil",
    SyntaxKind::SuccessSigil => "SuccessSigil",
    SyntaxKind::SynthOperator => "SynthOperator",
    SyntaxKind::Tab => "Tab",
    SyntaxKind::Tilde => "Tilde",
    SyntaxKind::TildeCodeBlockSigil => "TildeCodeBlockSigil",
    SyntaxKind::TransitionOperator => "TransitionOperator",
    SyntaxKind::True => "True",
    SyntaxKind::UnderlineSigil => "UnderlineSigil",
    SyntaxKind::Underscore => "Underscore",
    SyntaxKind::WarningSigil => "WarningSigil",
    SyntaxKind::Whitespace => "Space",
    other => panic!("no fixed-terminal legacy mapping for {other:?}"),
  }
}

fn assert_fixed_terminal(spec: &FixedTerminalSpec) {
  let (input, literal_start) = match spec.spacing {
    TerminalSpacing::Exact => (spec.literal.to_owned(), 0_usize),
    TerminalSpacing::Whitespace0Both => {
      (format!(" \n{}\t\r", spec.literal), 2)
    }
  };
  let parsed = parse(&input, spec.rule);
  assert!(parsed.matched, "terminal {:?} did not match", spec.rule);
  assert_eq!(
    parsed.consumed,
    TextRange::new(TextSize::ZERO, TextSize(input.len() as u32))
  );
  assert!(parsed.diagnostics.is_empty());
  validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
  assert_eq!(
    reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
    input
  );

  let expected = TextRange::new(
    TextSize(literal_start as u32),
    TextSize((literal_start + spec.literal.len()) as u32),
  );
  let semantic = parsed
    .syntax()
    .tokens()
    .into_iter()
    .filter(|token| token.kind() == spec.kind && token.range() == expected)
    .collect::<Vec<_>>();
  assert_eq!(
    semantic.len(),
    1,
    "terminal {:?} did not emit one exact semantic token",
    spec.rule
  );
  assert_eq!(semantic[0].text().unwrap(), spec.literal);

  let trailing = format!("{}x", spec.literal);
  let prefix = parse(&trailing, spec.rule);
  if spec.spacing == TerminalSpacing::Exact {
    assert_eq!(
      prefix.consumed,
      TextRange::new(
        TextSize::ZERO,
        TextSize(spec.literal.len() as u32),
      )
    );
  }
}

#[test]
fn the_complete_149_rule_lexical_selection_has_a_canonical_entry() {
  let selected = CANONICAL_PORTS
    .iter()
    .filter(|port| {
      port.phase == Some(PortPhase::Phase2A)
        && (port.family == RuleFamily::Base
          || matches!(
            port.name,
            "left-angle"
              | "right-angle"
              | "box-drawing-char"
              | "box-drawing-emoji"
              | "tag"
          ))
    })
    .collect::<Vec<_>>();
  assert_eq!(selected.len(), 149);
  for port in selected {
    assert!(
      canonical_base_rule_supported(port.rule),
      "{} has no canonical base implementation",
      port.name
    );
  }

  let tag = parse_canonical_tag_for_test(source("λx"), "λ", ParseConfig::default());
  assert!(tag.matched);
  assert_eq!(tag.consumed, TextRange::new(TextSize::ZERO, TextSize(2)));
  assert_eq!(tag.syntax().tokens()[0].kind(), SyntaxKind::Text);
}

#[test]
fn every_non_fixed_lexical_port_executes_a_canonical_contract() {
  let cases = [
    (rules::TRANSITION_OPERATOR, " -> "),
    (rules::OUTPUT_OPERATOR, "\n=>\t"),
    (rules::EMOJI_GRAPHEME, "💡"),
    (rules::ALPHA, "e\u{301}"),
    (rules::DIGIT, "1\u{20e3}"),
    (rules::ANY, "\r\n"),
    (rules::ANY_TOKEN, "λ"),
    (rules::FORBIDDEN_EMOJI, "┌"),
    (rules::EMOJI, "💡"),
    (rules::ALPHA_TOKEN, "Δ"),
    (rules::DIGIT_TOKEN, "٣"),
    (rules::ALPHANUMERIC, "9"),
    (rules::UNDERSCORE_DIGIT, "_9"),
    (rules::DIGIT_SEQUENCE, "1_2"),
    (rules::GROUPING_SYMBOL, "("),
    (rules::PUNCTUATION, "."),
    (rules::ESCAPED_CHAR, "\\n"),
    (rules::SYMBOL, "&"),
    (rules::IDENTIFIER_SYMBOL, "-"),
    (rules::TEXT, "\\n"),
    (rules::RAW_TEXT, "x"),
    (rules::NEW_LINE, "\r\n"),
    (rules::WHITESPACE, "\n"),
    (rules::WHITESPACE0, " \n"),
    (rules::WHITESPACE1, "\n"),
    (rules::NEWLINE_INDENT, "\n \t"),
    (rules::WS1E, "\u{00a0}"),
    (rules::WS0E, "\u{2009}"),
    (rules::SPACE_TAB, "\u{00a0}"),
    (rules::SPACE_TAB0, " \t"),
    (rules::SPACE_TAB1, "\u{2009}"),
    (rules::LIST_SEPARATOR, " \n,\t"),
    (rules::ENUM_SEPARATOR, "\n| \r"),
    (rules::IDENTIFIER, "a-b"),
    (rules::IDENTIFIER_PATH_SEGMENT_EMOJI, "💡"),
    (rules::IDENTIFIER_PATH_SEGMENT, "a-b"),
    (rules::LEFT_ANGLE, "⟨"),
    (rules::RIGHT_ANGLE, "⟩"),
    (rules::BOX_DRAWING_CHAR, "┌"),
    (rules::BOX_DRAWING_EMOJI, "┃"),
  ];
  assert_eq!(cases.len() + FIXED_TERMINAL_COUNT + 1, 149);
  for (index, (rule, input)) in cases.iter().enumerate() {
    assert!(
      cases[..index].iter().all(|(earlier, _)| earlier != rule),
      "duplicate non-fixed contract for {:?}",
      rule
    );
    let parsed = parse(input, *rule);
    assert!(parsed.matched, "{:?} did not match {input:?}", rule);
    assert_eq!(
      parsed.consumed,
      parsed.source.full_range(),
      "{:?} did not completely consume {input:?}",
      rule
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", rule);
    validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
    assert_eq!(
      reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
      *input
    );
  }
}

#[test]
fn all_41_non_fixed_lexical_contracts_match_legacy_boundaries() {
  assert_legacy_boundary_parity(
    rules::TRANSITION_OPERATOR,
    mech_syntax::transition_operator,
    &[" -> ", "\t→\n", "~>"],
  );
  assert_legacy_boundary_parity(
    rules::OUTPUT_OPERATOR,
    mech_syntax::output_operator,
    &[" => ", "⇒", "->"],
  );
  assert_legacy_boundary_parity(
    rules::EMOJI_GRAPHEME,
    mech_syntax::emoji_grapheme,
    &["💡", "┌", "a"],
  );
  assert_legacy_boundary_parity(
    rules::ALPHA,
    mech_syntax::alpha,
    &["e\u{301}", "Δ", "1"],
  );
  assert_legacy_boundary_parity(
    rules::DIGIT,
    mech_syntax::digit,
    &["1\u{20e3}", "٣", "a"],
  );
  assert_legacy_boundary_parity(rules::ANY, mech_syntax::any, &["\r\n", "λ", ""]);
  assert_legacy_boundary_parity(
    rules::ANY_TOKEN,
    mech_syntax::any_token,
    &["\r\n", "λ", ""],
  );
  assert_legacy_boundary_parity(
    rules::FORBIDDEN_EMOJI,
    mech_syntax::forbidden_emoji,
    &["┌", "⟨", "💡"],
  );
  assert_legacy_boundary_parity(rules::EMOJI, mech_syntax::emoji, &["💡", "┌", "a"]);
  assert_legacy_boundary_parity(
    rules::ALPHA_TOKEN,
    mech_syntax::alpha_token,
    &["e\u{301}", "Δ", "1"],
  );
  assert_legacy_boundary_parity(
    rules::DIGIT_TOKEN,
    mech_syntax::digit_token,
    &["1\u{20e3}", "٣", "a"],
  );
  assert_legacy_boundary_parity(
    rules::ALPHANUMERIC,
    mech_syntax::alphanumeric,
    &["A", "٣", "💡"],
  );
  assert_legacy_boundary_parity(
    rules::UNDERSCORE_DIGIT,
    mech_syntax::underscore_digit,
    &["_٣", "_a", "٣"],
  );
  assert_legacy_boundary_parity(
    rules::DIGIT_SEQUENCE,
    mech_syntax::digit_sequence,
    &["1_024", "٣_٤", "1_", "a"],
  );
  assert_legacy_boundary_parity(
    rules::GROUPING_SYMBOL,
    mech_syntax::grouping_symbol,
    &["(", "⟩", "x"],
  );
  assert_legacy_boundary_parity(
    rules::PUNCTUATION,
    mech_syntax::punctuation,
    &[".", "\"", "&"],
  );
  assert_legacy_boundary_parity(
    rules::ESCAPED_CHAR,
    mech_syntax::escaped_char,
    &["\\n", "\\e\u{301}", "\\.\u{301}"],
  );
  assert_legacy_boundary_parity(rules::SYMBOL, mech_syntax::symbol, &["&", "\\", "!"]);
  assert_legacy_boundary_parity(
    rules::IDENTIFIER_SYMBOL,
    mech_syntax::identifier_symbol,
    &["-", "/", "_"],
  );
  assert_legacy_boundary_parity(rules::TEXT, mech_syntax::text, &["\\n", "💡", "\n"]);
  assert_legacy_boundary_parity(
    rules::RAW_TEXT,
    mech_syntax::raw_text,
    &["\\n", "(", "\n"],
  );
  assert_legacy_boundary_parity(
    rules::NEW_LINE,
    mech_syntax::new_line,
    &["\r\n", "\r", "x"],
  );
  assert_legacy_boundary_parity(
    rules::WHITESPACE,
    mech_syntax::whitespace,
    &[" ", "\r\n", "\u{00a0}"],
  );
  assert_legacy_boundary_parity(
    rules::WHITESPACE0,
    mech_syntax::whitespace0,
    &[" \t\r\nx", "\u{00a0}", ""],
  );
  assert_legacy_boundary_parity(
    rules::WHITESPACE1,
    mech_syntax::whitespace1,
    &[" \n", "\u{00a0}", ""],
  );
  assert_legacy_boundary_parity(
    rules::NEWLINE_INDENT,
    mech_syntax::newline_indent,
    &["\r\n \tX", "\n\u{00a0}X", "X"],
  );
  assert_legacy_boundary_parity(
    rules::WS1E,
    mech_syntax::ws1e,
    &["\u{00a0}\u{2009}\tX", "\n", ""],
  );
  assert_legacy_boundary_parity(
    rules::WS0E,
    mech_syntax::ws0e,
    &["\u{00a0}\u{2009}\tX", "\n", ""],
  );
  assert_legacy_boundary_parity(
    rules::SPACE_TAB,
    mech_syntax::space_tab,
    &["\u{00a0}", "\u{2009}", "\n"],
  );
  assert_legacy_boundary_parity(
    rules::SPACE_TAB0,
    mech_syntax::space_tab0,
    &[" \t\u{00a0}X", "\n", ""],
  );
  assert_legacy_boundary_parity(
    rules::SPACE_TAB1,
    mech_syntax::space_tab1,
    &[" \t\u{00a0}X", "\n", ""],
  );
  assert_legacy_boundary_parity(
    rules::LIST_SEPARATOR,
    mech_syntax::list_separator,
    &[" \n,\t", ",", "|"],
  );
  assert_legacy_boundary_parity(
    rules::ENUM_SEPARATOR,
    mech_syntax::enum_separator,
    &[" \n|\t", "|", ","],
  );
  assert_legacy_boundary_parity(
    rules::IDENTIFIER,
    mech_syntax::identifier,
    &["a-b", "a/b", "Δx^2", "1a", "a:b"],
  );
  // The legacy helper is private, so isolate its behavior through the public
  // path-segment wrapper with a delimiter immediately after the emoji.
  assert_legacy_boundary_parity(
    rules::IDENTIFIER_PATH_SEGMENT_EMOJI,
    mech_syntax::identifier_path_segment,
    &["💡/", "💡,", "┌/"],
  );
  assert_legacy_boundary_parity(
    rules::IDENTIFIER_PATH_SEGMENT,
    mech_syntax::identifier_path_segment,
    &["a-b", "Δ2", "💡x", "a/b", "1a"],
  );
  assert_legacy_boundary_parity(
    rules::LEFT_ANGLE,
    mech_syntax::left_angle,
    &["<", "⟨", "("],
  );
  assert_legacy_boundary_parity(
    rules::RIGHT_ANGLE,
    mech_syntax::right_angle,
    &[">", "⟩", ")"],
  );
  assert_legacy_boundary_parity(
    rules::BOX_DRAWING_CHAR,
    mech_syntax::box_drawing_char,
    &["┌", "┛", "a"],
  );
  assert_legacy_boundary_parity(
    rules::BOX_DRAWING_EMOJI,
    mech_syntax::box_drawing_emoji,
    &["┃", "╯", "a"],
  );

  for input in ["λ", "λx", "x", "λ\u{301}"] {
    let canonical =
      parse_canonical_tag_for_test(source(input), "λ", ParseConfig::default());
    let graphemes = mech_syntax::graphemes::init_tag(input);
    let legacy = mech_syntax::tag("λ")(mech_syntax::ParseString::new(&graphemes))
      .ok()
      .map(|(remaining, _)| {
        TextSize(
          graphemes[..remaining.cursor]
            .iter()
            .map(|grapheme| grapheme.len() as u32)
            .sum(),
        )
      });
    assert_eq!(
      canonical.matched.then_some(canonical.consumed.end),
      legacy,
      "tag boundary mismatch for {input:?}"
    );
  }
}

#[test]
fn paired_lexical_boundaries_remain_distinct() {
  for (tight, spaced) in [("a-b", "a - b"), ("a/b", "a / b")] {
    assert_eq!(
      parse(tight, rules::IDENTIFIER).consumed,
      source(tight).full_range()
    );
    assert_eq!(parse(spaced, rules::IDENTIFIER).consumed.end, TextSize(1));
  }

  for source_text in ["1/2", "1 / 2"] {
    assert_eq!(
      parse(source_text, rules::DIGIT_SEQUENCE).consumed.end,
      TextSize(1)
    );
  }

  for (input, rule) in [
    (":", rules::COLON),
    ("=", rules::EQUAL),
    (":=", rules::DEFINE_OPERATOR),
  ] {
    assert_eq!(parse(input, rule).consumed, source(input).full_range());
  }
}

#[test]
fn whitespace_classes_preserve_the_frozen_distinctions() {
  let whitespace = parse(" \t\r\nx", rules::WHITESPACE0);
  assert!(whitespace.matched);
  assert_eq!(whitespace.consumed.end, TextSize(4));

  let whitespace_nbsp = parse("\u{00a0}", rules::WHITESPACE0);
  assert!(whitespace_nbsp.matched);
  assert!(whitespace_nbsp.consumed.is_empty());

  let horizontal = parse("\u{00a0}\u{2009}\tx", rules::SPACE_TAB0);
  assert_eq!(horizontal.consumed.end, TextSize(6));

  let horizontal_newline = parse("\n", rules::WS0E);
  assert!(horizontal_newline.matched);
  assert!(horizontal_newline.consumed.is_empty());

  assert!(!parse("", rules::WHITESPACE1).matched);
  assert!(!parse("\n", rules::SPACE_TAB1).matched);
  assert_eq!(parse("\r\n  x", rules::NEWLINE_INDENT).consumed.end, TextSize(4));
}

#[test]
fn identifiers_and_path_segments_keep_legacy_boundaries() {
  for accepted in ["a-b", "a/b", "A*", "Δx^2", "💡"] {
    let parsed = parse(accepted, rules::IDENTIFIER);
    assert!(parsed.matched, "{accepted:?}");
    assert_eq!(parsed.consumed.end, TextSize(accepted.len() as u32));
    assert_eq!(
      parsed.syntax().first_child(SyntaxKind::Identifier).unwrap().text().unwrap(),
      accepted
    );
  }
  for rejected in ["1a", "_a", ":a", "=a", "(a"] {
    let parsed = parse(rejected, rules::IDENTIFIER);
    assert!(!parsed.matched, "{rejected:?}");
    assert!(parsed.consumed.is_empty());
  }
  assert_eq!(parse("a:b", rules::IDENTIFIER).consumed.end, TextSize(1));

  for accepted in ["a-b", "Δ2", "💡x"] {
    let parsed = parse(accepted, rules::IDENTIFIER_PATH_SEGMENT);
    assert!(parsed.matched, "{accepted:?}");
    assert_eq!(parsed.consumed.end, TextSize(accepted.len() as u32));
  }
  for bounded in ["a/b", "a*b", "a,b", "a:b", "a=b", "a_b"] {
    assert_eq!(
      parse(bounded, rules::IDENTIFIER_PATH_SEGMENT).consumed.end,
      TextSize(1),
      "{bounded:?}"
    );
  }
}

#[test]
fn grapheme_primitives_and_emoji_exclusions_are_exact() {
  let alpha = source("e").append("\u{301}").unwrap();
  let alpha = parse_canonical_base_rule_for_test(alpha, rules::ALPHA, ParseConfig::default())
    .unwrap();
  assert!(alpha.matched);
  assert_eq!(alpha.consumed.end, TextSize(3));

  let digit = source("1").append("\u{20e3}").unwrap();
  let digit = parse_canonical_base_rule_for_test(digit, rules::DIGIT, ParseConfig::default())
    .unwrap();
  assert!(digit.matched);
  assert_eq!(digit.consumed.end, TextSize(4));

  let family = source("👨").append("\u{200d}").unwrap().append("👩").unwrap();
  let emoji =
    parse_canonical_base_rule_for_test(family, rules::EMOJI_GRAPHEME, ParseConfig::default())
      .unwrap();
  assert!(emoji.matched);
  assert_eq!(emoji.consumed.end, emoji.source.byte_len());

  for forbidden in [
    "┌", "\u{00a0}", "\u{2009}", "⸢", "⸥", "⟨", "⟩",
  ] {
    assert!(!parse(forbidden, rules::EMOJI).matched, "{forbidden:?}");
    assert!(parse(forbidden, rules::FORBIDDEN_EMOJI).matched);
  }
  assert!(parse("💡", rules::EMOJI).matched);
}

#[test]
fn structural_base_rules_retain_lossless_children() {
  let digits = parse("1_2_٣", rules::DIGIT_SEQUENCE);
  assert!(digits.matched);
  assert_eq!(digits.consumed.end, digits.source.byte_len());
  assert!(
    digits
      .syntax()
      .first_child(SyntaxKind::DigitSequence)
      .is_some()
  );
  assert_eq!(parse("1_", rules::DIGIT_SEQUENCE).consumed.end, TextSize(1));

  let escaped = parse("\\n", rules::ESCAPED_CHAR);
  assert!(escaped.matched);
  let escaped_node = escaped
    .syntax()
    .first_child(SyntaxKind::EscapedCharacter)
    .unwrap();
  assert_eq!(escaped_node.text().unwrap(), "\\n");
  assert_eq!(
    escaped_node.tokens().last().unwrap().kind(),
    SyntaxKind::EscapedChar
  );

  assert_eq!(parse("\\n", rules::TEXT).consumed.end, TextSize(2));
  assert_eq!(parse("\\n", rules::RAW_TEXT).consumed.end, TextSize(1));
}

#[test]
fn escaped_character_compatibility_values_match_the_legacy_parser() {
  for input in ["\\n", "\\t", "\\r", "\\a", "\\!", "\\\\", "\\e\u{301}"] {
    let parsed = parse(input, rules::ESCAPED_CHAR);
    assert!(parsed.matched, "{input:?}");
    assert_eq!(parsed.consumed.end, TextSize(input.len() as u32));
    validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
    assert_eq!(
      reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
      input
    );
    let syntax = parsed
      .syntax()
      .first_child(SyntaxKind::EscapedCharacter)
      .expect("escaped-char node");
    assert_eq!(syntax.text().unwrap(), input);
    let canonical = lower_legacy_escaped_character(&syntax).unwrap();

    let graphemes = mech_syntax::graphemes::init_tag(input);
    let (remaining, legacy) =
      mech_syntax::escaped_char(mech_syntax::ParseString::new(&graphemes)).unwrap();
    assert_eq!(remaining.cursor, graphemes.len(), "{input:?}");
    assert!(remaining.error_log.is_empty(), "{input:?}");
    assert_eq!(canonical, legacy, "{input:?}");
  }
}
