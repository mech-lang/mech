//! Compatibility lowering for the closed Phase 2F declaration layer.

use alloc::string::String;
use alloc::vec::Vec;

use mech_core::nodes::{
    ContextBase, ContextCapabilityDeclaration, ContextCapabilityScope, ContextDeclaration,
    ExportDeclaration,
};
use mech_core::{Identifier, Token as LegacyToken, TokenKind};

use crate::document::ast::declarations::{
    CanonicalContextBaseSyntax, ContextCapabilityDeclarationSyntax, ContextCapabilityScopeSyntax,
    ContextDeclarationSyntax, ExportDeclarationSyntax,
};
use crate::document::{
    AstNode, DiagnosticStore, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken,
};

use super::base::lower_legacy_identifier;
use super::common;

type LowerResult<T> = Result<T, String>;

/// A package-private direct-rule value used by the Phase 2F parity tests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum LegacyDeclarationValue {
    Export(ExportDeclaration),
    ContextDeclaration(ContextDeclaration),
    ContextBaseContext(ContextBase),
    ContextBaseResourceUri(ContextBase),
    CapabilityDeclaration(ContextCapabilityDeclaration),
    CapabilityPath(Identifier),
    CapabilityScope(ContextCapabilityScope),
}

/// Lower a complete canonical export declaration.
pub fn lower_legacy_export_declaration(
    syntax: &ExportDeclarationSyntax,
) -> Result<ExportDeclaration, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::ExportDeclaration,
        "export-declaration",
        lower_export_declaration_node,
    )
}

/// Lower a complete canonical context declaration.
pub fn lower_legacy_context_declaration(
    syntax: &ContextDeclarationSyntax,
) -> Result<ContextDeclaration, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::ContextDeclaration,
        "context-declaration",
        lower_context_declaration_node,
    )
}

/// Lower one selected canonical context base.
pub fn lower_legacy_context_base(
    syntax: &CanonicalContextBaseSyntax,
) -> Result<ContextBase, DiagnosticStore> {
    let node = syntax.syntax();
    let lowered = match syntax {
        CanonicalContextBaseSyntax::ResourceUri(_) => lower_context_base_resource_uri_node(node),
        CanonicalContextBaseSyntax::Context(_) => lower_context_base_context_node(node),
    };
    lowered.map_err(|message| {
        common::failure_store(node, "lowering/invalid-context-declaration-syntax", message)
    })
}

/// Lower one canonical context capability declaration.
pub fn lower_legacy_context_capability_declaration(
    syntax: &ContextCapabilityDeclarationSyntax,
) -> Result<ContextCapabilityDeclaration, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::ContextCapabilityDeclaration,
        "context-capability-declaration",
        lower_context_capability_declaration_node,
    )
}

/// Lower one selected canonical context capability scope.
pub fn lower_legacy_context_capability_scope(
    syntax: &ContextCapabilityScopeSyntax,
) -> Result<ContextCapabilityScope, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::ContextCapabilityScope,
        "context-capability-scope",
        lower_context_capability_scope_node,
    )
}

/// Lower any node-valued Phase 2F declaration production for internal parity
/// coverage. The direct token production intentionally has no value wrapper.
pub(crate) fn lower_phase_2f_declaration_value(
    syntax: &SyntaxNode,
) -> Result<LegacyDeclarationValue, DiagnosticStore> {
    let lowered =
        match syntax.kind() {
            SyntaxKind::ExportDeclaration => {
                lower_export_declaration_node(syntax).map(LegacyDeclarationValue::Export)
            }
            SyntaxKind::ContextDeclaration => lower_context_declaration_node(syntax)
                .map(LegacyDeclarationValue::ContextDeclaration),
            SyntaxKind::ContextBaseContext => lower_context_base_context_node(syntax)
                .map(LegacyDeclarationValue::ContextBaseContext),
            SyntaxKind::ContextBaseResourceUri => lower_context_base_resource_uri_node(syntax)
                .map(LegacyDeclarationValue::ContextBaseResourceUri),
            SyntaxKind::ContextCapabilityDeclaration => {
                lower_context_capability_declaration_node(syntax)
                    .map(LegacyDeclarationValue::CapabilityDeclaration)
            }
            SyntaxKind::ContextCapabilityPath => lower_context_capability_path_node(syntax)
                .map(LegacyDeclarationValue::CapabilityPath),
            SyntaxKind::ContextCapabilityScope => lower_context_capability_scope_node(syntax)
                .map(LegacyDeclarationValue::CapabilityScope),
            _ => Err(String::from(
                "expected a node-valued Phase 2F declaration production",
            )),
        };
    lowered.map_err(|message| {
        common::failure_store(
            syntax,
            "lowering/invalid-context-declaration-syntax",
            message,
        )
    })
}

