use mech_core::{Grammar, GrammarExpression, Token};
use mech_syntax::document::parser::canonical::{
    parse_canonical_base_rule_for_test, parse_canonical_tag_for_test,
};
use mech_syntax::document::parser::{CANONICAL_PORTS, PortPhase, RuleFamily, rules};
use mech_syntax::document::{
    DocumentId, FragmentKind, IdGenerator, ParseConfig, ParseContext, RecoveryAction, Revision,
    SyntaxKind, TextRange, TextSize, TextSnapshot, compact_debug_tree, lower_legacy_grammar,
    parse_canonical_grammar, parse_fragment, reconstruct_source, validate_lossless,
    validate_lossless_range,
};
use proptest::prelude::*;

fn parse(text: &str) -> mech_syntax::document::SyntaxSnapshot {
    parse_canonical_grammar(
        TextSnapshot::new(DocumentId(44), Revision(0), text).unwrap(),
        ParseConfig::default(),
    )
}

fn normalize_token_source(token: &mut Token) {
    token.src_range = Default::default();
}

fn normalize_expression_source(expression: &mut GrammarExpression) {
    match expression {
        GrammarExpression::Choice(items) | GrammarExpression::Sequence(items) => {
            for item in items {
                normalize_expression_source(item);
            }
        }
        GrammarExpression::Definition(identifier) => normalize_token_source(&mut identifier.name),
        GrammarExpression::Group(item)
        | GrammarExpression::Not(item)
        | GrammarExpression::Optional(item)
        | GrammarExpression::Peek(item)
        | GrammarExpression::Repeat0(item)
        | GrammarExpression::Repeat1(item) => normalize_expression_source(item),
        GrammarExpression::List(first, second) => {
            normalize_expression_source(first);
            normalize_expression_source(second);
        }
        GrammarExpression::Range(start, end) => {
            normalize_token_source(start);
            normalize_token_source(end);
        }
        GrammarExpression::Terminal(token) => normalize_token_source(token),
    }
}

fn normalize_grammar_source(mut grammar: Grammar) -> Grammar {
    for rule in &mut grammar.rules {
        normalize_token_source(&mut rule.name.name);
        normalize_expression_source(&mut rule.expr);
    }
    grammar
}

