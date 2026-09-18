use mech_syntax::document::parser::Cursor;
use mech_syntax::document::{
    DocumentId, ParseConfig, ParseRequestError, ParseRoot, ParserImplementation, Revision,
    SyntaxKind, SyntaxNode, TextSize, TextSnapshot, TokenFlags, parse_canonical_grammar,
    parse_syntax, reconstruct_source, validate_lossless,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(41), Revision(0), text).unwrap()
}

fn piece_source(parts: &[&str]) -> TextSnapshot {
    let mut source = source("");
    for part in parts {
        source = source.append((*part).to_owned()).unwrap();
    }
    assert_eq!(source.piece_count(), parts.len());
    source
}

fn parse(text: &str) -> mech_syntax::document::SyntaxSnapshot {
    parse_canonical_grammar(source(text), ParseConfig::default())
}

fn nodes_of_kind(root: &SyntaxNode, kind: SyntaxKind) -> Vec<SyntaxNode> {
    let mut nodes = Vec::new();
    if root.kind() == kind {
        nodes.push(root.clone());
    }
    for child in root.children() {
        nodes.extend(nodes_of_kind(&child, kind));
    }
    nodes
}

#[test]
fn canonical_grammar_represents_every_expression_variant() {
    let text = concat!(
        "definition := name;",
        "terminal := \"a\";",
        "choice := \"a\" | \"b\";",
        "sequence := name, \"b\";",
        "repeat-zero := *name;",
        "repeat-one := +name;",
        "optional := ?name;",
        "peek-ascii := >name;",
        "peek-unicode := ⟩name;",
        "not := ¬name;",
        "list := [name, \",\"];",
        "range := \"a\"..\"z\";",
        "group := (name | \"b\");",
    );
    let snapshot = parse(text);
    assert!(
        snapshot.diagnostics.is_empty(),
        "{:#?}",
        snapshot.diagnostics.as_slice()
    );
    validate_lossless(&snapshot.root, &snapshot.source).unwrap();
    assert_eq!(
        reconstruct_source(&snapshot.root, &snapshot.source).unwrap(),
        text
    );

    let root = snapshot.syntax();
    assert_eq!(root.kind(), SyntaxKind::GrammarDocument);
    let grammar = root.first_child(SyntaxKind::Grammar).unwrap();
    assert_eq!(
        grammar
            .children()
            .filter(|child| child.kind() == SyntaxKind::GrammarRule)
            .count(),
        13
    );
    for (kind, expected) in [
        (SyntaxKind::GrammarDefinition, 10),
        (SyntaxKind::GrammarRepeat0, 1),
        (SyntaxKind::GrammarRepeat1, 1),
        (SyntaxKind::GrammarOptional, 1),
        (SyntaxKind::GrammarNot, 1),
        (SyntaxKind::GrammarList, 1),
        (SyntaxKind::GrammarRange, 1),
        (SyntaxKind::GrammarGroup, 1),
    ] {
        assert_eq!(
            nodes_of_kind(&root, kind).len(),
            expected,
            "wrong count for {kind:?}"
        );
    }
    assert_eq!(nodes_of_kind(&root, SyntaxKind::GrammarPeek).len(), 2);
    let synthetic = root
        .tokens()
        .into_iter()
        .filter(|token| token.flags().contains(TokenFlags::SYNTHETIC))
        .collect::<Vec<_>>();
    assert_eq!(synthetic.len(), 1);
    assert_eq!(synthetic[0].kind(), SyntaxKind::Newline);
    assert!(synthetic[0].flags().contains(TokenFlags::TRIVIA));
    assert!(synthetic[0].range().is_empty());
    assert_eq!(synthetic[0].range().start, snapshot.source.byte_len());
}