fn lower_value<T>(
    syntax: &SyntaxNode,
    expected_kind: SyntaxKind,
    name: &str,
    lower: impl FnOnce(&SyntaxNode) -> LowerResult<T>,
) -> Result<T, DiagnosticStore> {
    let lowered = (|| {
        common::validate_node(syntax, expected_kind, name)?;
        lower(syntax)
    })();
    lowered.map_err(|message| {
        common::failure_store(
            syntax,
            "lowering/invalid-context-declaration-syntax",
            message,
        )
    })
}

fn lower_export_declaration_node(syntax: &SyntaxNode) -> LowerResult<ExportDeclaration> {
    common::validate_node(syntax, SyntaxKind::ExportDeclaration, "export-declaration")?;
    let mut sigils = 0_usize;
    let mut name = None;
    for element in syntax.children_with_tokens() {
        match element {
            SyntaxElement::Token(token) if token.kind() == SyntaxKind::ModuleExportSigil => {
                require_token(&token, SyntaxKind::ModuleExportSigil, "<+")?;
                sigils = sigils.saturating_add(1);
            }
            SyntaxElement::Token(token) if is_whitespace_token(&token) => {
                common::validate_token(&token)?;
            }
            SyntaxElement::Node(node)
                if node.kind() == SyntaxKind::Identifier && name.is_none() =>
            {
                name = Some(node);
            }
            _ => return Err(String::from("export-declaration has an invalid structure")),
        }
    }
    if sigils != 1 {
        return Err(String::from(
            "export-declaration requires exactly one export sigil",
        ));
    }
    let name = name.ok_or_else(|| String::from("export-declaration requires an identifier"))?;
    Ok(ExportDeclaration {
        name: lower_identifier(&name)?,
    })
}

fn lower_context_declaration_node(syntax: &SyntaxNode) -> LowerResult<ContextDeclaration> {
    common::validate_node(
        syntax,
        SyntaxKind::ContextDeclaration,
        "context-declaration",
    )?;
    let mut at_count = 0_usize;
    let mut name = None;
    let mut base = None;
    let mut capabilities = Vec::new();

    for element in syntax.children_with_tokens() {
        match element {
            SyntaxElement::Token(token) if token.kind() == SyntaxKind::At => {
                require_token(&token, SyntaxKind::At, "@")?;
                at_count = at_count.saturating_add(1);
            }
            SyntaxElement::Token(token) if is_context_declaration_punctuation(&token) => {
                common::validate_token(&token)?;
            }
            SyntaxElement::Node(node)
                if node.kind() == SyntaxKind::Identifier && name.is_none() =>
            {
                name = Some(node);
            }
            SyntaxElement::Node(node)
                if matches!(
                    node.kind(),
                    SyntaxKind::ContextBaseContext | SyntaxKind::ContextBaseResourceUri
                ) && base.is_none() =>
            {
                base = Some(node);
            }
            SyntaxElement::Node(node)
                if node.kind() == SyntaxKind::ContextCapabilityDeclaration =>
            {
                capabilities.push(lower_context_capability_declaration_node(&node)?);
            }
            _ => return Err(String::from("context-declaration has an invalid structure")),
        }
    }

    if at_count != 1 {
        return Err(String::from("context-declaration requires one leading `@`"));
    }
    let name = name.ok_or_else(|| String::from("context-declaration requires a name"))?;
    let base = base.ok_or_else(|| String::from("context-declaration requires a base"))?;
    let base = match base.kind() {
        SyntaxKind::ContextBaseContext => lower_context_base_context_node(&base)?,
        SyntaxKind::ContextBaseResourceUri => lower_context_base_resource_uri_node(&base)?,
        _ => return Err(String::from("context-declaration has an invalid base")),
    };
    Ok(ContextDeclaration {
        name: lower_identifier(&name)?,
        base,
        capabilities,
    })
}

