use alloc::string::String;
use alloc::vec::Vec;

use mech_core::{Token as LegacyToken, TokenKind};

use crate::document::{
    Diagnostic, DiagnosticAnchor, DiagnosticCode, DiagnosticPhase, DiagnosticStore, DiagnosticTags,
    IdGenerator, NodeFlags, Severity, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken,
    TokenFlags,
};

use super::source;

const INVALID_NODE_FLAGS: NodeFlags = NodeFlags(
    NodeFlags::ERROR.0
        | NodeFlags::MISSING.0
        | NodeFlags::CONTAINS_ERROR.0
        | NodeFlags::CONTAINS_MISSING.0,
);
const INVALID_TOKEN_FLAGS: TokenFlags = TokenFlags(
    TokenFlags::SYNTHETIC.0 | TokenFlags::MISSING.0 | TokenFlags::ERROR.0 | TokenFlags::TRIVIA.0,
);

/// Lowers a lossless canonical `escaped-char` node to its legacy token value.
///
/// The syntax node keeps the original backslash and spelling. Compatibility
/// lowering uses the value token's physical range and applies the legacy
/// `n`/`t`/`r` character mapping.
pub fn lower_legacy_escaped_character(syntax: &SyntaxNode) -> Result<LegacyToken, DiagnosticStore> {
    let lowered = (|| {
        if syntax.kind() != SyntaxKind::EscapedCharacter
            || syntax.flags().intersects(INVALID_NODE_FLAGS)
        {
            return Err(String::from(
                "expected an error-free canonical escaped-character node",
            ));
        }

        let mut tokens = Vec::new();
        for element in syntax.children_with_tokens() {
            match element {
                SyntaxElement::Node(_) => {
                    return Err(String::from(
                        "escaped-character syntax cannot contain child nodes",
                    ));
                }
                SyntaxElement::Token(token) => tokens.push(token),
            }
        }
        if tokens.len() != 2
            || tokens[0].kind() != SyntaxKind::Backslash
            || tokens[1].kind() != SyntaxKind::EscapedChar
            || tokens
                .iter()
                .any(|token| token.flags().intersects(INVALID_TOKEN_FLAGS))
        {
            return Err(String::from(
                "escaped-character syntax requires one backslash and one value token",
            ));
        }

        let backslash = tokens[0]
            .text()
            .map_err(|_| String::from("cannot read escaped-character backslash"))?;
        if backslash != "\\" {
            return Err(String::from(
                "escaped-character syntax has an invalid backslash token",
            ));
        }

        lower_escaped_value(syntax, &tokens[1])
    })();

    lowered.map_err(|message| failure_store(syntax, message))
}

fn lower_escaped_value(syntax: &SyntaxNode, value: &SyntaxToken) -> Result<LegacyToken, String> {
    let text = value
        .text()
        .map_err(|_| String::from("cannot read escaped-character value"))?;
    if text.is_empty() {
        return Err(String::from("escaped-character value cannot be empty"));
    }
    let chars = text
        .chars()
        .map(|character| match character {
            'n' => '\n',
            't' => '\t',
            'r' => '\r',
            other => other,
        })
        .collect();
    let src_range = source::source_range(syntax.source(), value.range())
        .ok_or_else(|| String::from("cannot convert escaped-character source range"))?;
    Ok(LegacyToken {
        kind: TokenKind::EscapedChar,
        chars,
        src_range,
    })
}

fn failure_store(syntax: &SyntaxNode, message: String) -> DiagnosticStore {
    let mut ids = IdGenerator::new();
    let mut diagnostics = DiagnosticStore::new(syntax.source().revision());
    diagnostics.push(Diagnostic {
        id: ids.diagnostic(),
        code: DiagnosticCode::from("lowering/invalid-escaped-character-syntax"),
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
