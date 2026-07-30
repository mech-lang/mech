use mech_syntax::document::parser::Cursor;
use mech_syntax::document::parser::canonical::{
  parse_canonical_base_rule_for_test, parse_canonical_tag_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
  DocumentId, ParseConfig, Revision, RuleId, TextRange, TextSize, TextSnapshot,
};
use mech_syntax::{ParseResult, ParseString};
use proptest::prelude::*;

fn piece_snapshot(parts: &[&str]) -> TextSnapshot {
  let mut source =
    TextSnapshot::new(DocumentId(201), Revision(0), "").unwrap();
  for part in parts {
    source = source.append((*part).to_owned()).unwrap();
  }
  assert_eq!(source.piece_count(), parts.len());
  source
}

fn assert_one_cross_piece_grapheme(parts: &[&str]) {
  let source = piece_snapshot(parts);
  let expected = source.full_range();
  let mut cursor = Cursor::new(&source);

  assert_eq!(cursor.context_peek_grapheme_range(), Some(expected));
  assert_eq!(cursor.peek_grapheme_range(), Some(expected));
  assert_eq!(cursor.bump_grapheme(), Some(expected));
  assert!(cursor.is_eof());
  assert_eq!(cursor.peek_grapheme_range(), None);
}

fn scalar_piece_snapshot(text: &str) -> TextSnapshot {
  let parts = text
    .chars()
    .map(|character| character.to_string())
    .collect::<Vec<_>>();
  let parts = parts.iter().map(String::as_str).collect::<Vec<_>>();
  piece_snapshot(&parts)
}

#[test]
fn combining_sequence_crosses_pieces() {
  assert_one_cross_piece_grapheme(&["e", "\u{301}"]);
}

#[test]
fn emoji_variation_selector_crosses_pieces() {
  assert_one_cross_piece_grapheme(&["\u{2764}", "\u{fe0f}"]);
}

#[test]
fn zwj_family_emoji_crosses_every_piece() {
  assert_one_cross_piece_grapheme(&[
    "\u{1f468}",
    "\u{200d}",
    "\u{1f469}",
    "\u{200d}",
    "\u{1f467}",
    "\u{200d}",
    "\u{1f466}",
  ]);
}

#[test]
fn regional_indicator_flag_crosses_pieces() {
  assert_one_cross_piece_grapheme(&["\u{1f1fa}", "\u{1f1f8}"]);
}

#[test]
fn crlf_crosses_pieces() {
  assert_one_cross_piece_grapheme(&["\r", "\n"]);
}

#[test]
fn ordinary_ascii_preserves_separate_graphemes_across_pieces() {
  let source = piece_snapshot(&["a", "b"]);
  let mut cursor = Cursor::new(&source);

  assert_eq!(
    cursor.bump_grapheme(),
    Some(TextRange::new(TextSize(0), TextSize(1)))
  );
  assert_eq!(
    cursor.bump_grapheme(),
    Some(TextRange::new(TextSize(1), TextSize(2)))
  );
  assert!(cursor.is_eof());
}

#[test]
fn consume_bound_does_not_split_a_context_grapheme() {
  let source = piece_snapshot(&["e", "\u{301}", "x"]);
  let mut cursor = Cursor::for_range(
    &source,
    TextRange::new(TextSize::ZERO, TextSize(1)),
  );
  let complete = TextRange::new(TextSize::ZERO, TextSize(3));

  assert_eq!(cursor.context_peek_grapheme_range(), Some(complete));
  assert_eq!(cursor.peek_grapheme_range(), None);
  assert_eq!(cursor.bump_grapheme(), None);
  assert_eq!(cursor.offset(), TextSize::ZERO);
}

#[test]
fn exact_tags_compare_complete_graphemes_across_piece_boundaries() {
  let literal = "e\u{301}b\u{2764}\u{fe0f}";
  let parsed = parse_canonical_tag_for_test(
    piece_snapshot(&["e", "\u{301}", "b", "\u{2764}", "\u{fe0f}"]),
    literal,
    ParseConfig::default(),
  );
  assert!(parsed.matched);
  assert_eq!(parsed.consumed, parsed.source.full_range());
  assert_eq!(parsed.syntax().tokens()[0].text().unwrap(), literal);

  let prefix = parse_canonical_tag_for_test(
    piece_snapshot(&["e", "\u{301}", "b"]),
    "e",
    ParseConfig::default(),
  );
  assert!(!prefix.matched);
  assert_eq!(prefix.consumed, TextRange::empty(TextSize::ZERO));
}

