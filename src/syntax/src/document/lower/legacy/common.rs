//! Shared, package-private primitives for canonical compatibility lowering.
//!
//! This module intentionally contains only mechanical syntax-to-value helpers.
//! Each closed production remains responsible for its own structural shape and
//! legacy-value decisions.

use alloc::string::String;
use alloc::vec::Vec;

use mech_core::{Token as LegacyToken, TokenKind};

use crate::document::{
    Diagnostic, DiagnosticAnchor, DiagnosticCode, DiagnosticPhase, DiagnosticStore,
    DiagnosticTags, IdGenerator, NodeFlags, Severity, SyntaxElement, SyntaxKind, SyntaxNode,
    SyntaxToken, TextRange, TokenFlags,
};

use super::source;

pub(super) const INVALID_NODE_FLAGS: NodeFlags = NodeFlags(
    NodeFlags::ERROR.0
        | NodeFlags::MISSING.0
        | NodeFlags::CONTAINS_ERROR.0
        | NodeFlags::CONTAINS_MISSING.0,
);
pub(super) const INVALID_TOKEN_FLAGS: TokenFlags = TokenFlags(
    TokenFlags::SYNTHETIC.0 | TokenFlags::MISSING.0 | TokenFlags::ERROR.0 | TokenFlags::TRIVIA.0,
);

pub(super) fn validate_node(
    syntax: &SyntaxNode,
    expected_kind: SyntaxKind,
    name: &str,
) -> Result<(), String> {
    if syntax.kind() != expected_kind || syntax.flags().intersects(INVALID_NODE_FLAGS) {
        return Err(alloc::format!(
            "expected an error-free canonical {name} node"
        ));
    }
    Ok(())
}

pub(super) fn validate_clean_node(syntax: &SyntaxNode, name: &str) -> Result<(), String> {
    if syntax.flags().intersects(INVALID_NODE_FLAGS) {
        return Err(alloc::format!(
            "expected error-free canonical {name} syntax"
        ));
    }
    Ok(())
}

pub(super) fn validate_token(token: &SyntaxToken) -> Result<(), String> {
    if token.flags().intersects(INVALID_TOKEN_FLAGS) {
        return Err(String::from("canonical syntax contains an invalid token"));
    }
    Ok(())
}

pub(super) fn direct_tokens(syntax: &SyntaxNode, name: &str) -> Result<Vec<SyntaxToken>, String> {
    let mut tokens = Vec::new();
    for element in syntax.children_with_tokens() {
        match element {
            SyntaxElement::Node(_) => {
                return Err(alloc::format!("{name} syntax cannot contain child nodes"));
            }
            SyntaxElement::Token(token) => {
                if token.flags().intersects(INVALID_TOKEN_FLAGS) {
                    return Err(alloc::format!("{name} syntax contains an invalid token"));
                }
                tokens.push(token);
            }
        }
    }
    Ok(tokens)
}

pub(super) fn source_range(
    syntax: &SyntaxNode,
    token: &SyntaxToken,
) -> Result<mech_core::SourceRange, String> {
    source_range_for_range(syntax, token.range())
}

pub(super) fn source_range_for_range(
    syntax: &SyntaxNode,
    range: TextRange,
) -> Result<mech_core::SourceRange, String> {
    source::source_range(syntax.source(), range)
        .ok_or_else(|| String::from("cannot convert canonical token source range"))
}

pub(super) fn lower_syntax_token(
    syntax: &SyntaxNode,
    token: &SyntaxToken,
    kind: TokenKind,
) -> Result<LegacyToken, String> {
    validate_token(token)?;
    let text = token
        .text()
        .map_err(|_| String::from("cannot read canonical token source"))?;
    Ok(LegacyToken {
        kind,
        chars: text.chars().collect(),
        src_range: source_range(syntax, token)?,
    })
}

pub(super) fn lower_escaped_character_node(
    syntax: &SyntaxNode,
) -> Result<LegacyToken, String> {
    validate_node(syntax, SyntaxKind::EscapedCharacter, "escaped-character")?;
    let tokens = direct_tokens(syntax, "escaped-character")?;
    if tokens.len() != 2
        || tokens[0].kind() != SyntaxKind::Backslash
        || tokens[1].kind() != SyntaxKind::EscapedChar
    {
        return Err(String::from(
            "escaped-character syntax requires one backslash and one value token",
        ));
    }
    if tokens[0]
        .text()
        .map_err(|_| String::from("cannot read escaped-character backslash"))?
        != "\\"
    {
        return Err(String::from(
            "escaped-character syntax has an invalid backslash token",
        ));
    }

    let text = tokens[1]
        .text()
        .map_err(|_| String::from("cannot read escaped-character value"))?;
    if text.is_empty() {
        return Err(String::from("escaped-character value cannot be empty"));
    }
    Ok(LegacyToken {
        kind: TokenKind::EscapedChar,
        chars: text
            .chars()
            .map(|character| match character {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                other => other,
            })
            .collect(),
        src_range: source_range(syntax, &tokens[1])?,
    })
}

pub(super) fn merge_legacy_tokens(
    tokens: &mut Vec<LegacyToken>,
    description: &str,
) -> Result<LegacyToken, String> {
    LegacyToken::merge_tokens(tokens)
        .ok_or_else(|| alloc::format!("{description} cannot be empty"))
}

pub(super) fn failure_store(
    syntax: &SyntaxNode,
    code: &str,
    message: String,
) -> DiagnosticStore {
    let mut ids = IdGenerator::new();
    let mut diagnostics = DiagnosticStore::new(syntax.source().revision());
    diagnostics.push(Diagnostic {
        id: ids.diagnostic(),
        code: DiagnosticCode::from(code),
        phase: DiagnosticPhase::Lowering,
        severity: Severity::Error,
        rule: None,
        context: None,
        primary: DiagnosticAnchor::Absolute {
            revision: syntax.source().revision(),
            range: syntax.range(),
        },
        labels: Vec::new(),
        expected: Vec::new(),
        found: None,
        fixes: Vec::new(),
        related: Vec::new(),
        recovery: None,
        tags: DiagnosticTags::NONE,
        message,
    });
    diagnostics
}
