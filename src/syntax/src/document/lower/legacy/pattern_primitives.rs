//! Compatibility lowering for the closed Phase 2G pattern primitives.

use alloc::string::String;

use mech_core::Pattern;

use crate::document::ast::pattern_primitives::WildcardPatternSyntax;
use crate::document::{AstNode, DiagnosticStore, SyntaxKind, SyntaxNode};

use super::common;

/// The direct legacy value emitted by the node-valued Phase 2G pattern leaf.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum LegacyPatternPrimitiveValue {
    Wildcard(Pattern),
}

/// Lower a canonical wildcard pattern leaf.
pub fn lower_legacy_wildcard_pattern(
    syntax: &WildcardPatternSyntax,
) -> Result<Pattern, DiagnosticStore> {
    let lowered = (|| {
        common::validate_node(syntax.syntax(), SyntaxKind::WildcardPattern, "wildcard")?;
        let tokens = common::direct_tokens(syntax.syntax(), "wildcard")?;
        if tokens.len() != 1 || tokens[0].kind() != SyntaxKind::Asterisk {
            return Err(String::from("wildcard syntax requires one asterisk token"));
        }
        let text = tokens[0]
            .text()
            .map_err(|_| String::from("cannot read wildcard token"))?;
        if text != "*" {
            return Err(String::from(
                "wildcard syntax has an invalid token spelling",
            ));
        }
        Ok(Pattern::Wildcard)
    })();
    lowered.map_err(|message| {
        common::failure_store(
            syntax.syntax(),
            "lowering/invalid-pattern-primitive-syntax",
            message,
        )
    })
}

/// Lower the node-valued Phase 2G wildcard leaf for direct parity coverage.
pub(crate) fn lower_phase_2g_pattern_value(
    syntax: &WildcardPatternSyntax,
) -> Result<LegacyPatternPrimitiveValue, DiagnosticStore> {
    lower_legacy_wildcard_pattern(syntax).map(LegacyPatternPrimitiveValue::Wildcard)
}
