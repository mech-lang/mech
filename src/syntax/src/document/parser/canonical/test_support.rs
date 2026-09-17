//! Shared hidden support for direct canonical source-rule contract tests.
//!
//! These helpers deliberately parse physical source fragments. They are not
//! public parser roots and must not be used by production document parsing.

use alloc::sync::Arc;

use crate::document::{
    DiagnosticAnchor, DiagnosticStore, GreenNode, IdGenerator, NodeFlags, NodeIndex, ParseStats,
    RuleId, SyntaxKind, SyntaxNode, TextRange, TextSnapshot,
};

use super::super::{LexicalMode, ParseConfig, Parser, sink};
use super::super::rule::rules;
use super::combinator::Attempt;
use super::{kinds, literals, operators, paths};

/// A narrow prefix snapshot used to exercise canonical source productions.
#[derive(Clone, Debug)]
pub struct CanonicalSourceRuleSnapshot {
    pub source: TextSnapshot,
    pub rule: RuleId,
    pub root: Arc<GreenNode>,
    pub diagnostics: DiagnosticStore,
    pub nodes: NodeIndex,
    pub stats: ParseStats,
    pub matched: bool,
    pub consumed: TextRange,
}

impl CanonicalSourceRuleSnapshot {
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root_at(
            self.root.clone(),
            self.source.clone(),
            self.consumed.start,
        )
    }

    pub fn is_strictly_clean(&self) -> bool {
        self.matched
            && self.diagnostics.is_empty()
            && !self.root.flags.intersects(
                NodeFlags::ERROR
                    | NodeFlags::MISSING
                    | NodeFlags::CONTAINS_ERROR
                    | NodeFlags::CONTAINS_MISSING,
            )
    }
}

