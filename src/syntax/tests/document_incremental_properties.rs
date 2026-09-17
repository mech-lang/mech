use mech_syntax::document::{
    DocumentId, DocumentSession, ParseConfig, SyntaxKind, TextEdit, TextRange, TextSize,
    TextSnapshot, compact_debug_tree, normalize_diagnostics, parse_document, reconstruct_source,
    validate_lossless,
};
use proptest::prelude::*;

fn unicode_string(max_chars: usize) -> impl Strategy<Value = String> {
    proptest::collection::vec(any::<char>(), 0..=max_chars)
        .prop_map(|characters| characters.into_iter().collect())
}

fn assert_equivalent(session: &DocumentSession) {
    let incremental = session.snapshot();
    let full = parse_document(
        TextSnapshot::new(
            DocumentId(1),
            incremental.revision,
            incremental.source.to_contiguous_string().as_str(),
        )
        .unwrap(),
        ParseConfig::default(),
    );
    assert_eq!(
        compact_debug_tree(&incremental.syntax()),
        compact_debug_tree(&full.syntax())
    );
    assert_eq!(
        normalize_diagnostics(
            &incremental.diagnostics,
            incremental.revision,
            &incremental.nodes,
        ),
        normalize_diagnostics(&full.diagnostics, full.revision, &full.nodes)
    );
    validate_lossless(&incremental.root, &incremental.source).unwrap();
    assert_eq!(
        reconstruct_source(&incremental.root, &incremental.source).unwrap(),
        incremental.source.to_contiguous_string()
    );
}

fn boundaries(text: &str) -> Vec<usize> {
    text.char_indices()
        .map(|(offset, _)| offset)
        .chain(core::iter::once(text.len()))
        .collect()
}

proptest! {
  #![proptest_config(ProptestConfig::with_cases(128))]

  #[test]
  fn random_edits_match_full_parse_after_every_revision(
    initial in unicode_string(128),
    edits in proptest::collection::vec(
      (any::<u16>(), any::<u16>(), unicode_string(24)),
      0..10,
    ),
  ) {
    let mut session = DocumentSession::new(&initial, ParseConfig::default());
    assert_equivalent(&session);
    for (raw_start, raw_end, insert) in edits {
      let text = session.snapshot().source.to_contiguous_string();
      let offsets = boundaries(&text);
      let first = offsets[raw_start as usize % offsets.len()];
      let second = offsets[raw_end as usize % offsets.len()];
      let (start, end) = if first <= second {
        (first, second)
      } else {
        (second, first)
      };
      session.apply_edits(&[TextEdit::replace(
        TextRange::new(TextSize(start as u32), TextSize(end as u32)),
        insert,
      )]);
      assert_equivalent(&session);
    }
  }

  #[test]
  fn unchanged_later_section_retains_identity(
    replacement in "[A-Za-z0-9 ]{0,32}",
  ) {
    let source =
      "editable paragraph\n1. Stable\n--------\nstable paragraph\n";
    let mut session = DocumentSession::new(source, ParseConfig::default());
    let later_section = session
      .snapshot()
      .nodes
      .nodes()
      .find(|(_, record)| {
        record.kind == SyntaxKind::Section && record.range.start.0 > 0
      })
      .map(|(id, _)| id)
      .unwrap();
    let end = source.find('\n').unwrap();
    session.apply_edits(&[TextEdit::replace(
      TextRange::new(TextSize(0), TextSize(end as u32)),
      replacement,
    )]);
    prop_assert!(session.snapshot().nodes.node(later_section).is_some());
    assert_equivalent(&session);
  }

  #[test]
  fn repeated_eof_appends_match_streaming(
    chunks in proptest::collection::vec(unicode_string(16), 0..12),
  ) {
    let mut session = DocumentSession::new("", ParseConfig::default());
    let mut expected = String::new();
    for chunk in chunks {
      expected.push_str(&chunk);
      let eof = session.snapshot().source.byte_len();
      session.apply_edits(&[TextEdit::insert(eof, chunk)]);
      prop_assert_eq!(session.snapshot().source.to_contiguous_string(), expected.as_str());
      assert_equivalent(&session);
    }
  }
}

#[test]
fn deleted_subtree_identity_is_removed() {
    let source = "delete paragraph\n1. Stable\n--------\nstable\n";
    let mut session = DocumentSession::new(source, ParseConfig::default());
    let deleted = session
        .snapshot()
        .nodes
        .nodes()
        .find(|(_, record)| record.kind == SyntaxKind::Paragraph)
        .map(|(id, _)| id)
        .unwrap();
    let end = source.find('\n').unwrap() + 1;
    session.apply_edits(&[TextEdit::delete(TextRange::new(
        TextSize::ZERO,
        TextSize(end as u32),
    ))]);
    assert!(session.snapshot().nodes.node(deleted).is_none());
    assert_equivalent(&session);
}

#[test]
fn removing_paragraph_newline_reparses_the_containing_section() {
    let mut session = DocumentSession::new("first\nsecond\n", ParseConfig::default());
    session.apply_edits(&[TextEdit::delete(TextRange::new(TextSize(5), TextSize(6)))]);
    assert_eq!(
        session.snapshot().source.to_contiguous_string(),
        "firstsecond\n"
    );
    assert_equivalent(&session);
    let paragraphs = session
        .snapshot()
        .nodes
        .nodes()
        .filter(|(_, record)| record.kind == SyntaxKind::Paragraph)
        .count();
    assert_eq!(paragraphs, 1);
}

