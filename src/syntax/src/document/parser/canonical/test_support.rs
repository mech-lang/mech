//! Shared hidden support for direct canonical source-rule contract tests.
//!
//! These helpers deliberately parse physical source fragments. They are not
//! public parser roots and must not be used by production document parsing.

use alloc::sync::Arc;

use crate::document::{
    DiagnosticAnchor, DiagnosticStore, GreenNode, IdGenerator, NodeFlags, NodeIndex, ParseStats,
    RuleId, SyntaxKind, SyntaxNode, TextRange, TextSnapshot,
};

use super::super::rule::rules;
use super::super::{LexicalMode, ParseConfig, Parser, sink};
use super::combinator::Attempt;
use super::{
    control_operators, declarations, imports, kinds, literals, operators, paths,
    pattern_primitives, recursive_core, source_imports, structure_shell, subscript_primitives,
};

/// The exact combined Phase 2F direct-rule surface.
pub(crate) const PHASE_2F_RULES: &[RuleId; 21] = &[
    rules::SOURCE_IMPORT_TAIL,
    rules::SOURCE_PATH_COMPONENT_TOKEN,
    rules::SOURCE_PATH_COMPONENT,
    rules::SOURCE_MEC_PATH,
    rules::SOURCE_MEC_PATH_WILDCARD_SUFFIX,
    rules::RELATIVE_SOURCE_IMPORT_SPECIFIER,
    rules::ABSOLUTE_SOURCE_IMPORT_SPECIFIER,
    rules::BARE_SOURCE_IMPORT_SPECIFIER,
    rules::URI_SCHEME_PART,
    rules::SOURCE_IMPORT_URI_SCHEME,
    rules::URI_SOURCE_IMPORT_SPECIFIER,
    rules::SOURCE_IMPORT_SPECIFIER,
    rules::IMPORT_DECLARATION,
    rules::EXPORT_DECLARATION,
    rules::CONTEXT_DECLARATION,
    rules::CONTEXT_BASE_CONTEXT,
    rules::CONTEXT_BASE_RESOURCE_URI,
    rules::CONTEXT_CAPABILITY_DECLARATION,
    rules::CONTEXT_CAPABILITY_PATH_TOKEN,
    rules::CONTEXT_CAPABILITY_PATH,
    rules::CONTEXT_CAPABILITY_SCOPE,
];

/// The exact combined Phase 2G direct executable-primitive surface.
pub(crate) const PHASE_2G_RULES: &[RuleId; 15] = &[
    rules::STATEMENT_SEPARATOR,
    rules::SELECT_ALL,
    rules::SWIZZLE_SUBSCRIPT,
    rules::DOT_SUBSCRIPT,
    rules::DOT_SUBSCRIPT_INT,
    rules::WILDCARD,
    rules::SPREAD_OPERATOR,
    rules::OP_ASSIGN_OPERATOR,
    rules::ADD_ASSIGN_OPERATOR,
    rules::SUB_ASSIGN_OPERATOR,
    rules::MUL_ASSIGN_OPERATOR,
    rules::DIV_ASSIGN_OPERATOR,
    rules::EXP_ASSIGN_OPERATOR,
    rules::SEND_OPERATOR,
    rules::GUARD_OPERATOR,
];

/// The exact closed Phase 2H structure-shell surface.
pub(crate) const PHASE_2H_RULES: &[RuleId; 10] = &[
    rules::MATRIX_START,
    rules::MATRIX_END,
    rules::TABLE_START,
    rules::TABLE_END,
    rules::TABLE_SEPARATOR,
    rules::TABLE_HORZ,
    rules::TABLE_TOP,
    rules::ROW_SEPARATOR,
    rules::EMPTY_MAP,
    rules::EMPTY_SET,
];

/// The exact direct-rule outcome, including local committed recovery.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalRuleOutcome {
    NoMatch,
    Matched,
    Committed,
}

impl From<Attempt> for CanonicalRuleOutcome {
    fn from(value: Attempt) -> Self {
        match value {
            Attempt::NoMatch => Self::NoMatch,
            Attempt::Matched => Self::Matched,
            Attempt::Committed => Self::Committed,
        }
    }
}

/// A narrow prefix snapshot used to exercise canonical source productions.
#[derive(Clone, Debug)]
pub struct CanonicalSourceRuleSnapshot {
    pub source: TextSnapshot,
    pub rule: RuleId,
    pub root: Arc<GreenNode>,
    pub diagnostics: DiagnosticStore,
    pub nodes: NodeIndex,
    pub stats: ParseStats,
    pub outcome: CanonicalRuleOutcome,
    pub matched: bool,
    pub consumed: TextRange,
}

