use alloc::string::String;
use alloc::vec::Vec;

use mech_core::{Identifier as LegacyIdentifier, Token as LegacyToken, TokenKind};

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

    lowered.map_err(|message| {
        failure_store(syntax, "lowering/invalid-escaped-character-syntax", message)
    })
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

/// Lowers a lossless canonical `digit-sequence` node to the digit tokens
/// returned by the legacy parser.
///
/// Canonical syntax retains underscore separators. The legacy value omits
/// those separators while preserving each digit grapheme and its source range.
pub fn lower_legacy_digit_sequence(
    syntax: &SyntaxNode,
) -> Result<Vec<LegacyToken>, DiagnosticStore> {
    let lowered = (|| {
        validate_node(syntax, SyntaxKind::DigitSequence, "digit-sequence")?;
        let tokens = direct_tokens(syntax, "digit-sequence")?;
        if tokens.is_empty() {
            return Err(String::from(
                "digit-sequence syntax requires at least one digit token",
            ));
        }

        let mut digits = Vec::new();
        let mut previous_was_underscore = false;
        for (index, token) in tokens.iter().enumerate() {
            match token.kind() {
                SyntaxKind::Digit => {
                    let text = token
                        .text()
                        .map_err(|_| String::from("cannot read digit-sequence digit"))?;
                    if !text.chars().next().is_some_and(char::is_numeric) {
                        return Err(String::from(
                            "digit-sequence contains an invalid digit token",
                        ));
                    }
                    digits.push(lower_token(syntax, token, TokenKind::Digit)?);
                    previous_was_underscore = false;
                }
                SyntaxKind::Underscore
                    if index > 0 && index + 1 < tokens.len() && !previous_was_underscore =>
                {
                    let text = token
                        .text()
                        .map_err(|_| String::from("cannot read digit-sequence separator"))?;
                    if text != "_" {
                        return Err(String::from(
                            "digit-sequence contains an invalid separator token",
                        ));
                    }
                    previous_was_underscore = true;
                }
                _ => {
                    return Err(String::from(
                        "digit-sequence syntax contains an unexpected token",
                    ));
                }
            }
        }
        if previous_was_underscore {
            return Err(String::from(
                "digit-sequence syntax cannot end with an underscore",
            ));
        }
        Ok(digits)
    })();

    lowered
        .map_err(|message| failure_store(syntax, "lowering/invalid-digit-sequence-syntax", message))
}

/// Lowers a lossless canonical `identifier` node to the merged legacy
/// `Identifier` value.
pub fn lower_legacy_identifier(syntax: &SyntaxNode) -> Result<LegacyIdentifier, DiagnosticStore> {
    lower_identifier(
        syntax,
        SyntaxKind::Identifier,
        "identifier",
        is_identifier_first,
        is_identifier_rest,
    )
    .map_err(|message| failure_store(syntax, "lowering/invalid-identifier-syntax", message))
}

/// Lowers a lossless canonical `identifier-path-segment` node to the merged
/// legacy `Identifier` value.
pub fn lower_legacy_identifier_path_segment(
    syntax: &SyntaxNode,
) -> Result<LegacyIdentifier, DiagnosticStore> {
    lower_identifier(
        syntax,
        SyntaxKind::IdentifierPathSegment,
        "identifier-path-segment",
        is_path_segment_first,
        is_path_segment_rest,
    )
    .map_err(|message| {
        failure_store(
            syntax,
            "lowering/invalid-identifier-path-segment-syntax",
            message,
        )
    })
}

fn lower_identifier(
    syntax: &SyntaxNode,
    expected_kind: SyntaxKind,
    name: &str,
    valid_first: impl Fn(SyntaxKind) -> bool,
    valid_rest: impl Fn(SyntaxKind) -> bool,
) -> Result<LegacyIdentifier, String> {
    validate_node(syntax, expected_kind, name)?;
    let tokens = direct_tokens(syntax, name)?;
    let Some(first) = tokens.first() else {
        return Err(String::from(
            "identifier syntax requires at least one value token",
        ));
    };
    if !valid_first(first.kind()) || tokens.iter().skip(1).any(|token| !valid_rest(token.kind())) {
        return Err(String::from(
            "identifier syntax contains an unexpected value token",
        ));
    }
    if tokens
        .windows(2)
        .any(|pair| pair[0].range().end != pair[1].range().start)
    {
        return Err(String::from(
            "identifier syntax contains noncontiguous value tokens",
        ));
    }

    let mut chars = Vec::new();
    for token in &tokens {
        let text = token
            .text()
            .map_err(|_| String::from("cannot read identifier value token"))?;
        if text.is_empty() {
            return Err(String::from(
                "identifier syntax contains an empty value token",
            ));
        }
        chars.extend(text.chars());
    }

    let last = tokens
        .last()
        .expect("a first token always has a last token");
    let physical = crate::document::TextRange::new(first.range().start, last.range().end);
    let src_range = source::source_range(syntax.source(), physical)
        .ok_or_else(|| String::from("cannot convert identifier source range"))?;
    Ok(LegacyIdentifier {
        name: LegacyToken {
            kind: TokenKind::Identifier,
            chars,
            src_range,
        },
    })
}

fn is_identifier_first(kind: SyntaxKind) -> bool {
    matches!(kind, SyntaxKind::Alpha | SyntaxKind::Emoji)
}

fn is_identifier_rest(kind: SyntaxKind) -> bool {
    is_identifier_first(kind)
        || matches!(
            kind,
            SyntaxKind::Digit
                | SyntaxKind::Ampersand
                | SyntaxKind::Dollar
                | SyntaxKind::Percent
                | SyntaxKind::Slash
                | SyntaxKind::HashTag
                | SyntaxKind::Backslash
                | SyntaxKind::Tilde
                | SyntaxKind::Plus
                | SyntaxKind::Dash
                | SyntaxKind::Asterisk
                | SyntaxKind::Caret
        )
}

fn is_path_segment_first(kind: SyntaxKind) -> bool {
    matches!(kind, SyntaxKind::Alpha | SyntaxKind::Emoji)
}

fn is_path_segment_rest(kind: SyntaxKind) -> bool {
    is_path_segment_first(kind) || matches!(kind, SyntaxKind::Digit | SyntaxKind::Dash)
}

fn validate_node(syntax: &SyntaxNode, expected_kind: SyntaxKind, name: &str) -> Result<(), String> {
    if syntax.kind() != expected_kind || syntax.flags().intersects(INVALID_NODE_FLAGS) {
        return Err(alloc::format!(
            "expected an error-free canonical {name} node"
        ));
    }
    Ok(())
}

fn direct_tokens(syntax: &SyntaxNode, name: &str) -> Result<Vec<SyntaxToken>, String> {
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

fn lower_token(
    syntax: &SyntaxNode,
    token: &SyntaxToken,
    kind: TokenKind,
) -> Result<LegacyToken, String> {
    let text = token
        .text()
        .map_err(|_| String::from("cannot read canonical token source"))?;
    let src_range = source::source_range(syntax.source(), token.range())
        .ok_or_else(|| String::from("cannot convert canonical token source range"))?;
    Ok(LegacyToken {
        kind,
        chars: text.chars().collect(),
        src_range,
    })
}

fn failure_store(syntax: &SyntaxNode, code: &'static str, message: String) -> DiagnosticStore {
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
