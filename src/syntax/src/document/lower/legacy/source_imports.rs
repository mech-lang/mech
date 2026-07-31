//! Compatibility lowering for the closed Phase 2F source-import layer.

use alloc::string::String;
use alloc::vec::Vec;

use mech_core::nodes::{ImportDeclaration, MechString};
use mech_core::{Token as LegacyToken, TokenKind};

use crate::document::ast::source_imports::{ImportDeclarationSyntax, SourceImportSpecifierSyntax};
use crate::document::{
    AstNode, DiagnosticStore, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken, TextRange,
    TextSize,
};

use super::common;

type LowerResult<T> = Result<T, String>;

/// A package-private direct-rule value used by the Phase 2F parity tests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum LegacySourceImportValue {
    Tail(LegacyToken),
    Component(Vec<LegacyToken>),
    MecPath(Vec<LegacyToken>),
    Relative(MechString),
    Absolute(MechString),
    Bare(MechString),
    UriScheme(Vec<LegacyToken>),
    Uri(MechString),
    Specifier(MechString),
    Declaration(ImportDeclaration),
}

/// Lower a complete canonical source-import declaration.
pub fn lower_legacy_import_declaration(
    syntax: &ImportDeclarationSyntax,
) -> Result<ImportDeclaration, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::ImportDeclaration,
        "import-declaration",
        lower_import_declaration_node,
    )
}

/// Lower a selected canonical source-import specifier.
pub fn lower_legacy_source_import_specifier(
    syntax: &SourceImportSpecifierSyntax,
) -> Result<MechString, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::SourceImportSpecifier,
        "source-import-specifier",
        lower_source_import_specifier_node,
    )
}

/// Lower any node-valued Phase 2F source-import production for internal parity
/// coverage. Token and transparent productions intentionally have no wrapper
/// node value here.
pub(crate) fn lower_phase_2f_source_import_value(
    syntax: &SyntaxNode,
) -> Result<LegacySourceImportValue, DiagnosticStore> {
    let lowered = match syntax.kind() {
        SyntaxKind::SourceImportTail => {
            lower_source_import_tail_node(syntax).map(LegacySourceImportValue::Tail)
        }
        SyntaxKind::SourcePathComponent => {
            lower_source_path_component_node(syntax).map(LegacySourceImportValue::Component)
        }
        SyntaxKind::SourceMecPath => {
            lower_source_mec_path_node(syntax).map(LegacySourceImportValue::MecPath)
        }
        SyntaxKind::RelativeSourceImportSpecifier => {
            lower_relative_source_import_specifier_node(syntax)
                .map(LegacySourceImportValue::Relative)
        }
        SyntaxKind::AbsoluteSourceImportSpecifier => {
            lower_absolute_source_import_specifier_node(syntax)
                .map(LegacySourceImportValue::Absolute)
        }
        SyntaxKind::BareSourceImportSpecifier => {
            lower_bare_source_import_specifier_node(syntax).map(LegacySourceImportValue::Bare)
        }
        SyntaxKind::SourceImportUriScheme => {
            lower_source_import_uri_scheme_node(syntax).map(LegacySourceImportValue::UriScheme)
        }
        SyntaxKind::UriSourceImportSpecifier => {
            lower_uri_source_import_specifier_node(syntax).map(LegacySourceImportValue::Uri)
        }
        SyntaxKind::SourceImportSpecifier => {
            lower_source_import_specifier_node(syntax).map(LegacySourceImportValue::Specifier)
        }
        SyntaxKind::ImportDeclaration => {
            lower_import_declaration_node(syntax).map(LegacySourceImportValue::Declaration)
        }
        _ => Err(String::from(
            "expected a node-valued Phase 2F source-import production",
        )),
    };
    lowered.map_err(|message| {
        common::failure_store(syntax, "lowering/invalid-source-import-syntax", message)
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
        common::failure_store(syntax, "lowering/invalid-source-import-syntax", message)
    })
}

fn lower_source_import_tail_node(syntax: &SyntaxNode) -> LowerResult<LegacyToken> {
    common::validate_node(syntax, SyntaxKind::SourceImportTail, "source-import-tail")?;
    let tokens = common::direct_tokens(syntax, "source-import-tail")?;
    validate_full_direct_coverage(syntax, &tokens, "source-import-tail")?;
    let physical = syntax
        .text()
        .map_err(|_| String::from("cannot read source-import-tail text"))?;
    let semantic = physical.trim_end_matches(char::is_whitespace);
    let semantic_end = syntax.range().start + TextSize::from_u32(semantic.len() as u32);
    Ok(LegacyToken {
        kind: TokenKind::Any,
        chars: semantic.chars().collect(),
        src_range: common::source_range_for_range(
            syntax,
            TextRange::new(syntax.range().start, semantic_end),
        )?,
    })
}

