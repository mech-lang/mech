use alloc::string::String;
use alloc::vec::Vec;

use mech_core::{Identifier as LegacyIdentifier, Token as LegacyToken, TokenKind};

use crate::document::{DiagnosticStore, SyntaxKind, SyntaxNode};

use super::common;

/// Lowers a lossless canonical `escaped-char` node to its legacy token value.
///
/// The syntax node keeps the original backslash and spelling. Compatibility
/// lowering uses the value token's physical range and applies the legacy
/// `n`/`t`/`r` character mapping.
pub fn lower_legacy_escaped_character(syntax: &SyntaxNode) -> Result<LegacyToken, DiagnosticStore> {
    common::lower_escaped_character_node(syntax).map_err(|message| {
        common::failure_store(syntax, "lowering/invalid-escaped-character-syntax", message)
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
        common::validate_node(syntax, SyntaxKind::DigitSequence, "digit-sequence")?;
        let tokens = common::direct_tokens(syntax, "digit-sequence")?;
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
                    digits.push(common::lower_syntax_token(
                        syntax,
                        token,
                        TokenKind::Digit,
                    )?);
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
        .map_err(|message| {
            common::failure_store(syntax, "lowering/invalid-digit-sequence-syntax", message)
        })
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
    .map_err(|message| common::failure_store(syntax, "lowering/invalid-identifier-syntax", message))
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
        common::failure_store(
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
    common::validate_node(syntax, expected_kind, name)?;
    let tokens = common::direct_tokens(syntax, name)?;
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
    let src_range = common::source_range_for_range(syntax, physical)?;
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
