use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn classify(source: &str) -> bool {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x57a), Revision(0), source).unwrap(),
        ParseConfig::default(),
    );
    DocumentSyntax::cast(parsed.syntax())
        .unwrap()
        .contains_executable_source()
}

#[test]
fn canonical_classification_distinguishes_run_source_from_presentation() {
    for source in [
        "answer := 42\n",
        "@ui/message <- \"hello\"\n",
        "```mech\nx := 1\n```\n",
        "```mech:worker\nx := 1\n```\n",
        "```mechworker\nx := 1\n```\n",
        "```mech unknown\nx := 1\n```\n",
        "Evaluated {1 + 2}.\n",
        include_str!("../../../tests/fixtures/syntax-source-boundary/executable.mec"),
        include_str!("../../../tests/fixtures/syntax-source-boundary/document.mec"),
    ] {
        assert!(classify(source), "expected executable source: {source:?}");
    }
    for source in [
        "",
        "Just prose.\n",
        "// comment\n",
        "-- comment\n",
        "+> ./dep.mec\n",
        "+> @env := cli/env\n",
        "<+ value\n",
        "@ui := fs://workspace\n",
        "Displayed {{x := 1}}.\n",
        "Code `x := 1`.\n",
        "```mech:disabled\nx := 1\n```\n",
        "```rust\nfn main() {}\n```\n",
        "```mech\n// comment\n```\n",
        "~∘~⸢x := 1\n⸥\n",
        include_str!("../../../tests/fixtures/syntax-source-boundary/empty.mec"),
        include_str!("../../../tests/fixtures/syntax-source-boundary/malformed.mec"),
    ] {
        assert!(
            !classify(source),
            "unexpected executable source: {source:?}"
        );
    }
}

#[test]
fn recovery_cannot_promote_a_valid_prefix_to_executable_input() {
    assert!(!classify("x := 1\ny := [1,,2]\n"));
    assert!(classify("// comment\nx := 1\n"));
}

#[test]
fn program_classification_includes_declarations_without_calling_them_executable() {
    for source in [
        "+> ./dep.mec\n",
        "+> @env := cli/env\n",
        "<+ value\n",
        "@ui := fs://workspace\n",
        "```mech\n+> @out := cli/stdout\n```\n",
    ] {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x57b), Revision(0), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(
            parsed.diagnostics.is_empty(),
            "{source:?}: {:?}",
            parsed.diagnostics
        );
        let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
        assert!(document.contains_program_source(), "{source:?}");
        assert!(!document.contains_executable_source(), "{source:?}");
    }
    for source in [
        "-- comment\n",
        "Displayed {{+> ./dep.mec}}.\n",
        "```mech:disabled\n+> ./dep.mec\n```\n",
    ] {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x57c), Revision(0), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(
            !DocumentSyntax::cast(parsed.syntax())
                .unwrap()
                .contains_program_source(),
            "{source:?}"
        );
    }
}
