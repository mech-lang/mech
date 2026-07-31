//! Compatibility lowering for the closed Phase 2G subscript primitives.

use alloc::string::String;
use alloc::vec::Vec;

use mech_core::{Identifier, RealNumber, Subscript};

use crate::document::ast::subscript_primitives::{
    DotSubscriptIntSyntax, DotSubscriptSyntax, SelectAllSubscriptSyntax, SubscriptPrimitiveSyntax,
    SwizzleSubscriptSyntax,
};
use crate::document::{AstNode, DiagnosticStore, SyntaxElement, SyntaxKind, SyntaxNode};

use super::base::lower_legacy_identifier;
use super::common;
use super::literals::lower_legacy_integer_literal;

type LowerResult<T> = Result<T, String>;

/// A direct legacy value emitted by a node-valued Phase 2G subscript leaf.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum LegacySubscriptPrimitiveValue {
    SelectAll(Subscript),
    Swizzle(Subscript),
    Dot(Subscript),
    DotInt(Subscript),
}

/// Lower a canonical select-all subscript.
pub fn lower_legacy_select_all_subscript(
    syntax: &SelectAllSubscriptSyntax,
) -> Result<Subscript, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::SelectAllSubscript,
        "select-all",
        lower_select_all,
    )
}

/// Lower a canonical swizzle subscript.
pub fn lower_legacy_swizzle_subscript(
    syntax: &SwizzleSubscriptSyntax,
) -> Result<Subscript, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::SwizzleSubscript,
        "swizzle-subscript",
        lower_swizzle,
    )
}

/// Lower a canonical identifier dot subscript.
pub fn lower_legacy_dot_subscript(
    syntax: &DotSubscriptSyntax,
) -> Result<Subscript, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::DotSubscript,
        "dot-subscript",
        lower_dot,
    )
}

/// Lower a canonical integer dot subscript.
pub fn lower_legacy_dot_subscript_int(
    syntax: &DotSubscriptIntSyntax,
) -> Result<Subscript, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::DotSubscriptInt,
        "dot-subscript-int",
        lower_dot_int,
    )
}

/// Lower any node-valued Phase 2G subscript primitive for direct parity
/// coverage without exposing a parent `subscript` lowering entry point.
pub(crate) fn lower_phase_2g_subscript_value(
    syntax: &SubscriptPrimitiveSyntax,
) -> Result<LegacySubscriptPrimitiveValue, DiagnosticStore> {
    let lowered = match syntax.syntax().kind() {
        SyntaxKind::SelectAllSubscript => {
            lower_select_all(syntax.syntax()).map(LegacySubscriptPrimitiveValue::SelectAll)
        }
        SyntaxKind::SwizzleSubscript => {
            lower_swizzle(syntax.syntax()).map(LegacySubscriptPrimitiveValue::Swizzle)
        }
        SyntaxKind::DotSubscript => {
            lower_dot(syntax.syntax()).map(LegacySubscriptPrimitiveValue::Dot)
        }
        SyntaxKind::DotSubscriptInt => {
            lower_dot_int(syntax.syntax()).map(LegacySubscriptPrimitiveValue::DotInt)
        }
        _ => Err(String::from(
            "expected a node-valued Phase 2G subscript primitive",
        )),
    };
    lowered.map_err(|message| {
        common::failure_store(
            syntax.syntax(),
            "lowering/invalid-subscript-primitive-syntax",
            message,
        )
    })
}

fn lower_value<T>(
    syntax: &SyntaxNode,
    expected_kind: SyntaxKind,
    name: &'static str,
    lower: impl FnOnce(&SyntaxNode) -> LowerResult<T>,
) -> Result<T, DiagnosticStore> {
    let lowered = (|| {
        common::validate_node(syntax, expected_kind, name)?;
        lower(syntax)
    })();
    lowered.map_err(|message| {
        common::failure_store(
            syntax,
            "lowering/invalid-subscript-primitive-syntax",
            message,
        )
    })
}

fn lower_select_all(syntax: &SyntaxNode) -> LowerResult<Subscript> {
    let tokens = common::direct_tokens(syntax, "select-all")?;
    if tokens.len() != 1 || tokens[0].kind() != SyntaxKind::Colon || token_text(&tokens[0])? != ":"
    {
        return Err(String::from("select-all syntax requires one `:` token"));
    }
    Ok(Subscript::All)
}

