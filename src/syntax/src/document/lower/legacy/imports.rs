//! Compatibility lowering for the closed Phase 2E module-import layer.

use alloc::string::String;
use alloc::vec::Vec;

use mech_core::nodes::{
    ModuleImport, ModuleImportAlias, ModuleImportGroupItem, ModuleImportIntrinsicSegment,
    ModuleImportKind, ModuleImportPath, ModuleImportPathSegment,
};
use mech_core::{Identifier, Token as LegacyToken, TokenKind};

use crate::document::ast::imports::{
    ModuleImportAliasSyntax, ModuleImportPathSyntax, ModuleImportSyntax,
};
use crate::document::{
    AstNode, DiagnosticStore, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken,
};

use super::base::lower_legacy_identifier_path_segment;
use super::common;

type LowerResult<T> = Result<T, String>;

/// A package-private direct-rule value used by the Phase 2E parity tests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum LegacyModuleImportValue {
    NameSegment(ModuleImportPathSegment),
    IntrinsicSegment(ModuleImportPathSegment),
    PathSegment(ModuleImportPathSegment),
    Path(ModuleImportPath),
    AliasSegment(ModuleImportPathSegment),
    AliasPath(ModuleImportPath),
    ValueAlias(ModuleImportAlias),
    ContextAliasSegment(Identifier),
    ContextAlias(Identifier),
    Alias(ModuleImportAlias),
    Root(Identifier),
    GroupItem(ModuleImportGroupItem),
    GroupItems(Vec<ModuleImportGroupItem>),
    AliasedItem(ModuleImport),
    Suffix(ModuleImport),
    ModuleOnly(ModuleImport),
    Import(ModuleImport),
}

/// Lower a complete canonical module import to the corrected legacy value.
pub fn lower_legacy_module_import(
    syntax: &ModuleImportSyntax,
) -> Result<ModuleImport, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::ModuleImport,
        "module-import",
        lower_module_import_node,
    )
}

/// Lower a canonical module-import path.
pub fn lower_legacy_module_import_path(
    syntax: &ModuleImportPathSyntax,
) -> Result<ModuleImportPath, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::ModuleImportPath,
        "module-import-path",
        lower_module_import_path_node,
    )
}

/// Lower a canonical module-import alias.
pub fn lower_legacy_module_import_alias(
    syntax: &ModuleImportAliasSyntax,
) -> Result<ModuleImportAlias, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::ModuleImportAlias,
        "module-import-alias",
        lower_module_import_alias_node,
    )
}