fn lower_source_path_component_node(syntax: &SyntaxNode) -> LowerResult<Vec<LegacyToken>> {
    common::validate_node(
        syntax,
        SyntaxKind::SourcePathComponent,
        "source-path-component",
    )?;
    let tokens = common::direct_tokens(syntax, "source-path-component")?;
    validate_full_direct_coverage(syntax, &tokens, "source-path-component")?;
    tokens
        .iter()
        .map(|token| {
            let kind = match token.kind() {
                SyntaxKind::Alpha => TokenKind::Alpha,
                SyntaxKind::Digit => TokenKind::Digit,
                SyntaxKind::Dash => TokenKind::Dash,
                SyntaxKind::Underscore => TokenKind::Underscore,
                SyntaxKind::Period => TokenKind::Period,
                _ => {
                    return Err(String::from(
                        "source-path-component has an unsupported token",
                    ));
                }
            };
            common::lower_syntax_token(syntax, token, kind)
        })
        .collect()
}

fn lower_source_mec_path_node(syntax: &SyntaxNode) -> LowerResult<Vec<LegacyToken>> {
    common::validate_node(syntax, SyntaxKind::SourceMecPath, "source-mec-path")?;
    let mut tokens = Vec::new();
    let mut expect_component = true;
    for element in syntax.children_with_tokens() {
        match (expect_component, element) {
            (true, SyntaxElement::Node(component))
                if component.kind() == SyntaxKind::SourcePathComponent =>
            {
                tokens.extend(lower_source_path_component_node(&component)?);
                expect_component = false;
            }
            (false, SyntaxElement::Token(slash)) if slash.kind() == SyntaxKind::Slash => {
                tokens.push(common::lower_syntax_token(
                    syntax,
                    &slash,
                    TokenKind::Slash,
                )?);
                expect_component = true;
            }
            _ => {
                return Err(String::from(
                    "source-mec-path has an invalid component/separator sequence",
                ));
            }
        }
    }
    if tokens.is_empty() || expect_component {
        return Err(String::from(
            "source-mec-path requires a complete nonempty component sequence",
        ));
    }
    let text = merge_text(&tokens);
    if !text.ends_with(".mec") {
        return Err(String::from(
            "source-mec-path must end with lowercase `.mec`",
        ));
    }
    Ok(tokens)
}

fn lower_relative_source_import_specifier_node(syntax: &SyntaxNode) -> LowerResult<MechString> {
    common::validate_node(
        syntax,
        SyntaxKind::RelativeSourceImportSpecifier,
        "relative-source-import-specifier",
    )?;
    let elements = syntax.children_with_tokens();
    match elements.as_slice() {
        [
            SyntaxElement::Token(period_a),
            SyntaxElement::Token(period_b),
            SyntaxElement::Token(slash),
            SyntaxElement::Node(path),
            suffix @ ..,
        ] if period_a.kind() == SyntaxKind::Period
            && period_b.kind() == SyntaxKind::Period
            && slash.kind() == SyntaxKind::Slash
            && path.kind() == SyntaxKind::SourceMecPath =>
        {
            lower_file_source_import_parts(
                syntax,
                "relative-source-import-specifier",
                &elements[..3],
                path,
                suffix,
            )
        }
        [
            SyntaxElement::Token(period),
            SyntaxElement::Token(slash),
            SyntaxElement::Node(path),
            suffix @ ..,
        ] if period.kind() == SyntaxKind::Period
            && slash.kind() == SyntaxKind::Slash
            && path.kind() == SyntaxKind::SourceMecPath =>
        {
            lower_file_source_import_parts(
                syntax,
                "relative-source-import-specifier",
                &elements[..2],
                path,
                suffix,
            )
        }
        _ => Err(String::from(
            "relative-source-import-specifier has an invalid structure",
        )),
    }
}

fn lower_absolute_source_import_specifier_node(syntax: &SyntaxNode) -> LowerResult<MechString> {
    common::validate_node(
        syntax,
        SyntaxKind::AbsoluteSourceImportSpecifier,
        "absolute-source-import-specifier",
    )?;
    let elements = syntax.children_with_tokens();
    match elements.as_slice() {
        [
            prefix @ SyntaxElement::Token(slash),
            SyntaxElement::Node(path),
            suffix @ ..,
        ] if slash.kind() == SyntaxKind::Slash && path.kind() == SyntaxKind::SourceMecPath => {
            lower_file_source_import_parts(
                syntax,
                "absolute-source-import-specifier",
                core::slice::from_ref(prefix),
                path,
                suffix,
            )
        }
        _ => Err(String::from(
            "absolute-source-import-specifier has an invalid structure",
        )),
    }
}