#[test]
fn deleting_a_line_prefix_can_reclassify_an_underlined_subtitle() {
    let regression =
        include_str!("fixtures/document/fuzz-regressions/underlined-subtitle-reclassification.mec");
    let regression = regression
        .strip_suffix('\n')
        .expect("the checked-in fixture has a conventional final newline");
    let mut session = DocumentSession::new(&format!("00- {regression}"), ParseConfig::default());
    session.apply_edits(&[TextEdit::delete(TextRange::new(
        TextSize::ZERO,
        TextSize(4),
    ))]);
    assert_eq!(session.snapshot().source.to_contiguous_string(), regression);
    assert_equivalent(&session);
    assert!(
        session
            .snapshot()
            .nodes
            .nodes()
            .any(|(_, record)| record.kind == SyntaxKind::UlSubtitle)
    );
}

#[test]
fn editing_a_comment_line_can_reclassify_an_underlined_subtitle() {
    let regression =
        include_str!("fixtures/document/fuzz-regressions/comment-to-subtitle-reclassification.mec");
    let regression = regression
        .strip_suffix('\n')
        .expect("the checked-in fixture has a conventional final newline");
    let mut initial = regression.to_string();
    initial.insert(11, ')');
    let mut session = DocumentSession::new(&initial, ParseConfig::default());
    session.apply_edits(&[TextEdit::delete(TextRange::new(TextSize(11), TextSize(12)))]);
    assert_eq!(session.snapshot().source.to_contiguous_string(), regression);
    assert_equivalent(&session);
    assert!(
        session
            .snapshot()
            .nodes
            .nodes()
            .any(|(_, record)| record.kind == SyntaxKind::UlSubtitle)
    );
}

#[test]
fn removing_a_section_heading_merges_it_with_the_prior_section() {
    let regression = include_str!("fixtures/document/fuzz-regressions/removed-section-heading.mec");
    let mut session = DocumentSession::new("2\n1. Code\n-\n", ParseConfig::default());
    session.apply_edits(&[TextEdit::replace(
        TextRange::new(TextSize(2), TextSize(9)),
        "plain",
    )]);
    assert_eq!(session.snapshot().source.to_contiguous_string(), regression);
    assert_equivalent(&session);
    let sections = session
        .snapshot()
        .nodes
        .nodes()
        .filter(|(_, record)| record.kind == SyntaxKind::Section)
        .count();
    assert_eq!(sections, 1);
}

#[test]
fn inserting_a_line_prefix_can_reclassify_an_underlined_subtitle() {
    let regression =
        include_str!("fixtures/document/fuzz-regressions/heading-prefix-joins-section.mec");
    let initial = regression
        .strip_suffix('\n')
        .expect("the checked-in fixture has a conventional final newline");
    let mut session = DocumentSession::new(initial, ParseConfig::default());
    session.apply_edits(&[TextEdit::replace(
        TextRange::new(TextSize(32), TextSize(124)),
        "\n4. Output",
    )]);
    assert_equivalent(&session);
    session.apply_edits(&[TextEdit::replace(
        TextRange::new(TextSize(10), TextSize(45)),
        "-------------",
    )]);
    assert_equivalent(&session);
    let update = session.apply_edits(&[TextEdit::insert(
        TextSize(45),
        "-------------",
    )]);
    assert_eq!(update.stats.document_fallbacks, 1);
    assert_equivalent(&session);
    assert_eq!(
        session
            .snapshot()
            .nodes
            .nodes()
            .filter(|(_, record)| record.kind == SyntaxKind::Section)
            .count(),
        1
    );
}

#[test]
fn incomplete_definition_recovery_includes_the_following_line() {
    let _case_description =
        include_str!("fixtures/document/fuzz-regressions/incomplete-definition-following-nul.case");
    let mut session = DocumentSession::new("x := 1= 19+x := \n\0\0\0", ParseConfig::default());
    session.apply_edits(&[TextEdit::delete(TextRange::new(TextSize(2), TextSize(12)))]);
    assert_eq!(
        session.snapshot().source.to_contiguous_string(),
        "x  := \n\0\0\0"
    );
    assert_equivalent(&session);
    assert!(
        session
            .snapshot()
            .nodes
            .nodes()
            .any(|(_, record)| record.kind == SyntaxKind::Error)
    );
}

#[test]
fn deleting_whitespace_can_join_a_definition_across_lines() {
    let regression = include_str!("fixtures/document/fuzz-regressions/cross-line-definition.mec");
    let regression = regression
        .strip_suffix('\n')
        .expect("the checked-in fixture has a conventional final newline");
    let mut session = DocumentSession::new("x :=2", ParseConfig::default());
    session.apply_edits(&[TextEdit::insert(TextSize(2), "3\n")]);
    assert_equivalent(&session);
    session.apply_edits(&[TextEdit::delete(TextRange::new(TextSize(1), TextSize(2)))]);
    assert_eq!(session.snapshot().source.to_contiguous_string(), regression);
    assert_equivalent(&session);
    assert!(
        session
            .snapshot()
            .nodes
            .nodes()
            .any(|(_, record)| record.kind == SyntaxKind::VariableDefine)
    );
}