/// Lower any node-valued Phase 2E direct production for internal parity
/// coverage. The two transparent productions intentionally have no value.
pub(crate) fn lower_phase_2e_module_import_value(
    syntax: &SyntaxNode,
) -> Result<LegacyModuleImportValue, DiagnosticStore> {
    let lowered = match syntax.kind() {
        SyntaxKind::ModuleImportNameSegment => {
            lower_module_import_name_segment_node(syntax).map(LegacyModuleImportValue::NameSegment)
        }
        SyntaxKind::ModuleImportIntrinsicSegment => {
            lower_module_import_intrinsic_segment_node(syntax)
                .map(LegacyModuleImportValue::IntrinsicSegment)
        }
        SyntaxKind::ModuleImportPathSegment => {
            lower_module_import_path_segment_node(syntax).map(LegacyModuleImportValue::PathSegment)
        }
        SyntaxKind::ModuleImportPath => {
            lower_module_import_path_node(syntax).map(LegacyModuleImportValue::Path)
        }
        SyntaxKind::ModuleImportAliasSegment => lower_module_import_alias_segment_node(syntax)
            .map(LegacyModuleImportValue::AliasSegment),
        SyntaxKind::ModuleImportAliasPath => {
            lower_module_import_alias_path_node(syntax).map(LegacyModuleImportValue::AliasPath)
        }
        SyntaxKind::ModuleImportValueAlias => {
            lower_module_import_value_alias_node(syntax).map(LegacyModuleImportValue::ValueAlias)
        }
        SyntaxKind::ContextImportAliasSegment => lower_context_import_alias_segment_node(syntax)
            .map(LegacyModuleImportValue::ContextAliasSegment),
        SyntaxKind::ModuleImportContextAlias => lower_module_import_context_alias_node(syntax)
            .map(LegacyModuleImportValue::ContextAlias),
        SyntaxKind::ModuleImportAlias => {
            lower_module_import_alias_node(syntax).map(LegacyModuleImportValue::Alias)
        }
        SyntaxKind::ModuleRoot => lower_module_root_node(syntax).map(LegacyModuleImportValue::Root),
        SyntaxKind::ImportGroupItem => {
            lower_import_group_item_node(syntax).map(LegacyModuleImportValue::GroupItem)
        }
        SyntaxKind::ImportGroupItems => {
            lower_import_group_items_node(syntax).map(LegacyModuleImportValue::GroupItems)
        }
        SyntaxKind::AliasedItemImport => {
            lower_aliased_item_import_node(syntax).map(LegacyModuleImportValue::AliasedItem)
        }
        SyntaxKind::ModuleSuffixImport => {
            lower_module_suffix_import_node(syntax).map(LegacyModuleImportValue::Suffix)
        }
        SyntaxKind::ModuleOnlyImport => {
            lower_module_only_import_node(syntax).map(LegacyModuleImportValue::ModuleOnly)
        }
        SyntaxKind::ModuleImport => {
            lower_module_import_node(syntax).map(LegacyModuleImportValue::Import)
        }
        _ => Err(String::from(
            "expected a node-valued Phase 2E module-import production",
        )),
    };
    lowered.map_err(|message| {
        common::failure_store(syntax, "lowering/invalid-module-import-syntax", message)
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
        common::failure_store(syntax, "lowering/invalid-module-import-syntax", message)
    })
}

fn lower_module_import_name_segment_node(
    syntax: &SyntaxNode,
) -> LowerResult<ModuleImportPathSegment> {
    common::validate_node(
        syntax,
        SyntaxKind::ModuleImportNameSegment,
        "module-import-name-segment",
    )?;
    let identifier = only_child_node(
        syntax,
        SyntaxKind::IdentifierPathSegment,
        "module-import-name-segment",
    )?;
    lower_identifier_path(&identifier).map(ModuleImportPathSegment::Name)
}

fn lower_module_import_intrinsic_segment_node(
    syntax: &SyntaxNode,
) -> LowerResult<ModuleImportPathSegment> {
    common::validate_node(
        syntax,
        SyntaxKind::ModuleImportIntrinsicSegment,
        "module-import-intrinsic-segment",
    )?;
    let elements = syntax.children_with_tokens();
    let [SyntaxElement::Token(marker), SyntaxElement::Node(name)] = elements.as_slice() else {
        return Err(String::from(
            "module-import-intrinsic-segment requires an underscore and name",
        ));
    };
    require_token(marker, SyntaxKind::Underscore, "_")?;
    if name.kind() != SyntaxKind::ModuleImportNameSegment {
        return Err(String::from(
            "module-import-intrinsic-segment requires a name child",
        ));
    }
    let name = lower_module_import_name_segment_node(name)?;
    let ModuleImportPathSegment::Name(name) = name else {
        return Err(String::from(
            "module-import-name-segment did not lower to a named segment",
        ));
    };
    Ok(ModuleImportPathSegment::Intrinsic(
        ModuleImportIntrinsicSegment {
            marker: common::lower_syntax_token(syntax, marker, TokenKind::Underscore)?,
            name,
        },
    ))
}

fn lower_module_import_path_segment_node(
    syntax: &SyntaxNode,
) -> LowerResult<ModuleImportPathSegment> {
    common::validate_node(
        syntax,
        SyntaxKind::ModuleImportPathSegment,
        "module-import-path-segment",
    )?;
    let child = only_child(syntax, "module-import-path-segment")?;
    match child.kind() {
        SyntaxKind::ModuleImportIntrinsicSegment => {
            lower_module_import_intrinsic_segment_node(&child)
        }
        SyntaxKind::ModuleImportNameSegment => lower_module_import_name_segment_node(&child),
        _ => Err(String::from(
            "module-import-path-segment selected an unsupported child",
        )),
    }
}

