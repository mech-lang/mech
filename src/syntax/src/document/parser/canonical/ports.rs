use crate::document::{
  ParseConfig, RuleId, SyntaxKind, TextRange, TextSize, TextSnapshot,
};

use super::base;
use super::combinator::Attempt;
use super::test_support::{CanonicalSourceRuleSnapshot, parse_source_rule_prefix};

/// Backwards-compatible name for Phase 2A direct-rule snapshots.
pub type CanonicalRuleSnapshot = CanonicalSourceRuleSnapshot;

#[doc(hidden)]
pub fn canonical_base_rule_supported(rule: RuleId) -> bool {
  base::supports(rule)
}

/// Parses one canonical base production for deterministic contract tests.
///
/// `tag` is caller-parameterized and therefore uses
/// [`parse_canonical_tag_for_test`] instead.
#[doc(hidden)]
pub fn parse_canonical_base_rule_for_test(
  source: TextSnapshot,
  rule: RuleId,
  config: ParseConfig,
) -> Option<CanonicalRuleSnapshot> {
  if rule == super::super::rules::TAG || !base::supports(rule) {
    return None;
  }
  Some(parse_source_rule_prefix(source, rule, config, |parser| {
    base::parse_rule(parser, rule)
      .then_some(Attempt::Matched)
      .unwrap_or(Attempt::NoMatch)
  }))
}

/// Parses the caller-supplied exact `tag` production for deterministic tests.
#[doc(hidden)]
pub fn parse_canonical_tag_for_test(
  source: TextSnapshot,
  literal: &str,
  config: ParseConfig,
) -> CanonicalRuleSnapshot {
  parse_source_rule_prefix(
    source,
    super::super::rules::TAG,
    config,
    |parser| {
      base::parse_exact_tag(parser, literal, SyntaxKind::Text)
        .then_some(Attempt::Matched)
        .unwrap_or(Attempt::NoMatch)
    },
  )
}

#[cfg(test)]
mod tests {
  use crate::document::{DocumentId, FoundSyntax, ParseLimits, Revision};

  use super::*;

  #[test]
  fn unsupported_and_parameterized_rules_are_distinct() {
    let source = TextSnapshot::new(DocumentId(1), Revision(0), "x").unwrap();
    assert!(
      parse_canonical_base_rule_for_test(
        source.clone(),
        super::super::super::rules::GRAMMAR,
        ParseConfig::default(),
      )
      .is_none()
    );
    assert!(
      parse_canonical_base_rule_for_test(
        source.clone(),
        super::super::super::rules::TAG,
        ParseConfig::default(),
      )
      .is_none()
    );
    let parsed = parse_canonical_tag_for_test(source, "x", ParseConfig::default());
    assert!(parsed.matched);
    assert_eq!(parsed.consumed, TextRange::new(TextSize::ZERO, TextSize(1)));
  }

  #[test]
  fn direct_base_rules_classify_resource_diagnostics_from_physical_source() {
    let source = TextSnapshot::new(DocumentId(1), Revision(0), " \n@").unwrap();
    let parsed = parse_source_rule_prefix(
      source,
      super::super::super::rules::ALPHA,
      ParseConfig {
        limits: ParseLimits {
          max_events: 8,
          ..ParseLimits::default()
        },
      },
      |parser| {
        parser.halt();
        parser.consume_resource_remainder();
        Attempt::Committed
      },
    );

    let diagnostic = parsed.diagnostics.iter().next().unwrap();
    assert_eq!(diagnostic.rule, Some(super::super::super::rules::ALPHA));
    assert_eq!(
      diagnostic.found,
      Some(FoundSyntax {
        kind: Some(SyntaxKind::Whitespace),
        text: Some(" ".into()),
      })
    );
  }
}
