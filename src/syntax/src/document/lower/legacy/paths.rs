use alloc::string::String;
use alloc::vec::Vec;

use mech_core::{Identifier as LegacyIdentifier, Token as LegacyToken, TokenKind};

use crate::document::ast::paths::{ContextAddressPathSyntax, PrefixedContextPathSyntax};
use crate::document::{
    AstNode, DiagnosticStore, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken,
};

use super::base::lower_legacy_identifier_path_segment;
use super::common;

/// Lower a canonical `context-address-path` node to the legacy identifier
/// value used by the private expression parser.
pub fn lower_legacy_context_address_path(
    syntax: &ContextAddressPathSyntax,
) -> Result<LegacyIdentifier, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::ContextAddressPath,
        "context-address-path",
        lower_context_address_path,
    )
}

/// Lower a canonical `prefixed-context-path` node to its legacy context and
/// address identifier pair. The `@` and separating slash are syntax only.
pub fn lower_legacy_prefixed_context_path(
    syntax: &PrefixedContextPathSyntax,
) -> Result<(LegacyIdentifier, LegacyIdentifier), DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::PrefixedContextPath,
        "prefixed-context-path",
        lower_prefixed_context_path,
    )
}

fn lower_context_address_path(syntax: &SyntaxNode) -> Result<LegacyIdentifier, String> {
    let tokens = common::direct_tokens(syntax, "context-address-path")?;
    if tokens.is_empty() {
        return Err(String::from(
            "context-address-path syntax requires at least one path token",
        ));
    }
    if tokens
        .windows(2)
        .any(|pair| pair[0].range().end != pair[1].range().start)
    {
        return Err(String::from(
            "context-address-path syntax contains noncontiguous path tokens",
        ));
    }

    let mut lowered = Vec::with_capacity(tokens.len());
    for token in &tokens {
        let kind = match token.kind() {
            SyntaxKind::Alpha => TokenKind::Alpha,
            SyntaxKind::Digit => TokenKind::Digit,
            SyntaxKind::Dash => TokenKind::Dash,
            SyntaxKind::Slash => TokenKind::Slash,
            SyntaxKind::Underscore => TokenKind::Underscore,
            SyntaxKind::Period => TokenKind::Period,
            _ => {
                return Err(String::from(
                    "context-address-path syntax contains an unsupported token",
                ));
            }
        };
        lowered.push(lower_token(syntax, token, kind)?);
    }

    let mut merged = LegacyToken::merge_tokens(&mut lowered).ok_or_else(|| {
        String::from("context-address-path syntax cannot lower an empty token sequence")
    })?;
    merged.kind = TokenKind::Identifier;
    Ok(LegacyIdentifier { name: merged })
}

fn lower_prefixed_context_path(
    syntax: &SyntaxNode,
) -> Result<(LegacyIdentifier, LegacyIdentifier), String> {
    let elements = syntax.children_with_tokens();
    if elements.len() != 4 {
        return Err(String::from(
            "prefixed-context-path syntax requires four direct elements",
        ));
    }
    require_token(&elements[0], SyntaxKind::At, "@")?;
    let context = match &elements[1] {
        SyntaxElement::Node(node) if node.kind() == SyntaxKind::IdentifierPathSegment => {
            lower_legacy_identifier_path_segment(node).map_err(|_| {
                String::from("prefixed-context-path syntax contains an invalid context segment")
            })?
        }
        _ => {
            return Err(String::from(
                "prefixed-context-path syntax requires an identifier-path-segment context",
            ));
        }
    };
    require_token(&elements[2], SyntaxKind::Slash, "/")?;
    let address = match &elements[3] {
        SyntaxElement::Node(node) if node.kind() == SyntaxKind::ContextAddressPath => {
            lower_context_address_path(node)?
        }
        _ => {
            return Err(String::from(
                "prefixed-context-path syntax requires a context-address-path address",
            ));
        }
    };
    Ok((context, address))
}

fn lower_value<T>(
    syntax: &SyntaxNode,
    expected_kind: SyntaxKind,
    name: &'static str,
    lower: impl FnOnce(&SyntaxNode) -> Result<T, String>,
) -> Result<T, DiagnosticStore> {
    let lowered = (|| {
        common::validate_node(syntax, expected_kind, name)?;
        lower(syntax)
    })();
    lowered.map_err(|message| {
        common::failure_store(syntax, "lowering/invalid-context-path-syntax", message)
    })
}

fn require_token(
    element: &SyntaxElement,
    expected_kind: SyntaxKind,
    expected_text: &str,
) -> Result<(), String> {
    let SyntaxElement::Token(token) = element else {
        return Err(String::from("expected a direct delimiter token"));
    };
    common::validate_token(token)?;
    let text = token
        .text()
        .map_err(|_| String::from("cannot read canonical delimiter source"))?;
    if token.kind() != expected_kind || text != expected_text {
        return Err(alloc::format!(
            "expected {expected_kind:?} delimiter {expected_text:?}"
        ));
    }
    Ok(())
}

fn lower_token(
    syntax: &SyntaxNode,
    token: &SyntaxToken,
    kind: TokenKind,
) -> Result<LegacyToken, String> {
    common::lower_syntax_token(syntax, token, kind)
}
