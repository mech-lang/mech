use mech_core::nodes::{ModuleImportAlias, ModuleImportPathSegment};
use mech_syntax::document::ast::{
    ModuleImportAliasSyntax, ModuleImportPathSyntax, ModuleImportSyntax,
};
use mech_syntax::document::parser::canonical::parse_canonical_phase_2e_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DiagnosticPhase, DocumentId, ParseConfig, Revision, RuleId, SyntaxKind, SyntaxNode,
    TextSnapshot, lower_legacy_module_import, lower_legacy_module_import_alias,
    lower_legacy_module_import_path,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(929), Revision(0), text).unwrap()
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

fn canonical<N: AstNode>(input: &str, rule: RuleId, kind: SyntaxKind) -> N {
    let parsed =
        parse_canonical_phase_2e_rule_for_test(source(input), rule, ParseConfig::default())
            .unwrap_or_else(|| panic!("{rule:?} is not a Phase 2E direct rule"));
    assert!(parsed.is_strictly_clean(), "{rule:?} on {input:?}");
    assert_eq!(parsed.consumed.end, parsed.source.byte_len(), "{input:?}");
    let node = find_node(&parsed.syntax(), kind)
        .unwrap_or_else(|| panic!("{rule:?} did not emit {kind:?} for {input:?}"));
    N::cast(node).unwrap_or_else(|| panic!("{kind:?} did not cast for {input:?}"))
}

fn canonical_module_prefix(input: &str) -> ModuleImportSyntax {
    let parsed = parse_canonical_phase_2e_rule_for_test(
        source(input),
        rules::MODULE_IMPORT,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(parsed.is_strictly_clean(), "{input:?}");
    ModuleImportSyntax::cast(find_node(&parsed.syntax(), SyntaxKind::ModuleImport).unwrap())
        .unwrap()
}

fn legacy_module(input: &str) -> mech_core::nodes::ModuleImport {
    let graphemes = mech_syntax::graphemes::init_tag(input);
    let (remaining, value) = mech_syntax::module_import(mech_syntax::ParseString::new(&graphemes))
        .unwrap_or_else(|error| panic!("legacy parser rejected {input:?}: {error:?}"));
    assert_eq!(remaining.cursor, graphemes.len(), "{input:?}");
    assert!(remaining.error_log.is_empty(), "{input:?}");
    value
}

#[test]
fn complete_module_import_lowering_matches_corrected_legacy_values_exactly() {
    for input in [
        "+>math",
        "+> math",
        "+>\tmath",
        "+>\u{00a0}math",
        "+>\u{2009}math",
        "+> math/sin",
        "+> math/trig/sin",
        "+> math/*",
        "+> math/{sin,cos}",
        "+> math/{sin cos}",
        "+> math/{sin\ncos}",
        "+> math/{trig/sin,stats/mean}",
        "+> math/_intrinsic",
        "+> math/path/_intrinsic",
        "+> alias := math/sin",
        "+> alias/path := math/sin",
        "+> @ctx := math/sin",
        "+> 💡",
        "+> 💡/run",
        "+> math/emoji-name",
    ] {
        let canonical = lower_legacy_module_import(&canonical::<ModuleImportSyntax>(
            input,
            rules::MODULE_IMPORT,
            SyntaxKind::ModuleImport,
        ))
        .unwrap_or_else(|error| panic!("canonical lowering rejected {input:?}: {error:?}"));
        assert_eq!(canonical, legacy_module(input), "{input:?}");
    }
}

#[test]
fn path_and_alias_lowerers_preserve_structural_variants() {
    let path = lower_legacy_module_import_path(&canonical::<ModuleImportPathSyntax>(
        "math/_internal",
        rules::MODULE_IMPORT_PATH,
        SyntaxKind::ModuleImportPath,
    ))
    .unwrap();
    assert_eq!(path.to_string(), "math/_internal");
    assert!(matches!(path.segments[0], ModuleImportPathSegment::Name(_)));
    assert!(matches!(
        path.segments[1],
        ModuleImportPathSegment::Intrinsic(_)
    ));

    let value = lower_legacy_module_import_alias(&canonical::<ModuleImportAliasSyntax>(
        "alias/path",
        rules::MODULE_IMPORT_ALIAS,
        SyntaxKind::ModuleImportAlias,
    ))
    .unwrap();
    assert_eq!(value.to_string(), "alias/path");
    assert!(matches!(value, ModuleImportAlias::Value(_)));

    let context = lower_legacy_module_import_alias(&canonical::<ModuleImportAliasSyntax>(
        "@ctx-2",
        rules::MODULE_IMPORT_ALIAS,
        SyntaxKind::ModuleImportAlias,
    ))
    .unwrap();
    let ModuleImportAlias::Context(identifier) = context else {
        panic!("expected a context alias");
    };
    assert_eq!(identifier.to_string(), "ctx-2");
    assert_eq!(identifier.name.to_string(), "ctx-2");
}

#[test]
fn trailing_source_is_not_included_in_the_lowered_module_identifier_range() {
    let input = "+> math   \n";
    let canonical = lower_legacy_module_import(&canonical_module_prefix(input)).unwrap();

    let graphemes = mech_syntax::graphemes::init_tag(input);
    let (remaining, legacy) = mech_syntax::module_import(mech_syntax::ParseString::new(&graphemes))
        .expect("legacy module import prefix");
    assert_eq!(remaining.rest(), "   \n");
    assert_eq!(canonical, legacy);
    assert_eq!(canonical.module.to_string(), "math");
    assert_eq!(canonical.module.name.src_range.end.col, 8);
}

#[test]
fn public_module_lowerer_rejects_recovered_incomplete_imports() {
    let parsed = parse_canonical_phase_2e_rule_for_test(
        source("+> math/"),
        rules::MODULE_IMPORT,
        ParseConfig::default(),
    )
    .unwrap();
    let syntax =
        ModuleImportSyntax::cast(find_node(&parsed.syntax(), SyntaxKind::ModuleImport).unwrap())
            .unwrap();
    let error = lower_legacy_module_import(&syntax).unwrap_err();
    assert_eq!(error.as_slice()[0].phase, DiagnosticPhase::Lowering);
}
