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

#[test]
fn every_fixed_terminal_executes_its_exact_phase_0_contract() {
  assert_eq!(FIXED_TERMINAL_COUNT, 108);
  assert_eq!(FIXED_TERMINALS.len(), FIXED_TERMINAL_COUNT);

  let phase_0 = fs::read_to_string(
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/grammar_conformance.rs"),
  )
  .expect("read Phase 0 terminal contracts");
  let table = phase_0
    .split_once("const FIXED_TERMINAL_CONTRACTS")
    .expect("Phase 0 fixed-terminal table")
    .1
    .split_once("];")
    .expect("end of Phase 0 fixed-terminal table")
    .0;
  assert_eq!(
    table.matches("terminal_contract!(").count(),
    FIXED_TERMINAL_COUNT,
    "canonical table must not weaken the existing Phase 0 contract count"
  );

  for spec in FIXED_TERMINALS {
    let name = canonical_rule_name(spec.rule).expect("registered terminal RuleId");
    assert!(
      table.contains(&format!("\"base.{name}\"")),
      "canonical terminal {name} is absent from the Phase 0 contract table"
    );
    let contract_start = table
      .find(&format!("\"base.{name}\""))
      .expect("Phase 0 terminal contract");
    let contract_tail = &table[contract_start..];
    let contract_end = contract_tail[1..]
      .find("terminal_contract!(")
      .map_or(contract_tail.len(), |offset| offset + 1);
    assert!(
      contract_tail[..contract_end].contains(expected_legacy_kind(spec.kind)),
      "canonical terminal {name} does not retain its Phase 0 token kind"
    );
    assert_fixed_terminal(spec);
  }
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