fn lower_module_import_path_node(syntax: &SyntaxNode) -> LowerResult<ModuleImportPath> {
    common::validate_node(syntax, SyntaxKind::ModuleImportPath, "module-import-path")?;
    let mut segments = Vec::new();
    let mut expect_segment = true;
    for element in syntax.children_with_tokens() {
        match (expect_segment, element) {
            (true, SyntaxElement::Node(node))
                if node.kind() == SyntaxKind::ModuleImportPathSegment =>
            {
                segments.push(lower_module_import_path_segment_node(&node)?);
                expect_segment = false;
            }
            (false, SyntaxElement::Token(token)) => {
                require_token(&token, SyntaxKind::Slash, "/")?;
                expect_segment = true;
            }
            _ => {
                return Err(String::from(
                    "module-import-path has an invalid segment/separator sequence",
                ));
            }
        }
    }
    if segments.is_empty() || expect_segment {
        return Err(String::from(
            "module-import-path requires a complete nonempty segment sequence",
        ));
    }
    Ok(ModuleImportPath { segments })
}

fn lower_module_import_alias_segment_node(
    syntax: &SyntaxNode,
) -> LowerResult<ModuleImportPathSegment> {
    common::validate_node(
        syntax,
        SyntaxKind::ModuleImportAliasSegment,
        "module-import-alias-segment",
    )?;
    let identifier = only_child_node(
        syntax,
        SyntaxKind::IdentifierPathSegment,
        "module-import-alias-segment",
    )?;
    lower_identifier_path(&identifier).map(ModuleImportPathSegment::Name)
}

fn lower_module_import_alias_path_node(syntax: &SyntaxNode) -> LowerResult<ModuleImportPath> {
    common::validate_node(
        syntax,
        SyntaxKind::ModuleImportAliasPath,
        "module-import-alias-path",
    )?;
    let mut segments = Vec::new();
    let mut expect_segment = true;
    for element in syntax.children_with_tokens() {
        match (expect_segment, element) {
            (true, SyntaxElement::Node(node))
                if node.kind() == SyntaxKind::ModuleImportAliasSegment =>
            {
                segments.push(lower_module_import_alias_segment_node(&node)?);
                expect_segment = false;
            }
            (false, SyntaxElement::Token(token)) => {
                require_token(&token, SyntaxKind::Slash, "/")?;
                expect_segment = true;
            }
            _ => {
                return Err(String::from(
                    "module-import-alias-path has an invalid segment/separator sequence",
                ));
            }
        }
    }
    if segments.is_empty() || expect_segment {
        return Err(String::from(
            "module-import-alias-path requires a complete nonempty segment sequence",
        ));
    }
    Ok(ModuleImportPath { segments })
}

fn lower_module_import_value_alias_node(syntax: &SyntaxNode) -> LowerResult<ModuleImportAlias> {
    common::validate_node(
        syntax,
        SyntaxKind::ModuleImportValueAlias,
        "module-import-value-alias",
    )?;
    let path = only_child_node(
        syntax,
        SyntaxKind::ModuleImportAliasPath,
        "module-import-value-alias",
    )?;
    Ok(ModuleImportAlias::Value(
        lower_module_import_alias_path_node(&path)?,
    ))
}

