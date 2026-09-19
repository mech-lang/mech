use mech_syntax::document::ast::{
    AliasedItemImportSyntax, CanonicalModuleImportBodySyntax, CanonicalModuleImportKind,
    ContextImportAliasSegmentSyntax, ImportGroupItemsSyntax, ModuleImportAliasPathSyntax,
    ModuleImportAliasSegmentSyntax, ModuleImportAliasSyntax, ModuleImportContextAliasSyntax,
    ModuleImportIntrinsicSegmentSyntax, ModuleImportNameSegmentSyntax,
    ModuleImportPathSegmentSyntax, ModuleImportPathSyntax, ModuleImportSyntax,
    ModuleImportValueAliasSyntax, ModuleOnlyImportSyntax, ModuleRootSyntax,
    ModuleSuffixImportSyntax,
};
use mech_syntax::document::parser::canonical::parse_canonical_phase_2e_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, ParseConfig, Revision, RuleId, SyntaxKind, SyntaxNode, TextSnapshot,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(928), Revision(0), text).unwrap()
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

fn parse_typed<N: AstNode>(input: &str, rule: RuleId, kind: SyntaxKind) -> N {
    let parsed =
        parse_canonical_phase_2e_rule_for_test(source(input), rule, ParseConfig::default())
            .unwrap_or_else(|| panic!("{rule:?} is not a Phase 2E direct rule"));
    assert!(parsed.is_strictly_clean(), "{rule:?} on {input:?}");
    assert_eq!(parsed.consumed.end, parsed.source.byte_len(), "{input:?}");
    let node = find_node(&parsed.syntax(), kind)
        .unwrap_or_else(|| panic!("{rule:?} did not emit {kind:?} for {input:?}"));
    N::cast(node).unwrap_or_else(|| panic!("{kind:?} did not cast for {input:?}"))
}

fn syntax_text<N: AstNode>(node: N) -> String {
    node.syntax().text().unwrap()
}

#[test]
fn path_and_intrinsic_views_preserve_segment_shape_and_order() {
    let name = parse_typed::<ModuleImportNameSegmentSyntax>(
        "math",
        rules::MODULE_IMPORT_NAME_SEGMENT,
        SyntaxKind::ModuleImportNameSegment,
    );
    assert_eq!(syntax_text(name.identifier().unwrap()), "math");

    let intrinsic = parse_typed::<ModuleImportIntrinsicSegmentSyntax>(
        "_internal",
        rules::MODULE_IMPORT_INTRINSIC_SEGMENT,
        SyntaxKind::ModuleImportIntrinsicSegment,
    );
    assert_eq!(intrinsic.marker().unwrap().text().unwrap(), "_");
    assert_eq!(syntax_text(intrinsic.name().unwrap()), "internal");

    let named = parse_typed::<ModuleImportPathSegmentSyntax>(
        "math",
        rules::MODULE_IMPORT_PATH_SEGMENT,
        SyntaxKind::ModuleImportPathSegment,
    );
    assert_eq!(syntax_text(named.name().unwrap()), "math");
    assert!(named.intrinsic().is_none());

    let intrinsic_segment = parse_typed::<ModuleImportPathSegmentSyntax>(
        "_internal",
        rules::MODULE_IMPORT_PATH_SEGMENT,
        SyntaxKind::ModuleImportPathSegment,
    );
    assert!(intrinsic_segment.name().is_none());
    assert_eq!(
        syntax_text(intrinsic_segment.intrinsic().unwrap()),
        "_internal"
    );

    let path = parse_typed::<ModuleImportPathSyntax>(
        "math/trig/_internal",
        rules::MODULE_IMPORT_PATH,
        SyntaxKind::ModuleImportPath,
    );
    assert_eq!(
        path.segments()
            .map(|segment| segment.syntax().text().unwrap())
            .collect::<Vec<_>>(),
        vec!["math", "trig", "_internal"],
    );
}