#[test]
fn grammar_filtering_is_lossless_and_preserves_canonical_values() {
    let text = concat!(
        "r u l e : = \"a b\" ;",
        "r a n g e := \"a\" . . \"z\";",
        "e \u{301} := \"x\";",
        "wide := \"a\u{00a0}\u{2009}b\";",
    );
    let snapshot = parse(text);
    assert!(
        snapshot.diagnostics.is_empty(),
        "{:#?}",
        snapshot.diagnostics.as_slice()
    );
    assert_eq!(
        reconstruct_source(&snapshot.root, &snapshot.source).unwrap(),
        text
    );
    assert!(
        snapshot
            .syntax()
            .tokens()
            .iter()
            .filter(|token| {
                matches!(token.kind(), SyntaxKind::Whitespace | SyntaxKind::Newline)
                    && !token.flags().contains(TokenFlags::SYNTHETIC)
            })
            .all(|token| token.flags().contains(TokenFlags::TRIVIA))
    );

    let root = snapshot.syntax();
    let identifiers = nodes_of_kind(&root, SyntaxKind::GrammarIdentifier)
        .into_iter()
        .map(|node| {
            node.text()
                .unwrap()
                .chars()
                .filter(|ch| !ch.is_whitespace())
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    assert_eq!(identifiers, ["rule", "range", "e\u{301}", "wide"]);
    let terminals = nodes_of_kind(&root, SyntaxKind::GrammarTerminalToken)
        .into_iter()
        .map(|node| node.text().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(terminals[0], "\"a b\"");
    assert_eq!(terminals[4], "\"a\u{00a0}\u{2009}b\"");
}

#[test]
fn grammar_literals_respect_graphemes_across_piece_boundaries() {
    let text = "rule := \"e\u{301}b\u{2764}\u{fe0f}\";";
    let snapshot = parse_canonical_grammar(
        piece_source(&[
            "rule := \"",
            "e",
            "\u{301}",
            "b",
            "\u{2764}",
            "\u{fe0f}",
            "\";",
        ]),
        ParseConfig::default(),
    );
    assert!(
        snapshot.diagnostics.is_empty(),
        "{:#?}",
        snapshot.diagnostics.as_slice()
    );
    validate_lossless(&snapshot.root, &snapshot.source).unwrap();
    assert_eq!(
        reconstruct_source(&snapshot.root, &snapshot.source).unwrap(),
        text
    );

    let terminals = nodes_of_kind(&snapshot.syntax(), SyntaxKind::GrammarTerminalToken);
    assert_eq!(terminals.len(), 1);
    assert_eq!(
        terminals[0].text().unwrap(),
        "\"e\u{301}b\u{2764}\u{fe0f}\""
    );
}

#[test]
fn clustered_quote_is_not_accepted_as_a_grammar_delimiter() {
    let text = "rule := \"\u{301}a\";";
    let snapshot = parse_canonical_grammar(
        piece_source(&["rule := \"", "\u{301}", "a\";"]),
        ParseConfig::default(),
    );
    assert!(!snapshot.diagnostics.is_empty());
    validate_lossless(&snapshot.root, &snapshot.source).unwrap();
    assert_eq!(
        reconstruct_source(&snapshot.root, &snapshot.source).unwrap(),
        text
    );

    let mut boundaries = vec![TextSize::ZERO];
    let mut cursor = Cursor::new(&snapshot.source);
    while let Some(range) = cursor.bump_grapheme() {
        boundaries.push(range.end);
    }
    for token in snapshot.syntax().tokens() {
        if token.range().is_empty() {
            continue;
        }
        assert!(
            boundaries.contains(&token.range().start) && boundaries.contains(&token.range().end),
            "{:?} splits an extended grapheme at {:?}",
            token.kind(),
            token.range(),
        );
    }
}

#[test]
fn clustered_quote_inside_a_terminal_remains_whole_content() {
    let text = "rule := \"a\"\u{301}b\";";
    let snapshot = parse_canonical_grammar(
        piece_source(&["rule := \"a\"", "\u{301}", "b\";"]),
        ParseConfig::default(),
    );
    assert!(
        snapshot.diagnostics.is_empty(),
        "{:#?}",
        snapshot.diagnostics.as_slice()
    );
    validate_lossless(&snapshot.root, &snapshot.source).unwrap();
    assert_eq!(
        reconstruct_source(&snapshot.root, &snapshot.source).unwrap(),
        text
    );

    let terminals = nodes_of_kind(&snapshot.syntax(), SyntaxKind::GrammarTerminalToken);
    assert_eq!(terminals.len(), 1);
    assert_eq!(terminals[0].text().unwrap(), "\"a\"\u{301}b\"");

    let clustered = snapshot
        .syntax()
        .tokens()
        .into_iter()
        .find(|token| token.text().as_deref() == Ok("\"\u{301}"))
        .expect("clustered quote content token");
    assert_eq!(clustered.kind(), SyntaxKind::Any);
}

#[test]
fn dispatcher_supports_the_canonical_document_root() {
    let config = ParseConfig::default();
    assert!(
        parse_syntax(
            source("x := \"a\";"),
            ParseRoot::Grammar,
            ParserImplementation::Canonical,
            config,
        )
        .is_ok()
    );
    assert!(
        parse_syntax(
            source("x := 1"),
            ParseRoot::Document,
            ParserImplementation::Prototype,
            config,
        )
        .is_ok()
    );
    assert!(
        parse_syntax(
            source("x := 1"),
            ParseRoot::Document,
            ParserImplementation::Canonical,
            config,
        )
        .is_ok()
    );
    let implementation = ParserImplementation::Prototype;
    let root = ParseRoot::Grammar;
    let error = parse_syntax(source(""), root, implementation, config).unwrap_err();
    assert_eq!(
        error,
        ParseRequestError::Unsupported {
            implementation,
            root,
        }
    );
}