fn lower_bare_source_import_specifier_node(syntax: &SyntaxNode) -> LowerResult<MechString> {
    common::validate_node(
        syntax,
        SyntaxKind::BareSourceImportSpecifier,
        "bare-source-import-specifier",
    )?;
    let elements = syntax.children_with_tokens();
    match elements.as_slice() {
        [SyntaxElement::Node(path), suffix @ ..] if path.kind() == SyntaxKind::SourceMecPath => {
            lower_file_source_import_parts(
                syntax,
                "bare-source-import-specifier",
                &[],
                path,
                suffix,
            )
        }
        _ => Err(String::from(
            "bare-source-import-specifier has an invalid structure",
        )),
    }
}

fn lower_file_source_import_parts(
    syntax: &SyntaxNode,
    name: &str,
    prefix: &[SyntaxElement],
    path: &SyntaxNode,
    suffix: &[SyntaxElement],
) -> LowerResult<MechString> {
    let mut tokens = Vec::new();
    for element in prefix {
        let SyntaxElement::Token(token) = element else {
            return Err(alloc::format!("{name} prefix must contain only tokens"));
        };
        tokens.push(lower_file_token(syntax, token)?);
    }
    tokens.extend(lower_source_mec_path_node(path)?);
    lower_wildcard_suffix(syntax, suffix, name, &mut tokens)?;
    merge_any_string(tokens, name)
}

fn lower_source_import_uri_scheme_node(syntax: &SyntaxNode) -> LowerResult<Vec<LegacyToken>> {
    common::validate_node(
        syntax,
        SyntaxKind::SourceImportUriScheme,
        "source-import-uri-scheme",
    )?;
    let tokens = common::direct_tokens(syntax, "source-import-uri-scheme")?;
    validate_full_direct_coverage(syntax, &tokens, "source-import-uri-scheme")?;
    let Some(first) = tokens.first() else {
        return Err(String::from(
            "source-import-uri-scheme requires an alpha token",
        ));
    };
    if first.kind() != SyntaxKind::Alpha {
        return Err(String::from(
            "source-import-uri-scheme must begin with an alpha token",
        ));
    }
    tokens
        .iter()
        .map(|token| {
            let kind = match token.kind() {
                SyntaxKind::Alpha => TokenKind::Alpha,
                SyntaxKind::Digit => TokenKind::Digit,
                SyntaxKind::Plus => TokenKind::Plus,
                SyntaxKind::Dash => TokenKind::Dash,
                SyntaxKind::Period => TokenKind::Period,
                _ => {
                    return Err(String::from(
                        "source-import-uri-scheme has an unsupported token",
                    ));
                }
            };
            common::lower_syntax_token(syntax, token, kind)
        })
        .collect()
}

fn lower_uri_source_import_specifier_node(syntax: &SyntaxNode) -> LowerResult<MechString> {
    common::validate_node(
        syntax,
        SyntaxKind::UriSourceImportSpecifier,
        "uri-source-import-specifier",
    )?;
    let elements = syntax.children_with_tokens();
    let [
        SyntaxElement::Node(scheme),
        SyntaxElement::Token(marker),
        SyntaxElement::Node(tail),
    ] = elements.as_slice()
    else {
        return Err(String::from(
            "uri-source-import-specifier requires scheme, `://`, and tail",
        ));
    };
    if scheme.kind() != SyntaxKind::SourceImportUriScheme
        || tail.kind() != SyntaxKind::SourceImportTail
    {
        return Err(String::from(
            "uri-source-import-specifier contains unsupported structural children",
        ));
    }
    require_token(marker, SyntaxKind::Text, "://")?;
    let mut tokens = lower_source_import_uri_scheme_node(scheme)?;
    tokens.push(common::lower_syntax_token(syntax, marker, TokenKind::Any)?);
    tokens.push(lower_source_import_tail_node(tail)?);
    merge_any_string(tokens, "uri-source-import-specifier")
}