fn lower_context_import_alias_segment_node(syntax: &SyntaxNode) -> LowerResult<Identifier> {
    common::validate_node(
        syntax,
        SyntaxKind::ContextImportAliasSegment,
        "context-import-alias-segment",
    )?;
    let tokens = common::direct_tokens(syntax, "context-import-alias-segment")?;
    let Some(first) = tokens.first() else {
        return Err(String::from(
            "context-import-alias-segment requires an alpha token",
        ));
    };
    if first.kind() != SyntaxKind::Alpha {
        return Err(String::from(
            "context-import-alias-segment must begin with an alpha token",
        ));
    }
    if tokens
        .windows(2)
        .any(|pair| pair[0].range().end != pair[1].range().start)
    {
        return Err(String::from(
            "context-import-alias-segment tokens must be contiguous",
        ));
    }

    let mut lowered = Vec::with_capacity(tokens.len());
    for token in &tokens {
        let kind = match token.kind() {
            SyntaxKind::Alpha => TokenKind::Alpha,
            SyntaxKind::Digit => TokenKind::Digit,
            SyntaxKind::Dash => TokenKind::Dash,
            _ => {
                return Err(String::from(
                    "context-import-alias-segment contains an unsupported token",
                ));
            }
        };
        lowered.push(common::lower_syntax_token(syntax, token, kind)?);
    }
    let mut name = common::merge_legacy_tokens(&mut lowered, "context-import-alias-segment")?;
    name.kind = TokenKind::Identifier;
    Ok(Identifier { name })
}

fn lower_module_import_context_alias_node(syntax: &SyntaxNode) -> LowerResult<Identifier> {
    common::validate_node(
        syntax,
        SyntaxKind::ModuleImportContextAlias,
        "module-import-context-alias",
    )?;
    let elements = syntax.children_with_tokens();
    let [SyntaxElement::Token(at), SyntaxElement::Node(segment)] = elements.as_slice() else {
        return Err(String::from(
            "module-import-context-alias requires `@` and a context alias segment",
        ));
    };
    require_token(at, SyntaxKind::At, "@")?;
    if segment.kind() != SyntaxKind::ContextImportAliasSegment {
        return Err(String::from(
            "module-import-context-alias requires a context alias segment",
        ));
    }
    lower_context_import_alias_segment_node(segment)
}

fn lower_module_import_alias_node(syntax: &SyntaxNode) -> LowerResult<ModuleImportAlias> {
    common::validate_node(syntax, SyntaxKind::ModuleImportAlias, "module-import-alias")?;
    let child = only_child(syntax, "module-import-alias")?;
    match child.kind() {
        SyntaxKind::ModuleImportContextAlias => {
            lower_module_import_context_alias_node(&child).map(ModuleImportAlias::Context)
        }
        SyntaxKind::ModuleImportValueAlias => lower_module_import_value_alias_node(&child),
        _ => Err(String::from(
            "module-import-alias selected an unsupported child",
        )),
    }
}

fn lower_module_root_node(syntax: &SyntaxNode) -> LowerResult<Identifier> {
    common::validate_node(syntax, SyntaxKind::ModuleRoot, "module-root")?;
    let identifier = only_child_node(syntax, SyntaxKind::IdentifierPathSegment, "module-root")?;
    lower_identifier_path(&identifier)
}

fn lower_import_group_item_node(syntax: &SyntaxNode) -> LowerResult<ModuleImportGroupItem> {
    common::validate_node(syntax, SyntaxKind::ImportGroupItem, "import-group-item")?;
    let path = only_child_node(syntax, SyntaxKind::ModuleImportPath, "import-group-item")?;
    Ok(ModuleImportGroupItem {
        item: lower_module_import_path_node(&path)?,
    })
}

fn lower_import_group_items_node(syntax: &SyntaxNode) -> LowerResult<Vec<ModuleImportGroupItem>> {
    common::validate_node(syntax, SyntaxKind::ImportGroupItems, "import-group-items")?;
    let mut items = Vec::new();
    for element in syntax.children_with_tokens() {
        match element {
            SyntaxElement::Node(node) if node.kind() == SyntaxKind::ImportGroupItem => {
                items.push(lower_import_group_item_node(&node)?);
            }
            SyntaxElement::Token(token) if is_group_separator_token(&token) => {
                common::validate_token(&token)?;
            }
            _ => {
                return Err(String::from(
                    "import-group-items contains an unsupported direct element",
                ));
            }
        }
    }
    if items.is_empty() {
        return Err(String::from(
            "import-group-items requires at least one item",
        ));
    }
    Ok(items)
}