proptest! {
  #![proptest_config(ProptestConfig {
    cases: 128,
    rng_seed: proptest::test_runner::RngSeed::Fixed(0x2a_540_167),
    ..ProptestConfig::default()
  })]

  #[test]
  fn generated_terminal_rules_are_lossless_and_match_legacy_lowering(
    identifier in "[a-z][a-z0-9-]{0,12}",
    terminal in "[A-Za-z0-9.,!]{1,20}",
    leading_space in any::<bool>(),
    inner_space in any::<bool>(),
  ) {
    let leading = if leading_space { " \t" } else { "" };
    let inner = if inner_space { " \n" } else { "" };
    let text = format!(
      "{leading}{identifier}{inner}:={inner}\"{terminal}\"{inner};{leading}"
    );
    let snapshot = parse(&text);
    prop_assert!(
      snapshot.diagnostics.is_empty(),
      "{:?}",
      snapshot.diagnostics.as_slice()
    );
    prop_assert_eq!(
      reconstruct_source(&snapshot.root, &snapshot.source).unwrap(),
      text.as_str()
    );
    validate_lossless(&snapshot.root, &snapshot.source).unwrap();
    let lowered = lower_legacy_grammar(&snapshot).unwrap();
    prop_assert_eq!(lowered.rules.len(), 1);
    prop_assert_eq!(lowered.rules[0].name.to_string(), identifier);
    let legacy = mech_syntax::parse_grammar(&text).unwrap();
    prop_assert_eq!(
      normalize_grammar_source(lowered),
      normalize_grammar_source(legacy)
    );
  }

  #[test]
  fn arbitrary_short_grammar_is_total_lossless_and_diagnostic_ranges_are_bounded(
    characters in proptest::collection::vec(any::<char>(), 0..96),
  ) {
    let text = characters.into_iter().collect::<String>();
    let snapshot = parse(&text);
    prop_assert_eq!(
      reconstruct_source(&snapshot.root, &snapshot.source).unwrap(),
      text
    );
    validate_lossless(&snapshot.root, &snapshot.source).unwrap();
    prop_assert!(snapshot.stats.parser_steps <= ParseConfig::default().limits.fuel);
    prop_assert!(
      snapshot.stats.events_emitted
        <= u64::from(ParseConfig::default().limits.max_events)
    );
    let source_range = snapshot.source.full_range();
    for diagnostic in snapshot.diagnostics.iter() {
      let primary = diagnostic
        .primary
        .resolve(snapshot.revision, &snapshot.nodes);
      prop_assert!(primary.is_some(), "unresolvable diagnostic: {diagnostic:?}");
      prop_assert!(source_range.contains_range(primary.unwrap()));
      for label in &diagnostic.labels {
        let range = label.anchor.resolve(snapshot.revision, &snapshot.nodes);
        prop_assert!(range.is_some(), "unresolvable label: {label:?}");
        prop_assert!(source_range.contains_range(range.unwrap()));
      }
      for fix in &diagnostic.fixes {
        for edit in &fix.edits {
          prop_assert!(source_range.contains_range(edit.delete));
        }
      }
      match diagnostic.recovery.as_ref() {
        Some(RecoveryAction::Insert { at, .. })
        | Some(RecoveryAction::Abandon { at, .. }) => {
          prop_assert!(source_range.contains_inclusive(*at));
        }
        Some(RecoveryAction::Skip { range })
        | Some(RecoveryAction::ResourceLimit { range }) => {
          prop_assert!(source_range.contains_range(*range));
        }
        None => {}
      }
    }
  }

  #[test]
  fn every_canonical_lexical_rule_is_total_for_short_utf8(
    characters in proptest::collection::vec(any::<char>(), 0..16),
  ) {
    let text = characters.into_iter().collect::<String>();
    let source =
      TextSnapshot::new(DocumentId(45), Revision(0), text.as_str()).unwrap();
    let ports = CANONICAL_PORTS
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
    prop_assert_eq!(ports.len(), 149);

    for port in ports {
      let parsed = if port.rule == rules::TAG {
        parse_canonical_tag_for_test(source.clone(), "x", ParseConfig::default())
      } else {
        parse_canonical_base_rule_for_test(
          source.clone(),
          port.rule,
          ParseConfig::default(),
        )
        .expect("every selected lexical rule has a canonical implementation")
      };
      prop_assert_eq!(parsed.consumed.start, TextSize::ZERO);
      prop_assert!(parsed.consumed.end <= parsed.source.byte_len());
      validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
    }
  }

  #[test]
  fn generated_rule_fragments_match_whole_grammar_trees(
    identifier in "[a-z][a-z0-9-]{0,12}",
    terminal in "[A-Za-z0-9.,!]{1,20}",
  ) {
    let rule = format!("{identifier}:=\"{terminal}\";");
    let suffix = "next:=\"z\";";
    let prefix = "physical-prefix:";
    let physical = format!("{prefix}{rule}{suffix}");
    let range = TextRange::new(
      TextSize(prefix.len() as u32),
      TextSize((prefix.len() + rule.len()) as u32),
    );
    let source =
      TextSnapshot::new(DocumentId(46), Revision(0), physical).unwrap();
    let mut ids = IdGenerator::new();
    let fragment = parse_fragment(
      &source,
      range,
      FragmentKind::GrammarRule,
      ParseContext::for_kind(FragmentKind::GrammarRule),
      ParseConfig::default(),
      &mut ids,
    );
    prop_assert!(fragment.matched);
    prop_assert!(fragment.consumed_complete);
    prop_assert_eq!(fragment.consumed, range);
    validate_lossless_range(&fragment.root, &fragment.source, range).unwrap();

    let whole = parse(&format!("{rule}{suffix}"));
    prop_assert!(
      whole.diagnostics.is_empty(),
      "{:?}",
      whole.diagnostics.as_slice()
    );
    let grammar = whole
      .syntax()
      .first_child(SyntaxKind::Grammar)
      .expect("whole parse grammar");
    let expected = grammar
      .first_child(SyntaxKind::GrammarRule)
      .expect("whole parse grammar rule");
    prop_assert_eq!(
      compact_debug_tree(&fragment.syntax()),
      compact_debug_tree(&expected)
    );
  }
}

#[test]
fn parser_work_grows_linearly_with_rule_count() {
    let measurements = [16_usize, 32, 64, 128]
        .into_iter()
        .map(|rules| {
            let text = (0..rules)
                .map(|index| format!("r{index} := \"x\";"))
                .collect::<String>();
            let snapshot = parse(&text);
            assert!(snapshot.diagnostics.is_empty());
            (
                rules,
                snapshot.stats.parser_steps,
                snapshot.stats.events_emitted,
            )
        })
        .collect::<Vec<_>>();

    for pair in measurements.windows(2) {
        let (smaller_rules, smaller_steps, smaller_events) = pair[0];
        let (larger_rules, larger_steps, larger_events) = pair[1];
        assert_eq!(larger_rules, smaller_rules * 2);
        assert!(
            larger_steps <= smaller_steps * 2 + 64,
            "parser steps were not linear: {measurements:?}"
        );
        assert!(
            larger_events <= smaller_events * 2 + 64,
            "parser events were not linear: {measurements:?}"
        );
    }
}
