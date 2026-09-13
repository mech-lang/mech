//! Canonical module-import productions for the Phase 2E closed island.
//!
//! This module deliberately stops at module imports. Source-import
//! declarations and the complete code-level alternative remain outside this
//! phase, so a direct `module-import` parse retains only its own prefix
//! behavior.

use alloc::string::String;

use crate::document::{ExpectedSyntax, RuleId, SyntaxKind};

use super::super::Parser;
use super::super::recovery::{self, RecoveryClass};
use super::super::rule::rules;
use super::base;
use super::combinator::{self, Attempt};

/// The complete closed module-import set directly ported by Phase 2E.
pub(crate) const PHASE_2E_IMPORT_RULES: &[RuleId; 19] = &[
    rules::MODULE_IMPORT_NAME_SEGMENT,
    rules::MODULE_IMPORT_INTRINSIC_SEGMENT,
    rules::MODULE_IMPORT_PATH_SEGMENT,
    rules::MODULE_IMPORT_PATH,
    rules::MODULE_IMPORT_ALIAS_SEGMENT,
    rules::MODULE_IMPORT_ALIAS_PATH,
    rules::MODULE_IMPORT_VALUE_ALIAS,
    rules::CONTEXT_IMPORT_ALIAS_SEGMENT,
    rules::MODULE_IMPORT_CONTEXT_ALIAS,
    rules::MODULE_IMPORT_ALIAS,
    rules::MODULE_ROOT,
    rules::IMPORT_ALIAS_OPERATOR,
    rules::IMPORT_GROUP_SEPARATOR,
    rules::IMPORT_GROUP_ITEM,
    rules::IMPORT_GROUP_ITEMS,
    rules::ALIASED_ITEM_IMPORT,
    rules::MODULE_SUFFIX_IMPORT,
    rules::MODULE_ONLY_IMPORT,
    rules::MODULE_IMPORT,
];

/// Whether `rule` belongs to the Phase 2E closed module-import layer.
pub(crate) fn supports(rule: RuleId) -> bool {
    PHASE_2E_IMPORT_RULES.contains(&rule)
}

/// Dispatch one exact Phase 2E module-import production.
pub(crate) fn parse_rule(parser: &mut Parser<'_>, rule: RuleId) -> Option<Attempt> {
    supports(rule).then(|| match rule {
        rules::MODULE_IMPORT_NAME_SEGMENT => parse_module_import_name_segment(parser),
        rules::MODULE_IMPORT_INTRINSIC_SEGMENT => parse_module_import_intrinsic_segment(parser),
        rules::MODULE_IMPORT_PATH_SEGMENT => parse_module_import_path_segment(parser),
        rules::MODULE_IMPORT_PATH => parse_module_import_path(parser),
        rules::MODULE_IMPORT_ALIAS_SEGMENT => parse_module_import_alias_segment(parser),
        rules::MODULE_IMPORT_ALIAS_PATH => parse_module_import_alias_path(parser),
        rules::MODULE_IMPORT_VALUE_ALIAS => parse_module_import_value_alias(parser),
        rules::CONTEXT_IMPORT_ALIAS_SEGMENT => parse_context_import_alias_segment(parser),
        rules::MODULE_IMPORT_CONTEXT_ALIAS => parse_module_import_context_alias(parser),
        rules::MODULE_IMPORT_ALIAS => parse_module_import_alias(parser),
        rules::MODULE_ROOT => parse_module_root(parser),
        rules::IMPORT_ALIAS_OPERATOR => parse_import_alias_operator(parser),
        rules::IMPORT_GROUP_SEPARATOR => parse_import_group_separator(parser),
        rules::IMPORT_GROUP_ITEM => parse_import_group_item(parser),
        rules::IMPORT_GROUP_ITEMS => parse_import_group_items(parser),
        rules::ALIASED_ITEM_IMPORT => parse_aliased_item_import(parser),
        rules::MODULE_SUFFIX_IMPORT => parse_module_suffix_import(parser),
        rules::MODULE_ONLY_IMPORT => parse_module_only_import(parser),
        rules::MODULE_IMPORT => parse_module_import(parser),
        _ => unreachable!("Phase 2E support guard rejects every other RuleId"),
    })
}

mod continuation;
pub(crate) use continuation::{Continuation, Progress};
fn drive(parser: &mut Parser<'_>, rule: RuleId) -> Attempt {
    let mut continuation = Continuation::new(rule);
    loop {
        let mut allowance = u64::MAX;
        match continuation.advance(parser, true, &mut allowance) {
            Progress::Complete(result) => return result,
            Progress::NeedsProcessing => {}
            _ => unreachable!("final module-import input"),
        }
    }
}
pub(crate) fn parse_module_import_name_segment(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::MODULE_IMPORT_NAME_SEGMENT)
}
pub(crate) fn parse_module_import_intrinsic_segment(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::MODULE_IMPORT_INTRINSIC_SEGMENT)
}
pub(crate) fn parse_module_import_path_segment(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::MODULE_IMPORT_PATH_SEGMENT)
}
pub(crate) fn parse_module_import_path(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::MODULE_IMPORT_PATH)
}
pub(crate) fn parse_module_import_alias_segment(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::MODULE_IMPORT_ALIAS_SEGMENT)
}
pub(crate) fn parse_module_import_alias_path(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::MODULE_IMPORT_ALIAS_PATH)
}
pub(crate) fn parse_module_import_value_alias(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::MODULE_IMPORT_VALUE_ALIAS)
}
pub(crate) fn parse_context_import_alias_segment(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::CONTEXT_IMPORT_ALIAS_SEGMENT)
}
pub(crate) fn parse_module_import_context_alias(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::MODULE_IMPORT_CONTEXT_ALIAS)
}
pub(crate) fn parse_module_import_alias(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::MODULE_IMPORT_ALIAS)
}
pub(crate) fn parse_module_root(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::MODULE_ROOT)
}
pub(crate) fn parse_import_alias_operator(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::IMPORT_ALIAS_OPERATOR)
}
pub(crate) fn parse_import_group_separator(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::IMPORT_GROUP_SEPARATOR)
}
pub(crate) fn parse_import_group_item(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::IMPORT_GROUP_ITEM)
}
pub(crate) fn parse_import_group_items(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::IMPORT_GROUP_ITEMS)
}
pub(crate) fn parse_aliased_item_import(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::ALIASED_ITEM_IMPORT)
}
pub(crate) fn parse_module_suffix_import(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::MODULE_SUFFIX_IMPORT)
}
pub(crate) fn parse_module_only_import(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::MODULE_ONLY_IMPORT)
}
pub(crate) fn parse_module_import(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::MODULE_IMPORT)
}