fn lower_context_base_context_node(syntax: &SyntaxNode) -> LowerResult<ContextBase> {
    common::validate_node(
        syntax,
        SyntaxKind::ContextBaseContext,
        "context-base-context",
    )?;
    let elements = syntax.children_with_tokens();
    let [SyntaxElement::Token(at), SyntaxElement::Node(name)] = elements.as_slice() else {
        return Err(String::from(
            "context-base-context requires `@` and an identifier",
        ));
    };
    require_token(at, SyntaxKind::At, "@")?;
    if name.kind() != SyntaxKind::Identifier {
        return Err(String::from(
            "context-base-context requires an identifier child",
        ));
    }
    Ok(ContextBase::Context(lower_identifier(name)?))
}

fn lower_context_base_resource_uri_node(syntax: &SyntaxNode) -> LowerResult<ContextBase> {
    common::validate_node(
        syntax,
        SyntaxKind::ContextBaseResourceUri,
        "context-base-resource-uri",
    )?;
    let tokens = common::direct_tokens(syntax, "context-base-resource-uri")?;
    validate_full_direct_coverage(syntax, &tokens, "context-base-resource-uri")?;
    let text = syntax
        .text()
        .map_err(|_| String::from("cannot read context-base-resource-uri text"))?;
    Ok(ContextBase::ResourceUri(LegacyToken {
        kind: TokenKind::Any,
        chars: text.chars().collect(),
        src_range: common::source_range_for_range(syntax, syntax.range())?,
    }))
}

fn lower_context_capability_declaration_node(
    syntax: &SyntaxNode,
) -> LowerResult<ContextCapabilityDeclaration> {
    common::validate_node(
        syntax,
        SyntaxKind::ContextCapabilityDeclaration,
        "context-capability-declaration",
    )?;
    let elements = syntax.children_with_tokens();
    let [
        SyntaxElement::Token(colon),
        SyntaxElement::Node(operation),
        SyntaxElement::Token(left),
        SyntaxElement::Node(scope),
        SyntaxElement::Token(right),
    ] = elements.as_slice()
    else {
        return Err(String::from(
            "context-capability-declaration requires an operation and scope",
        ));
    };
    require_token(colon, SyntaxKind::Colon, ":")?;
    require_token(left, SyntaxKind::LeftParen, "(")?;
    require_token(right, SyntaxKind::RightParen, ")")?;
    if operation.kind() != SyntaxKind::Identifier
        || scope.kind() != SyntaxKind::ContextCapabilityScope
    {
        return Err(String::from(
            "context-capability-declaration contains unsupported children",
        ));
    }
    Ok(ContextCapabilityDeclaration {
        operation: lower_identifier(operation)?,
        scope: lower_context_capability_scope_node(scope)?,
    })
}

