use alloc::string::String;

use mech_core::nodes::Kind;

use crate::document::ast::kinds::{KindAnySyntax, KindAtomSyntax, KindEmptySyntax};
use crate::document::{
    AstNode, DiagnosticStore, SyntaxElement, SyntaxKind, SyntaxNode,
};

use super::base::lower_legacy_identifier;
use super::common;

/// Lower the primitive `kind-any` form.
pub fn lower_legacy_kind_any(syntax: &KindAnySyntax) -> Result<Kind, DiagnosticStore> {
    lower_value(syntax.syntax(), SyntaxKind::KindAny, "kind-any", |node| {
        require_exact_token_sequence(node, SyntaxKind::Asterisk, "*")?;
        Ok(Kind::Any)
    })
}

/// Lower the primitive `kind-empty` form.
pub fn lower_legacy_kind_empty(syntax: &KindEmptySyntax) -> Result<Kind, DiagnosticStore> {
    lower_value(syntax.syntax(), SyntaxKind::KindEmpty, "kind-empty", |node| {
        let tokens = common::direct_tokens(node, "kind-empty")?;
        if tokens.is_empty() {
            return Err(String::from(
                "kind-empty syntax requires at least one underscore",
            ));
        }
        for token in &tokens {
            common::validate_token(token)?;
            let text = token
                .text()
                .map_err(|_| String::from("cannot read canonical kind-empty token source"))?;
            if token.kind() != SyntaxKind::Underscore || text != "_" {
                return Err(String::from(
                    "kind-empty syntax contains a non-underscore token",
                ));
            }
        }
        Ok(Kind::Empty)
    })
}

/// Lower the primitive `kind-atom` form.
pub fn lower_legacy_kind_atom(syntax: &KindAtomSyntax) -> Result<Kind, DiagnosticStore> {
    lower_value(syntax.syntax(), SyntaxKind::KindAtom, "kind-atom", |node| {
        let elements = node.children_with_tokens();
        if elements.len() != 2 {
            return Err(String::from(
                "kind-atom syntax requires a colon and identifier",
            ));
        }
        require_token(&elements[0], SyntaxKind::Colon, ":")?;
        let SyntaxElement::Node(identifier) = &elements[1] else {
            return Err(String::from("kind-atom syntax requires an identifier node"));
        };
        if identifier.kind() != SyntaxKind::Identifier {
            return Err(String::from("kind-atom syntax requires an identifier node"));
        }
        let name = lower_legacy_identifier(identifier)
            .map_err(|_| String::from("kind-atom syntax contains an invalid identifier"))?;
        Ok(Kind::Atom(name))
    })
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
        common::failure_store(syntax, "lowering/invalid-primitive-kind-syntax", message)
    })
}

fn require_exact_token_sequence(
    syntax: &SyntaxNode,
    expected_kind: SyntaxKind,
    expected_text: &str,
) -> Result<(), String> {
    let tokens = common::direct_tokens(syntax, "primitive kind")?;
    if tokens.len() != 1 {
        return Err(String::from(
            "primitive kind syntax requires exactly one direct token",
        ));
    }
    let token = &tokens[0];
    let text = token
        .text()
        .map_err(|_| String::from("cannot read canonical primitive-kind token source"))?;
    if token.kind() != expected_kind || text != expected_text {
        return Err(alloc::format!(
            "expected {expected_kind:?} token {expected_text:?}"
        ));
    }
    Ok(())
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
