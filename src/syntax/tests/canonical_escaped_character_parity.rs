use mech_syntax::document::parser::canonical::{
  CanonicalRuleSnapshot, parse_canonical_base_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
  DocumentId, ParseConfig, Revision, TextRange, TextSnapshot,
};

const SYMBOL_VALUES: &[&str] = &[
  "&", "`", "$", "|", "%", "@", "/", "#", "=", "\\", "~", "+", "-", "*",
  "^", "_",
];
const PUNCTUATION_VALUES: &[&str] =
  &[".", "!", "?", ",", ":", ";", "\"", "'"];
const NONALPHABETIC_FAMILIES: &[(&str, &[&str])] = &[
  ("symbol", SYMBOL_VALUES),
  ("punctuation", PUNCTUATION_VALUES),
];
const GRAPHEME_EXTENSIONS: &[(&str, &str)] = &[
  ("combining mark", "\u{301}"),
  ("text variation selector", "\u{fe0e}"),
  ("emoji variation selector", "\u{fe0f}"),
];

fn canonical(input: &str) -> CanonicalRuleSnapshot {
  let source =
    TextSnapshot::new(DocumentId(72), Revision(0), input).unwrap();
  parse_canonical_base_rule_for_test(
    source,
    rules::ESCAPED_CHAR,
    ParseConfig::default(),
  )
  .expect("escaped-char has a canonical Phase 2A port")
}

fn legacy_accepts_whole(input: &str) -> bool {
  let graphemes = mech_syntax::graphemes::init_tag(input);
  mech_syntax::escaped_char(mech_syntax::ParseString::new(&graphemes))
    .is_ok_and(|(remaining, _)| {
      remaining.cursor == graphemes.len()
        && remaining.error_log.is_empty()
    })
}

fn assert_parity(input: &str, expected: bool, case: &str) {
  let parsed = canonical(input);
  let canonical_accepts_whole = parsed.matched
    && parsed.diagnostics.is_empty()
    && parsed.consumed == parsed.source.full_range();
  let legacy_accepts_whole = legacy_accepts_whole(input);

  assert_eq!(
    legacy_accepts_whole, expected,
    "legacy expectation for {case}: {input:?}"
  );
  assert_eq!(
    canonical_accepts_whole, legacy_accepts_whole,
    "canonical/legacy parity for {case}: {input:?}"
  );
  if !expected {
    assert_eq!(
      parsed.consumed,
      TextRange::empty(parsed.source.full_range().start),
      "a rejected escaped character must not consume input: {case}: {input:?}"
    );
  }
}

#[test]
fn every_nonalphabetic_escaped_value_matches_legacy_acceptance() {
  for (family, values) in NONALPHABETIC_FAMILIES {
    for value in *values {
      let input = format!("\\{value}");
      assert_parity(&input, true, family);
    }
  }
}

#[test]
fn extended_nonalphabetic_graphemes_match_legacy_rejection() {
  for (family, values) in NONALPHABETIC_FAMILIES {
    for value in *values {
      for (extension_name, extension) in GRAPHEME_EXTENSIONS {
        let input = format!("\\{value}{extension}");
        let case = format!("{family} with {extension_name}");
        assert_parity(&input, false, &case);
      }
    }
  }
}

#[test]
fn alphabetic_escaped_value_keeps_its_complete_grapheme() {
  assert_parity("\\e\u{301}", true, "alphabetic combining grapheme");
}