fn lower_source_import_specifier_node(syntax: &SyntaxNode) -> LowerResult<MechString> {
    common::validate_node(
        syntax,
        SyntaxKind::SourceImportSpecifier,
        "source-import-specifier",
    )?;
    let elements = syntax.children_with_tokens();
    let [SyntaxElement::Node(selected)] = elements.as_slice() else {
        return Err(String::from(
            "source-import-specifier requires exactly one selected child",
        ));
    };
    match selected.kind() {
        SyntaxKind::RelativeSourceImportSpecifier => {
            lower_relative_source_import_specifier_node(selected)
        }
        SyntaxKind::AbsoluteSourceImportSpecifier => {
            lower_absolute_source_import_specifier_node(selected)
        }
        SyntaxKind::BareSourceImportSpecifier => lower_bare_source_import_specifier_node(selected),
        SyntaxKind::UriSourceImportSpecifier => lower_uri_source_import_specifier_node(selected),
        _ => Err(String::from(
            "source-import-specifier selected an unsupported child",
        )),
    }
}

fn lower_import_declaration_node(syntax: &SyntaxNode) -> LowerResult<ImportDeclaration> {
    common::validate_node(syntax, SyntaxKind::ImportDeclaration, "import-declaration")?;
    let mut sigil_count = 0_usize;
    let mut specifier = None;
    for element in syntax.children_with_tokens() {
        match element {
            SyntaxElement::Token(token) if token.kind() == SyntaxKind::ModuleImportSigil => {
                require_token(&token, SyntaxKind::ModuleImportSigil, "+>")?;
                sigil_count = sigil_count.saturating_add(1);
            }
            SyntaxElement::Token(token) if is_whitespace_token(&token) => {
                common::validate_token(&token)?;
            }
            SyntaxElement::Node(node)
                if node.kind() == SyntaxKind::SourceImportSpecifier && specifier.is_none() =>
            {
                specifier = Some(node);
            }
            _ => return Err(String::from("import-declaration has an invalid structure")),
        }
    }
    if sigil_count != 1 {
        return Err(String::from(
            "import-declaration requires exactly one exact import sigil",
        ));
    }
    let specifier =
        specifier.ok_or_else(|| String::from("import-declaration requires a specifier"))?;
    let specifier = lower_source_import_specifier_node(&specifier)?;
    let semantic = specifier.text.to_string();
    if !source_wildcard_is_valid(&semantic) {
        return Err(String::from(
            "source-import wildcard must be the sole final `/*` suffix",
        ));
    }
    Ok(ImportDeclaration { specifier })
}

fn is_whitespace_token(token: &SyntaxToken) -> bool {
    matches!(
        token.kind(),
        SyntaxKind::Whitespace | SyntaxKind::Tab | SyntaxKind::Newline | SyntaxKind::CarriageReturn
    )
}

fn lower_file_token(syntax: &SyntaxNode, token: &SyntaxToken) -> LowerResult<LegacyToken> {
    let kind = match token.kind() {
        SyntaxKind::Period => TokenKind::Period,
        SyntaxKind::Slash => TokenKind::Slash,
        _ => {
            return Err(String::from(
                "source-import prefix has an unsupported token",
            ));
        }
    };
    common::lower_syntax_token(syntax, token, kind)
}

fn lower_wildcard_suffix(
    syntax: &SyntaxNode,
    suffix: &[SyntaxElement],
    name: &str,
    tokens: &mut Vec<LegacyToken>,
) -> LowerResult<()> {
    match suffix {
        [] => Ok(()),
        [SyntaxElement::Token(slash), SyntaxElement::Token(asterisk)]
            if slash.kind() == SyntaxKind::Slash && asterisk.kind() == SyntaxKind::Asterisk =>
        {
            tokens.push(common::lower_syntax_token(syntax, slash, TokenKind::Slash)?);
            tokens.push(common::lower_syntax_token(
                syntax,
                asterisk,
                TokenKind::Asterisk,
            )?);
            Ok(())
        }
        _ => Err(alloc::format!("{name} has an invalid wildcard suffix")),
    }
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

fn merge_any_string(mut tokens: Vec<LegacyToken>, name: &str) -> LowerResult<MechString> {
    let mut text = common::merge_legacy_tokens(&mut tokens, name)?;
    text.kind = TokenKind::Any;
    Ok(MechString { text })
}

fn merge_text(tokens: &[LegacyToken]) -> String {
    tokens.iter().flat_map(|token| token.chars.iter()).collect()
}

fn source_wildcard_is_valid(specifier: &str) -> bool {
    let wildcard_count = specifier.matches('*').count();
    wildcard_count == 0 || (wildcard_count == 1 && specifier.ends_with("/*"))
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