fn lower_context_capability_path_node(syntax: &SyntaxNode) -> LowerResult<Identifier> {
    common::validate_node(
        syntax,
        SyntaxKind::ContextCapabilityPath,
        "context-capability-path",
    )?;
    let tokens = common::direct_tokens(syntax, "context-capability-path")?;
    validate_full_direct_coverage(syntax, &tokens, "context-capability-path")?;
    let mut lowered = Vec::with_capacity(tokens.len());
    for token in &tokens {
        let kind = match token.kind() {
            SyntaxKind::Alpha => TokenKind::Alpha,
            SyntaxKind::Digit => TokenKind::Digit,
            SyntaxKind::Dash => TokenKind::Dash,
            SyntaxKind::Slash => TokenKind::Slash,
            SyntaxKind::Underscore => TokenKind::Underscore,
            SyntaxKind::Period => TokenKind::Period,
            SyntaxKind::Asterisk => TokenKind::Asterisk,
            _ => {
                return Err(String::from(
                    "context-capability-path has an unsupported token",
                ));
            }
        };
        lowered.push(common::lower_syntax_token(syntax, token, kind)?);
    }
    let text = merge_text(&lowered);
    if !context_capability_path_is_valid(&text) {
        return Err(String::from(
            "context-capability-path has invalid wildcard placement",
        ));
    }
    let mut name = common::merge_legacy_tokens(&mut lowered, "context-capability-path")?;
    name.kind = TokenKind::Identifier;
    Ok(Identifier { name })
}

fn lower_context_capability_scope_node(syntax: &SyntaxNode) -> LowerResult<ContextCapabilityScope> {
    common::validate_node(
        syntax,
        SyntaxKind::ContextCapabilityScope,
        "context-capability-scope",
    )?;
    let elements = syntax.children_with_tokens();
    match elements.as_slice() {
        [SyntaxElement::Token(wildcard)] if wildcard.kind() == SyntaxKind::Asterisk => {
            Ok(ContextCapabilityScope::Wildcard(
                common::lower_syntax_token(syntax, wildcard, TokenKind::Asterisk)?,
            ))
        }
        [SyntaxElement::Node(path)] if path.kind() == SyntaxKind::ContextCapabilityPath => Ok(
            ContextCapabilityScope::Path(lower_context_capability_path_node(path)?),
        ),
        _ => Err(String::from(
            "context-capability-scope selected an unsupported child",
        )),
    }
}

fn lower_identifier(syntax: &SyntaxNode) -> LowerResult<Identifier> {
    lower_legacy_identifier(syntax)
        .map_err(|_| String::from("invalid identifier syntax in canonical declaration"))
}

fn is_whitespace_token(token: &SyntaxToken) -> bool {
    matches!(
        token.kind(),
        SyntaxKind::Whitespace | SyntaxKind::Tab | SyntaxKind::Newline | SyntaxKind::CarriageReturn
    )
}

fn is_context_declaration_punctuation(token: &SyntaxToken) -> bool {
    is_whitespace_token(token)
        || matches!(
            token.kind(),
            SyntaxKind::Colon
                | SyntaxKind::Equal
                | SyntaxKind::DefineOperatorToken
                | SyntaxKind::LeftBrace
                | SyntaxKind::RightBrace
                | SyntaxKind::Comma
        )
}

fn validate_full_direct_coverage(
    syntax: &SyntaxNode,
    tokens: &[SyntaxToken],
    name: &str,
) -> LowerResult<()> {
    let Some(first) = tokens.first() else {
        return Err(alloc::format!("{name} requires at least one direct token"));
    };
    if first.range().start != syntax.range().start {
        return Err(alloc::format!("{name} tokens must cover the node start"));
    }
    let mut end = first.range().end;
    for token in tokens.iter().skip(1) {
        if token.range().start != end {
            return Err(alloc::format!("{name} tokens must be contiguous"));
        }
        end = token.range().end;
    }
    if end != syntax.range().end {
        return Err(alloc::format!("{name} tokens must cover the node range"));
    }
    Ok(())
}

fn context_capability_path_is_valid(path: &str) -> bool {
    let star_count = path.chars().filter(|character| *character == '*').count();
    star_count == 0 || (star_count == 1 && path.ends_with("/*") && path.len() > 2)
}

fn merge_text(tokens: &[LegacyToken]) -> String {
    tokens.iter().flat_map(|token| token.chars.iter()).collect()
}

fn require_token(
    token: &SyntaxToken,
    expected_kind: SyntaxKind,
    expected_text: &str,
) -> LowerResult<()> {
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