fn lower_aliased_item_import_node(syntax: &SyntaxNode) -> LowerResult<ModuleImport> {
    common::validate_node(syntax, SyntaxKind::AliasedItemImport, "aliased-item-import")?;
    let elements = non_horizontal_elements(syntax, "aliased-item-import")?;
    let [
        SyntaxElement::Node(alias),
        SyntaxElement::Token(colon),
        SyntaxElement::Token(equal),
        SyntaxElement::Node(module),
        SyntaxElement::Token(slash),
        SyntaxElement::Node(item),
    ] = elements.as_slice()
    else {
        return Err(String::from(
            "aliased-item-import requires alias, `:=`, module root, and item path",
        ));
    };
    if alias.kind() != SyntaxKind::ModuleImportAlias
        || module.kind() != SyntaxKind::ModuleRoot
        || item.kind() != SyntaxKind::ModuleImportPath
    {
        return Err(String::from(
            "aliased-item-import contains unsupported structural children",
        ));
    }
    require_token(colon, SyntaxKind::Colon, ":")?;
    require_token(equal, SyntaxKind::Equal, "=")?;
    require_token(slash, SyntaxKind::Slash, "/")?;
    Ok(ModuleImport {
        module: lower_module_root_node(module)?,
        item: Some(lower_module_import_path_node(item)?),
        group_items: None,
        alias: Some(lower_module_import_alias_node(alias)?),
        kind: ModuleImportKind::Item,
    })
}

fn lower_module_suffix_import_node(syntax: &SyntaxNode) -> LowerResult<ModuleImport> {
    common::validate_node(
        syntax,
        SyntaxKind::ModuleSuffixImport,
        "module-suffix-import",
    )?;
    let elements = syntax.children_with_tokens();
    let [
        SyntaxElement::Node(module),
        SyntaxElement::Token(slash),
        tail @ ..,
    ] = elements.as_slice()
    else {
        return Err(String::from(
            "module-suffix-import requires a module root and slash",
        ));
    };
    if module.kind() != SyntaxKind::ModuleRoot {
        return Err(String::from(
            "module-suffix-import requires a module-root child",
        ));
    }
    require_token(slash, SyntaxKind::Slash, "/")?;
    let module = lower_module_root_node(module)?;

    match tail {
        [SyntaxElement::Token(asterisk)] => {
            require_token(asterisk, SyntaxKind::Asterisk, "*")?;
            Ok(ModuleImport {
                module,
                item: None,
                group_items: None,
                alias: None,
                kind: ModuleImportKind::Glob,
            })
        }
        [
            SyntaxElement::Token(left_brace),
            SyntaxElement::Node(items),
            SyntaxElement::Token(right_brace),
        ] => {
            require_token(left_brace, SyntaxKind::LeftBrace, "{")?;
            require_token(right_brace, SyntaxKind::RightBrace, "}")?;
            if items.kind() != SyntaxKind::ImportGroupItems {
                return Err(String::from(
                    "module-suffix-import group requires import-group-items",
                ));
            }
            Ok(ModuleImport {
                module,
                item: None,
                group_items: Some(lower_import_group_items_node(items)?),
                alias: None,
                kind: ModuleImportKind::Group,
            })
        }
        [SyntaxElement::Node(item)] if item.kind() == SyntaxKind::ModuleImportPath => {
            Ok(ModuleImport {
                module,
                item: Some(lower_module_import_path_node(item)?),
                group_items: None,
                alias: None,
                kind: ModuleImportKind::Item,
            })
        }
        _ => Err(String::from(
            "module-suffix-import selected an unsupported suffix",
        )),
    }
}