#[test]
fn alias_and_group_views_select_their_structural_children() {
    let alias_segment = parse_typed::<ModuleImportAliasSegmentSyntax>(
        "alias",
        rules::MODULE_IMPORT_ALIAS_SEGMENT,
        SyntaxKind::ModuleImportAliasSegment,
    );
    assert_eq!(syntax_text(alias_segment.identifier().unwrap()), "alias");

    let alias_path = parse_typed::<ModuleImportAliasPathSyntax>(
        "alias/path",
        rules::MODULE_IMPORT_ALIAS_PATH,
        SyntaxKind::ModuleImportAliasPath,
    );
    assert_eq!(
        alias_path
            .segments()
            .map(|segment| segment.syntax().text().unwrap())
            .collect::<Vec<_>>(),
        vec!["alias", "path"],
    );

    let value_alias = parse_typed::<ModuleImportValueAliasSyntax>(
        "alias/path",
        rules::MODULE_IMPORT_VALUE_ALIAS,
        SyntaxKind::ModuleImportValueAlias,
    );
    assert_eq!(syntax_text(value_alias.path().unwrap()), "alias/path");

    let context_segment = parse_typed::<ContextImportAliasSegmentSyntax>(
        "ctx-2",
        rules::CONTEXT_IMPORT_ALIAS_SEGMENT,
        SyntaxKind::ContextImportAliasSegment,
    );
    assert_eq!(
        context_segment
            .tokens()
            .into_iter()
            .map(|token| token.text().unwrap())
            .collect::<String>(),
        "ctx-2",
    );

    let context_alias = parse_typed::<ModuleImportContextAliasSyntax>(
        "@ctx-2",
        rules::MODULE_IMPORT_CONTEXT_ALIAS,
        SyntaxKind::ModuleImportContextAlias,
    );
    assert_eq!(syntax_text(context_alias.name().unwrap()), "ctx-2");

    let context = parse_typed::<ModuleImportAliasSyntax>(
        "@ctx-2",
        rules::MODULE_IMPORT_ALIAS,
        SyntaxKind::ModuleImportAlias,
    );
    assert!(context.value().is_none());
    assert_eq!(syntax_text(context.context().unwrap()), "@ctx-2");

    let value = parse_typed::<ModuleImportAliasSyntax>(
        "alias/path",
        rules::MODULE_IMPORT_ALIAS,
        SyntaxKind::ModuleImportAlias,
    );
    assert!(value.context().is_none());
    let path = value.value().unwrap().path().unwrap();
    assert_eq!(
        path.segments()
            .map(|segment| segment.syntax().text().unwrap())
            .collect::<Vec<_>>(),
        vec!["alias", "path"],
    );

    let group = parse_typed::<ImportGroupItemsSyntax>(
        "sin,cos trig",
        rules::IMPORT_GROUP_ITEMS,
        SyntaxKind::ImportGroupItems,
    );
    assert_eq!(
        group
            .items()
            .map(|item| item.path().unwrap().syntax().text().unwrap())
            .collect::<Vec<_>>(),
        vec!["sin", "cos", "trig"],
    );
}

#[test]
fn complete_import_views_report_all_module_import_kinds_from_structure() {
    let root = parse_typed::<ModuleRootSyntax>("math", rules::MODULE_ROOT, SyntaxKind::ModuleRoot);
    assert_eq!(syntax_text(root.identifier().unwrap()), "math");

    let aliased = parse_typed::<AliasedItemImportSyntax>(
        "alias := math/sin",
        rules::ALIASED_ITEM_IMPORT,
        SyntaxKind::AliasedItemImport,
    );
    assert_eq!(syntax_text(aliased.alias().unwrap()), "alias");
    assert_eq!(syntax_text(aliased.module().unwrap()), "math");
    assert_eq!(syntax_text(aliased.item().unwrap()), "sin");

    let glob = parse_typed::<ModuleSuffixImportSyntax>(
        "math/*",
        rules::MODULE_SUFFIX_IMPORT,
        SyntaxKind::ModuleSuffixImport,
    );
    assert!(glob.is_glob());
    assert!(glob.item().is_none());
    assert!(glob.group().is_none());
    assert_eq!(syntax_text(glob.module().unwrap()), "math");

    let group = parse_typed::<ModuleSuffixImportSyntax>(
        "math/{sin,cos}",
        rules::MODULE_SUFFIX_IMPORT,
        SyntaxKind::ModuleSuffixImport,
    );
    assert!(!group.is_glob());
    assert!(group.item().is_none());
    assert_eq!(
        group
            .group()
            .unwrap()
            .items()
            .map(|item| item.path().unwrap().syntax().text().unwrap())
            .collect::<Vec<_>>(),
        vec!["sin", "cos"],
    );

    let item = parse_typed::<ModuleSuffixImportSyntax>(
        "math/trig/sin",
        rules::MODULE_SUFFIX_IMPORT,
        SyntaxKind::ModuleSuffixImport,
    );
    assert!(!item.is_glob());
    assert!(item.group().is_none());
    assert_eq!(syntax_text(item.item().unwrap()), "trig/sin");

    let module = parse_typed::<ModuleOnlyImportSyntax>(
        "math",
        rules::MODULE_ONLY_IMPORT,
        SyntaxKind::ModuleOnlyImport,
    );
    assert_eq!(syntax_text(module.module().unwrap()), "math");
}

#[test]
fn module_import_view_derives_kind_without_searching_source_text() {
    for (input, expected) in [
        ("+> math", CanonicalModuleImportKind::Module),
        ("+> math/sin", CanonicalModuleImportKind::Item),
        ("+> math/*", CanonicalModuleImportKind::Glob),
        ("+> math/{sin,cos}", CanonicalModuleImportKind::Group),
        ("+> alias := math/sin", CanonicalModuleImportKind::Item),
    ] {
        let import = parse_typed::<ModuleImportSyntax>(
            input,
            rules::MODULE_IMPORT,
            SyntaxKind::ModuleImport,
        );
        assert_eq!(import.import_kind(), Some(expected), "{input:?}");
        match (expected, import.body().unwrap()) {
            (CanonicalModuleImportKind::Module, CanonicalModuleImportBodySyntax::Module(_))
            | (CanonicalModuleImportKind::Item, CanonicalModuleImportBodySyntax::AliasedItem(_))
            | (CanonicalModuleImportKind::Item, CanonicalModuleImportBodySyntax::Suffix(_))
            | (CanonicalModuleImportKind::Glob, CanonicalModuleImportBodySyntax::Suffix(_))
            | (CanonicalModuleImportKind::Group, CanonicalModuleImportBodySyntax::Suffix(_)) => {}
            (_, body) => panic!("unexpected structural body for {input:?}: {body:?}"),
        }
    }
}