fn lower_swizzle(syntax: &SyntaxNode) -> LowerResult<Subscript> {
    common::validate_node(syntax, SyntaxKind::SwizzleSubscript, "swizzle-subscript")?;
    let elements = syntax.children_with_tokens();
    let Some(SyntaxElement::Token(period)) = elements.first() else {
        return Err(String::from(
            "swizzle-subscript syntax requires a leading period",
        ));
    };
    if period.kind() != SyntaxKind::Period || token_text(period)? != "." {
        return Err(String::from(
            "swizzle-subscript syntax has an invalid leading period",
        ));
    }

    let mut identifiers = Vec::new();
    let mut expect_identifier = true;
    for element in &elements[1..] {
        if expect_identifier {
            let SyntaxElement::Node(identifier) = element else {
                return Err(String::from(
                    "swizzle-subscript syntax requires an identifier",
                ));
            };
            if identifier.kind() != SyntaxKind::Identifier {
                return Err(String::from(
                    "swizzle-subscript syntax contains an invalid identifier",
                ));
            }
            identifiers.push(lower_legacy_identifier(identifier).map_err(|_| {
                String::from("swizzle-subscript syntax contains an invalid identifier")
            })?);
        } else {
            let SyntaxElement::Token(comma) = element else {
                return Err(String::from(
                    "swizzle-subscript syntax requires comma separators",
                ));
            };
            if comma.kind() != SyntaxKind::Comma || token_text(comma)? != "," {
                return Err(String::from(
                    "swizzle-subscript syntax contains an invalid comma",
                ));
            }
        }
        expect_identifier = !expect_identifier;
    }
    if expect_identifier || identifiers.len() < 2 {
        return Err(String::from(
            "swizzle-subscript syntax requires two or more identifiers",
        ));
    }
    Ok(Subscript::Swizzle(identifiers))
}

fn lower_dot(syntax: &SyntaxNode) -> LowerResult<Subscript> {
    common::validate_node(syntax, SyntaxKind::DotSubscript, "dot-subscript")?;
    let elements = syntax.children_with_tokens();
    let [
        SyntaxElement::Token(period),
        SyntaxElement::Node(identifier),
    ] = elements.as_slice()
    else {
        return Err(String::from(
            "dot-subscript syntax requires a period and identifier",
        ));
    };
    if period.kind() != SyntaxKind::Period || token_text(period)? != "." {
        return Err(String::from("dot-subscript syntax has an invalid period"));
    }
    if identifier.kind() != SyntaxKind::Identifier {
        return Err(String::from(
            "dot-subscript syntax has an invalid identifier",
        ));
    }
    let identifier = lower_legacy_identifier(identifier)
        .map_err(|_| String::from("dot-subscript syntax contains an invalid identifier"))?;
    Ok(Subscript::Dot(identifier))
}

fn lower_dot_int(syntax: &SyntaxNode) -> LowerResult<Subscript> {
    common::validate_node(syntax, SyntaxKind::DotSubscriptInt, "dot-subscript-int")?;
    let elements = syntax.children_with_tokens();
    let [SyntaxElement::Token(period), SyntaxElement::Node(integer)] = elements.as_slice() else {
        return Err(String::from(
            "dot-subscript-int syntax requires a period and integer literal",
        ));
    };
    if period.kind() != SyntaxKind::Period || token_text(period)? != "." {
        return Err(String::from(
            "dot-subscript-int syntax has an invalid period",
        ));
    }
    if integer.kind() != SyntaxKind::IntegerLiteral {
        return Err(String::from(
            "dot-subscript-int syntax has an invalid integer literal",
        ));
    }
    let integer = lower_legacy_integer_literal(
        &crate::document::ast::literals::IntegerLiteralSyntax::cast(integer.clone())
            .expect("checked integer-literal kind"),
    )
    .map_err(|_| String::from("dot-subscript-int syntax contains an invalid integer literal"))?;
    Ok(Subscript::DotInt(integer))
}

fn token_text(token: &crate::document::SyntaxToken) -> LowerResult<alloc::string::String> {
    token
        .text()
        .map_err(|_| String::from("cannot read canonical subscript token"))
}