fn lower_module_only_import_node(syntax: &SyntaxNode) -> LowerResult<ModuleImport> {
    common::validate_node(syntax, SyntaxKind::ModuleOnlyImport, "module-only-import")?;
    let module = only_child_node(syntax, SyntaxKind::ModuleRoot, "module-only-import")?;
    Ok(ModuleImport {
        module: lower_module_root_node(&module)?,
        item: None,
        group_items: None,
        alias: None,
        kind: ModuleImportKind::Module,
    })
}

fn lower_module_import_node(syntax: &SyntaxNode) -> LowerResult<ModuleImport> {
    common::validate_node(syntax, SyntaxKind::ModuleImport, "module-import")?;
    let mut sigil_count = 0_usize;
    let mut body = None;
    for element in syntax.children_with_tokens() {
        match element {
            SyntaxElement::Token(token) if is_import_space_token(&token) => {
                common::validate_token(&token)?;
            }
            SyntaxElement::Token(token) => {
                require_token(&token, SyntaxKind::ModuleImportSigil, "+>")?;
                sigil_count = sigil_count.saturating_add(1);
            }
            SyntaxElement::Node(node) if body.is_none() => {
                body = Some(node);
            }
            SyntaxElement::Node(_) => {
                return Err(String::from(
                    "module-import contains more than one body child",
                ));
            }
        }
    }
    if sigil_count != 1 {
        return Err(String::from(
            "module-import requires exactly one exact import sigil",
        ));
    }
    let body = body.ok_or_else(|| String::from("module-import requires a body child"))?;
    match body.kind() {
        SyntaxKind::AliasedItemImport => lower_aliased_item_import_node(&body),
        SyntaxKind::ModuleSuffixImport => lower_module_suffix_import_node(&body),
        SyntaxKind::ModuleOnlyImport => lower_module_only_import_node(&body),
        _ => Err(String::from(
            "module-import selected an unsupported body child",
        )),
    }
}

fn lower_identifier_path(syntax: &SyntaxNode) -> LowerResult<Identifier> {
    lower_legacy_identifier_path_segment(syntax)
        .map_err(|_| String::from("invalid identifier-path-segment syntax"))
}

fn only_child(syntax: &SyntaxNode, name: &str) -> LowerResult<SyntaxNode> {
    let elements = syntax.children_with_tokens();
    let [SyntaxElement::Node(child)] = elements.as_slice() else {
        return Err(alloc::format!("{name} requires exactly one child node"));
    };
    Ok(child.clone())
}

fn only_child_node(
    syntax: &SyntaxNode,
    expected_kind: SyntaxKind,
    name: &str,
) -> LowerResult<SyntaxNode> {
    let child = only_child(syntax, name)?;
    if child.kind() != expected_kind {
        return Err(alloc::format!("{name} requires a {expected_kind:?} child"));
    }
    Ok(child)
}

fn non_horizontal_elements(syntax: &SyntaxNode, name: &str) -> LowerResult<Vec<SyntaxElement>> {
    let mut elements = Vec::new();
    for element in syntax.children_with_tokens() {
        match &element {
            SyntaxElement::Token(token) if is_horizontal_space_token(token) => {
                common::validate_token(token)?;
            }
            SyntaxElement::Token(token) => {
                common::validate_token(token)?;
                elements.push(element);
            }
            SyntaxElement::Node(node) => {
                common::validate_clean_node(node, name)?;
                elements.push(element);
            }
        }
    }
    Ok(elements)
}

fn is_horizontal_space_token(token: &SyntaxToken) -> bool {
    matches!(token.kind(), SyntaxKind::Whitespace | SyntaxKind::Tab)
}

fn is_import_space_token(token: &SyntaxToken) -> bool {
    matches!(
        token.kind(),
        SyntaxKind::Whitespace | SyntaxKind::Tab | SyntaxKind::Newline | SyntaxKind::CarriageReturn
    )
}

fn is_group_separator_token(token: &SyntaxToken) -> bool {
    matches!(
        token.kind(),
        SyntaxKind::Whitespace
            | SyntaxKind::Tab
            | SyntaxKind::Newline
            | SyntaxKind::CarriageReturn
            | SyntaxKind::Comma
    )
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
