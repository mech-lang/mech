use mech_core::nodes::{MechCode, SourceLocation, Statement, TokenKind};
use mech_syntax::{
    ParseString, graphemes, import_declaration, import_sigil, mech_code_alt, module_import,
    right_angle,
};

fn code(source: &str) -> MechCode {
    let graphemes = graphemes::init_source(source);
    let (remaining, code) = mech_code_alt(ParseString::new(&graphemes))
        .unwrap_or_else(|error| panic!("expected mech code for {source:?}: {error:?}"));
    assert_eq!(
        remaining.rest(),
        "\n",
        "unexpected remainder for {source:?}"
    );
    code
}

#[test]
fn corrected_import_sigil_is_exact_without_changing_right_angle() {
    let graphemes = graphemes::init_tag("+>");
    let (remaining, token) = import_sigil(ParseString::new(&graphemes)).expect("exact sigil");
    assert_eq!(token.kind, TokenKind::ModuleImportSigil);
    assert!(remaining.rest().is_empty());

    for source in ["+⟩", "+", ">"] {
        let graphemes = graphemes::init_tag(source);
        assert!(
            import_sigil(ParseString::new(&graphemes)).is_err(),
            "{source:?} must not be an import sigil"
        );
    }

    let graphemes = graphemes::init_tag("⟩");
    let (remaining, _) =
        right_angle(ParseString::new(&graphemes)).expect("right-angle remains independently valid");
    assert!(remaining.rest().is_empty());
}

#[test]
fn corrected_module_import_spacing_is_horizontal_only() {
    for source in [
        "+>math",
        "+> math",
        "+>\tmath",
        "+>\u{00a0}math",
        "+>\u{2009}math",
    ] {
        let graphemes = graphemes::init_source(source);
        let (remaining, import) = module_import(ParseString::new(&graphemes))
            .unwrap_or_else(|error| panic!("expected module import for {source:?}: {error:?}"));
        assert_eq!(import.module.to_string(), "math");
        assert_eq!(remaining.rest(), "\n");
    }

    for source in ["+⟩ math", "+>", "+> ", "+>\nmath"] {
        let graphemes = graphemes::init_source(source);
        assert!(
            module_import(ParseString::new(&graphemes)).is_err(),
            "{source:?} must not be a direct module import"
        );
    }
}

#[test]
fn corrected_source_import_spacing_requires_horizontal_space() {
    for source in [
        "+> foo.mec",
        "+>\tfoo.mec",
        "+>\u{00a0}foo.mec",
        "+>\u{2009}foo.mec",
    ] {
        let graphemes = graphemes::init_source(source);
        let (remaining, import) = import_declaration(ParseString::new(&graphemes))
            .unwrap_or_else(|error| panic!("expected source import for {source:?}: {error:?}"));
        assert_eq!(import.specifier.to_string(), "foo.mec");
        assert_eq!(remaining.rest(), "\n");
    }

    for source in ["+>foo.mec", "+>\nfoo.mec", "+⟩ foo.mec"] {
        let graphemes = graphemes::init_source(source);
        assert!(
            import_declaration(ParseString::new(&graphemes)).is_err(),
            "{source:?} must not be a source import declaration"
        );
    }
}

#[test]
fn complete_legacy_selection_prefers_source_imports_after_full_parse() {
    for source in ["+> math", "+> math/sin"] {
        assert!(
            matches!(code(source), MechCode::Import(_)),
            "{source:?} should select the module import alternative"
        );
    }

    for source in [
        "+> foo.mec",
        "+> ./foo.mec",
        "+> ../foo.mec",
        "+> /foo.mec",
        "+> https://example.com/foo.mec",
        "+> path/to/foo.mec",
        "+> path/to/foo.mec/*",
    ] {
        assert!(
            matches!(
                code(source),
                MechCode::Statement(Statement::ImportDeclaration(_))
            ),
            "{source:?} should select the complete source import alternative"
        );
    }
}

#[test]
fn corrected_module_import_range_stops_at_the_module_root() {
    for (source, trailing) in [
        ("+> math", ""),
        ("+> math   ", "   "),
        ("+> math\n", "\n"),
        ("+> math   \n", "   \n"),
    ] {
        let graphemes = graphemes::init_source(source);
        let (remaining, import) = module_import(ParseString::new(&graphemes))
            .unwrap_or_else(|error| panic!("expected module import for {source:?}: {error:?}"));
        assert_eq!(import.module.to_string(), "math");
        assert_eq!(
            import.module.name.src_range.end,
            SourceLocation { row: 1, col: 8 },
            "module identifier range should end immediately after `math`"
        );
        assert_eq!(remaining.rest(), format!("{trailing}\n"));
    }
}