type LegacyParser = for<'source> fn(ParseString<'source>) -> ParseResult<'source, String>;

fn assert_canonical_legacy_extent(
  parts: &[&str],
  rule: RuleId,
  legacy: LegacyParser,
) {
  let source = piece_snapshot(parts);
  let text = parts.concat();
  let parsed =
    parse_canonical_base_rule_for_test(source, rule, ParseConfig::default()).unwrap();
  assert!(parsed.matched, "{:?} did not match {text:?}", rule);

  let graphemes = mech_syntax::graphemes::init_source(&text);
  let input = ParseString::new(&graphemes);
  let (_, legacy_text) = legacy(input).expect("legacy lexical contract");
  assert_eq!(
    parsed.consumed.len().to_usize(),
    legacy_text.len(),
    "{:?} consumed a different grapheme extent for {text:?}",
    rule
  );
  assert_eq!(
    parsed.source.text(parsed.consumed).unwrap(),
    legacy_text,
    "{:?} consumed different source text for {text:?}",
    rule
  );
}

#[test]
fn canonical_lexical_rules_match_legacy_cross_piece_grapheme_extents() {
  for parts in [
    &["e", "\u{301}"][..],
    &["\u{2764}", "\u{fe0f}"][..],
    &[
      "\u{1f468}",
      "\u{200d}",
      "\u{1f469}",
      "\u{200d}",
      "\u{1f467}",
      "\u{200d}",
      "\u{1f466}",
    ][..],
    &["\u{1f1fa}", "\u{1f1f8}"][..],
    &["\r", "\n"][..],
    &["a", "b"][..],
  ] {
    assert_canonical_legacy_extent(parts, rules::ANY, mech_syntax::any);
  }

  assert_canonical_legacy_extent(&["e", "\u{301}"], rules::ALPHA, mech_syntax::alpha);
  assert_canonical_legacy_extent(&["1", "\u{20e3}"], rules::DIGIT, mech_syntax::digit);
  for parts in [
    &["\u{2764}", "\u{fe0f}"][..],
    &[
      "\u{1f468}",
      "\u{200d}",
      "\u{1f469}",
      "\u{200d}",
      "\u{1f467}",
      "\u{200d}",
      "\u{1f466}",
    ][..],
    &["\u{1f1fa}", "\u{1f1f8}"][..],
  ] {
    assert_canonical_legacy_extent(parts, rules::EMOJI_GRAPHEME, mech_syntax::emoji_grapheme);
  }
}

proptest! {
  #![proptest_config(ProptestConfig {
    cases: 128,
    rng_seed: proptest::test_runner::RngSeed::Fixed(0x2a_6_128),
    ..ProptestConfig::default()
  })]

  #[test]
  fn scalar_piece_boundaries_preserve_canonical_grapheme_classification(
    cluster in prop::sample::select(vec![
      "e\u{301}",
      "1\u{20e3}",
      "\u{2764}\u{fe0f}",
      "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}\u{200d}\u{1f466}",
      "\u{1f1fa}\u{1f1f8}",
      "\r\n",
      "a",
    ]),
    suffix in prop::sample::select(vec!["", "x", "\u{301}", "💡"]),
  ) {
    let text = format!("{cluster}{suffix}");
    for rule in [
      rules::ANY,
      rules::ALPHA,
      rules::DIGIT,
      rules::EMOJI_GRAPHEME,
    ] {
      let contiguous = parse_canonical_base_rule_for_test(
        TextSnapshot::new(DocumentId(202), Revision(0), text.as_str()).unwrap(),
        rule,
        ParseConfig::default(),
      )
      .unwrap();
      let piecewise = parse_canonical_base_rule_for_test(
        scalar_piece_snapshot(&text),
        rule,
        ParseConfig::default(),
      )
      .unwrap();

      prop_assert_eq!(piecewise.matched, contiguous.matched);
      prop_assert_eq!(piecewise.consumed, contiguous.consumed);
      let contiguous_tokens = contiguous
        .syntax()
        .tokens()
        .into_iter()
        .map(|token| (token.kind(), token.range(), token.text().unwrap()))
        .collect::<Vec<_>>();
      let piecewise_tokens = piecewise
        .syntax()
        .tokens()
        .into_iter()
        .map(|token| (token.kind(), token.range(), token.text().unwrap()))
        .collect::<Vec<_>>();
      prop_assert_eq!(piecewise_tokens, contiguous_tokens);
    }
  }
}
