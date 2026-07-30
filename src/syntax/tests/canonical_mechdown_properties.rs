use mech_syntax::document::parser::canonical::parse_canonical_mechdown_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DocumentId, ParseConfig, Revision, RuleId, SyntaxKind, TextSize, TextSnapshot,
    reconstruct_source_range, validate_lossless_range,
};
use proptest::prelude::*;

const CLOSED_RULES: &[RuleId] = &[
    rules::COMMENT_SIGIL,
    rules::COMMENT,
    rules::CODEBLOCK_SIGIL,
    rules::INLINE_CODE,
    rules::INLINE_EQUATION,
    rules::RAW_HYPERLINK,
    rules::FOOTNOTE_REFERENCE,
    rules::REFERENCE,
    rules::SECTION_REFERENCE,
    rules::PARAGRAPH_TEXT,
    rules::THEMATIC_BREAK,
    rules::BLANK_LINE,
    rules::EQUATION,
];

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(204), Revision(0), text).unwrap()
}

fn find_node(
    root: &mech_syntax::document::SyntaxNode,
    kind: SyntaxKind,
) -> Option<mech_syntax::document::SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

proptest! {
  #![proptest_config(ProptestConfig {
    cases: 128,
    rng_seed: proptest::test_runner::RngSeed::Fixed(0x2b_540_180),
    ..ProptestConfig::default()
  })]

  #[test]
  fn every_direct_closed_fragment_is_total_lossless_and_bounded(
    characters in proptest::collection::vec(any::<char>(), 0..64),
  ) {
    let text = characters.into_iter().collect::<String>();
    for rule in CLOSED_RULES {
      let parsed = parse_canonical_mechdown_rule_for_test(
        source(&text),
        *rule,
        ParseConfig::default(),
      ).expect("every Phase 2B rule has a direct fragment parser");
      prop_assert_eq!(parsed.rule, *rule);
      prop_assert_eq!(parsed.syntax().kind(), SyntaxKind::CanonicalFragment);
      prop_assert_eq!(parsed.consumed.start, TextSize::ZERO);
      prop_assert!(parsed.consumed.end <= parsed.source.byte_len());
      prop_assert!(
        parsed.stats.parser_steps <= ParseConfig::default().limits.fuel,
        "{rule:?} exceeded its parser fuel on {text:?}"
      );
      prop_assert!(
        parsed.stats.events_emitted <= u64::from(ParseConfig::default().limits.max_events),
        "{rule:?} exceeded its event budget on {text:?}"
      );
      prop_assert!(validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).is_ok());
      let reconstructed =
        reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
      let consumed = parsed.source.text(parsed.consumed).unwrap();
      prop_assert_eq!(reconstructed, consumed);
    }
  }

  #[test]
  fn direct_comments_keep_generated_content_raw_and_stop_before_a_newline(
    content in "[A-Za-z0-9 ]{0,48}",
  ) {
    let text = format!("//{content}\nnext");
    let parsed = parse_canonical_mechdown_rule_for_test(
      source(&text),
      rules::COMMENT,
      ParseConfig::default(),
    ).unwrap();
    prop_assert!(parsed.is_strictly_clean());
    prop_assert_eq!(parsed.consumed.end, TextSize((2 + content.len()) as u32));
    let comment = find_node(&parsed.syntax(), SyntaxKind::Comment)
      .expect("direct comment fragment emits Comment");
    prop_assert_eq!(comment.text().unwrap(), format!("//{content}"));
    prop_assert_eq!(
      parsed.source.text(mech_syntax::document::TextRange::new(
        parsed.consumed.end,
        parsed.source.byte_len(),
      )).unwrap(),
      "\nnext",
    );
  }

  #[test]
  fn line_fragments_never_accept_an_eof_substitute_for_a_newline(
    horizontal in "[ \\t]{0,24}",
    stars in "\\*{1,24}",
  ) {
    let blank = parse_canonical_mechdown_rule_for_test(
      source(&horizontal),
      rules::BLANK_LINE,
      ParseConfig::default(),
    ).unwrap();
    prop_assert!(!blank.matched);
    prop_assert!(blank.diagnostics.is_empty());

    let thematic_text = format!("{stars}{horizontal}");
    let thematic = parse_canonical_mechdown_rule_for_test(
      source(&thematic_text),
      rules::THEMATIC_BREAK,
      ParseConfig::default(),
    ).unwrap();
    prop_assert!(!thematic.matched);
    prop_assert!(thematic.diagnostics.is_empty());
  }
}