/// Parse a single canonical production as a deterministic source prefix.
///
/// This remains internal test support: callers select one already-closed
/// production and receive its lossless fragment snapshot. Physical source is
/// intentionally preserved, including whitespace for diagnostic reporting.
pub(crate) fn parse_source_rule_prefix(
    source: TextSnapshot,
    rule: RuleId,
    config: ParseConfig,
    parse: impl FnOnce(&mut Parser<'_>) -> Attempt,
) -> CanonicalSourceRuleSnapshot {
    let mut ids = IdGenerator::new();
    let mut parser = Parser::new(
        &source,
        LexicalMode::CanonicalSourceFragment,
        config,
        &mut ids,
    );
    parser.set_resource_rule(rule);
    let fragment = parser.start();
    let start = parser.offset();
    let matched = parse(&mut parser).accepted();
    let end = parser.offset();
    fragment.complete(&mut parser, SyntaxKind::CanonicalFragment);
    let output = parser.finish();
    let sink_result = sink(&output.events, &source, &mut ids)
        .expect("canonical source-rule events must form one root");

    let mut diagnostics = DiagnosticStore::new(source.revision());
    for mut pending in output.diagnostics {
        if let Some(event) = pending.event
            && let Some(node) = sink_result.event_nodes.get(&event)
        {
            pending.diagnostic.primary = DiagnosticAnchor::Element {
                element: crate::document::SyntaxElementId::Node(*node),
                relative: pending.relative,
            };
        }
        diagnostics.push(pending.diagnostic);
    }

    let consumed = TextRange::new(start, end);
    let nodes = NodeIndex::build_at(&sink_result.root, consumed.start);
    let mut stats = output.stats;
    stats.new_node_count = nodes.node_count() as u64;
    CanonicalSourceRuleSnapshot {
        source,
        rule,
        root: sink_result.root,
        diagnostics,
        nodes,
        stats,
        matched,
        consumed,
    }
}

/// Parse one exact Phase 2C production as a deterministic source prefix.
///
/// This hidden test surface deliberately exposes only the selected closed
/// island. It is not a production parser root and does not dispatch any
/// enclosing literal, kind, variable, expression, or document production.
#[doc(hidden)]
pub fn parse_canonical_phase_2c_rule_for_test(
    source: TextSnapshot,
    rule: RuleId,
    config: ParseConfig,
) -> Option<CanonicalSourceRuleSnapshot> {
    is_phase_2c_rule(rule).then(|| {
        parse_source_rule_prefix(source, rule, config, |parser| match rule {
            rules::EMPTY => literals::parse_empty(parser),
            rules::ATOM => literals::parse_atom(parser),
            rules::STRING => literals::parse_string(parser),
            rules::UTF8_STRING => literals::parse_utf8_string(parser),
            rules::RAW_STRING => literals::parse_raw_string(parser),
            rules::BOOLEAN => literals::parse_boolean(parser),
            rules::TRUE_LITERAL => literals::parse_true_literal(parser),
            rules::FALSE_LITERAL => literals::parse_false_literal(parser),
            rules::NUMBER => literals::parse_number(parser),
            rules::COMPLEX_NUMBER => literals::parse_complex_number(parser),
            rules::REAL_NUMBER => literals::parse_real_number(parser),
            rules::UNTYPED_REAL_NUMBER => literals::parse_untyped_real_number(parser),
            rules::RATIONAL_LITERAL => literals::parse_rational_literal(parser),
            rules::SCIENTIFIC_LITERAL => literals::parse_scientific_literal(parser),
            rules::FLOAT_DECIMAL_START => literals::parse_float_decimal_start(parser),
            rules::FLOAT_FULL => literals::parse_float_full(parser),
            rules::FLOAT_LITERAL => literals::parse_float_literal(parser),
            rules::INTEGER_LITERAL => literals::parse_integer_literal(parser),
            rules::TYPED_INTEGER => literals::parse_typed_integer(parser),
            rules::UNTYPED_INTEGER => literals::parse_untyped_integer(parser),
            rules::DECIMAL_LITERAL => literals::parse_decimal_literal(parser),
            rules::HEXADECIMAL_LITERAL => literals::parse_hexadecimal_literal(parser),
            rules::OCTAL_LITERAL => literals::parse_octal_literal(parser),
            rules::BINARY_LITERAL => literals::parse_binary_literal(parser),
            rules::CONTEXT_ADDRESS_PATH_TOKEN => paths::parse_context_address_path_token(parser),
            rules::CONTEXT_ADDRESS_PATH => paths::parse_context_address_path(parser),
            rules::PREFIXED_CONTEXT_PATH => paths::parse_prefixed_context_path(parser),
            rules::KIND_ANY => kinds::parse_kind_any(parser),
            rules::KIND_EMPTY => kinds::parse_kind_empty(parser),
            rules::KIND_ATOM => kinds::parse_kind_atom(parser),
            _ => unreachable!("Phase 2C support guard rejects every other RuleId"),
        })
    })
}

/// Parse one exact Phase 2D operator production as a deterministic source
/// prefix. This hidden surface deliberately exposes only the closed operator
/// layer; it does not introduce a production parser root.
#[doc(hidden)]
pub fn parse_canonical_phase_2d_rule_for_test(
    source: TextSnapshot,
    rule: RuleId,
    config: ParseConfig,
) -> Option<CanonicalSourceRuleSnapshot> {
    operators::supports(rule).then(|| {
        parse_source_rule_prefix(source, rule, config, |parser| {
            operators::parse_rule(parser, rule)
                .expect("Phase 2D support guard accepts this RuleId")
        })
    })
}

fn is_phase_2c_rule(rule: RuleId) -> bool {
    matches!(
        rule,
        rules::EMPTY
            | rules::ATOM
            | rules::STRING
            | rules::UTF8_STRING
            | rules::RAW_STRING
            | rules::BOOLEAN
            | rules::TRUE_LITERAL
            | rules::FALSE_LITERAL
            | rules::NUMBER
            | rules::COMPLEX_NUMBER
            | rules::REAL_NUMBER
            | rules::UNTYPED_REAL_NUMBER
            | rules::RATIONAL_LITERAL
            | rules::SCIENTIFIC_LITERAL
            | rules::FLOAT_DECIMAL_START
            | rules::FLOAT_FULL
            | rules::FLOAT_LITERAL
            | rules::INTEGER_LITERAL
            | rules::TYPED_INTEGER
            | rules::UNTYPED_INTEGER
            | rules::DECIMAL_LITERAL
            | rules::HEXADECIMAL_LITERAL
            | rules::OCTAL_LITERAL
            | rules::BINARY_LITERAL
            | rules::CONTEXT_ADDRESS_PATH_TOKEN
            | rules::CONTEXT_ADDRESS_PATH
            | rules::PREFIXED_CONTEXT_PATH
            | rules::KIND_ANY
            | rules::KIND_EMPTY
            | rules::KIND_ATOM
    )
}

#[cfg(test)]
mod tests {
    use crate::document::{DocumentId, Revision};

    use super::*;

    #[test]
    fn phase_2c_helper_accepts_exactly_the_closed_30_rule_set() {
        let source = TextSnapshot::new(DocumentId(1), Revision(0), "").unwrap();
        let phase_rules = [
            rules::EMPTY,
            rules::ATOM,
            rules::STRING,
            rules::UTF8_STRING,
            rules::RAW_STRING,
            rules::BOOLEAN,
            rules::TRUE_LITERAL,
            rules::FALSE_LITERAL,
            rules::NUMBER,
            rules::COMPLEX_NUMBER,
            rules::REAL_NUMBER,
            rules::UNTYPED_REAL_NUMBER,
            rules::RATIONAL_LITERAL,
            rules::SCIENTIFIC_LITERAL,
            rules::FLOAT_DECIMAL_START,
            rules::FLOAT_FULL,
            rules::FLOAT_LITERAL,
            rules::INTEGER_LITERAL,
            rules::TYPED_INTEGER,
            rules::UNTYPED_INTEGER,
            rules::DECIMAL_LITERAL,
            rules::HEXADECIMAL_LITERAL,
            rules::OCTAL_LITERAL,
            rules::BINARY_LITERAL,
            rules::CONTEXT_ADDRESS_PATH_TOKEN,
            rules::CONTEXT_ADDRESS_PATH,
            rules::PREFIXED_CONTEXT_PATH,
            rules::KIND_ANY,
            rules::KIND_EMPTY,
            rules::KIND_ATOM,
        ];
        assert_eq!(phase_rules.len(), 30);
        for rule in phase_rules {
            assert!(
                parse_canonical_phase_2c_rule_for_test(
                    source.clone(),
                    rule,
                    ParseConfig::default(),
                )
                .is_some(),
                "{rule:?}"
            );
        }
        assert!(
            parse_canonical_phase_2c_rule_for_test(
                source,
                rules::LITERAL,
                ParseConfig::default(),
            )
            .is_none()
        );
    }

    #[test]
    fn phase_2d_helper_accepts_exactly_the_closed_53_rule_set() {
        let source = TextSnapshot::new(DocumentId(1), Revision(0), "").unwrap();
        assert_eq!(operators::PHASE_2D_OPERATOR_RULES.len(), 53);
        for rule in operators::PHASE_2D_OPERATOR_RULES {
            assert!(
                parse_canonical_phase_2d_rule_for_test(
                    source.clone(),
                    *rule,
                    ParseConfig::default(),
                )
                .is_some(),
                "{rule:?}"
            );
        }
        assert!(
            parse_canonical_phase_2d_rule_for_test(
                source,
                rules::EXPRESSION,
                ParseConfig::default(),
            )
            .is_none()
        );
    }
}