impl CanonicalSourceRuleSnapshot {
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root_at(self.root.clone(), self.source.clone(), self.consumed.start)
    }

    pub fn is_strictly_clean(&self) -> bool {
        self.outcome == CanonicalRuleOutcome::Matched
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
    let outcome = parse(&mut parser);
    let matched = outcome.accepted();
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
        outcome: outcome.into(),
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
            operators::parse_rule(parser, rule).expect("Phase 2D support guard accepts this RuleId")
        })
    })
}

/// Parse one exact Phase 2E module-import production as a deterministic
/// source prefix. This hidden surface exposes only the closed module-import
/// layer and deliberately does not introduce an import parser root.
#[doc(hidden)]
pub fn parse_canonical_phase_2e_rule_for_test(
    source: TextSnapshot,
    rule: RuleId,
    config: ParseConfig,
) -> Option<CanonicalSourceRuleSnapshot> {
    imports::supports(rule).then(|| {
        parse_source_rule_prefix(source, rule, config, |parser| {
            imports::parse_rule(parser, rule).expect("Phase 2E support guard accepts this RuleId")
        })
    })
}

/// Parse one exact Phase 2F declaration or source-import production as a
/// deterministic source prefix. This hidden surface stays within the closed
/// island and does not introduce a statement or document parser root.
#[doc(hidden)]
pub fn parse_canonical_phase_2f_rule_for_test(
    source: TextSnapshot,
    rule: RuleId,
    config: ParseConfig,
) -> Option<CanonicalSourceRuleSnapshot> {
    PHASE_2F_RULES.contains(&rule).then(|| {
        parse_source_rule_prefix(source, rule, config, |parser| {
            source_imports::parse_rule(parser, rule)
                .or_else(|| declarations::parse_rule(parser, rule))
                .expect("Phase 2F support guard accepts this RuleId")
        })
    })
}

/// Parse one exact Phase 2G executable primitive as a deterministic source
/// prefix. This hidden surface deliberately does not introduce a subscript,
/// pattern, statement, expression, state-machine, or document root.
#[doc(hidden)]
pub fn parse_canonical_phase_2g_rule_for_test(
    source: TextSnapshot,
    rule: RuleId,
    config: ParseConfig,
) -> Option<CanonicalSourceRuleSnapshot> {
    PHASE_2G_RULES.contains(&rule).then(|| {
        parse_source_rule_prefix(source, rule, config, |parser| {
            subscript_primitives::parse_rule(parser, rule)
                .or_else(|| pattern_primitives::parse_rule(parser, rule))
                .or_else(|| control_operators::parse_rule(parser, rule))
                .expect("Phase 2G support guard accepts this RuleId")
        })
    })
}

/// Parse one exact Phase 2H structure-shell rule as a deterministic source
/// prefix. This hidden surface deliberately does not introduce a matrix,
/// table, map, set, structure, expression, or document root.
#[doc(hidden)]
pub fn parse_canonical_phase_2h_rule_for_test(
    source: TextSnapshot,
    rule: RuleId,
    config: ParseConfig,
) -> Option<CanonicalSourceRuleSnapshot> {
    PHASE_2H_RULES.contains(&rule).then(|| {
        parse_source_rule_prefix(source, rule, config, |parser| {
            structure_shell::parse_rule(parser, rule)
                .expect("Phase 2H support guard accepts this RuleId")
        })
    })
}

/// Parse one frozen Phase 2I recursive-core rule as a deterministic source
/// prefix without activating it in the public canonical registry.
#[doc(hidden)]
pub fn parse_canonical_phase_2i_rule_for_test(
    source: TextSnapshot,
    rule: RuleId,
    config: ParseConfig,
) -> Option<CanonicalSourceRuleSnapshot> {
    recursive_core::supports(rule).then(|| {
        parse_source_rule_prefix(source, rule, config, |parser| {
            recursive_core::parse_rule(parser, rule)
                .expect("Phase 2I support guard accepts this RuleId")
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
            parse_canonical_phase_2c_rule_for_test(source, rules::LITERAL, ParseConfig::default(),)
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

    #[test]
    fn phase_2e_helper_accepts_exactly_the_closed_19_rule_set() {
        let source = TextSnapshot::new(DocumentId(1), Revision(0), "").unwrap();
        assert_eq!(imports::PHASE_2E_IMPORT_RULES.len(), 19);
        for rule in imports::PHASE_2E_IMPORT_RULES {
            assert!(
                parse_canonical_phase_2e_rule_for_test(
                    source.clone(),
                    *rule,
                    ParseConfig::default(),
                )
                .is_some(),
                "{rule:?}"
            );
        }
        assert!(
            parse_canonical_phase_2e_rule_for_test(
                source,
                rules::IMPORT_DECLARATION,
                ParseConfig::default(),
            )
            .is_none()
        );
    }
}
