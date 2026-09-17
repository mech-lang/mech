use mech_core::{Grammar, GrammarExpression, Token};
use mech_syntax::document::parser::Cursor;
use mech_syntax::document::{
    DocumentId, ParseConfig, ParseRequestError, ParseRoot, ParserImplementation, Revision,
    SyntaxKind, TextSize, TextSnapshot, TokenFlags, lower_legacy_grammar, parse_canonical_grammar,
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

fn token_text(token: &Token) -> String {
    token.chars.iter().collect()
}

fn expression_shape(expression: &GrammarExpression) -> String {
    match expression {
        GrammarExpression::Choice(items) => format!(
            "choice({})",
            items
                .iter()
                .map(expression_shape)
                .collect::<Vec<_>>()
                .join(",")
        ),
        GrammarExpression::Definition(identifier) => {
            format!("definition({})", token_text(&identifier.name))
        }
        GrammarExpression::Group(item) => {
            format!("group({})", expression_shape(item))
        }
        GrammarExpression::List(first, second) => format!(
            "list({},{})",
            expression_shape(first),
            expression_shape(second)
        ),
        GrammarExpression::Not(item) => {
            format!("not({})", expression_shape(item))
        }
        GrammarExpression::Optional(item) => {
            format!("optional({})", expression_shape(item))
        }
        GrammarExpression::Peek(item) => {
            format!("peek({})", expression_shape(item))
        }
        GrammarExpression::Repeat0(item) => {
            format!("repeat0({})", expression_shape(item))
        }
        GrammarExpression::Repeat1(item) => {
            format!("repeat1({})", expression_shape(item))
        }
        GrammarExpression::Range(start, end) => {
            format!("range({},{})", token_text(start), token_text(end))
        }
        GrammarExpression::Sequence(items) => format!(
            "sequence({})",
            items
                .iter()
                .map(expression_shape)
                .collect::<Vec<_>>()
                .join(",")
        ),
        GrammarExpression::Terminal(token) => {
            format!("terminal({})", token_text(token))
        }
    }
}

fn grammar_shape(grammar: &Grammar) -> Vec<(String, String)> {
    grammar
        .rules
        .iter()
        .map(|rule| (token_text(&rule.name.name), expression_shape(&rule.expr)))
        .collect()
}

#[test]
fn canonical_grammar_lowers_to_every_legacy_expression_variant() {
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

    let canonical = lower_legacy_grammar(&snapshot).unwrap();
    let legacy = mech_syntax::parse_grammar(text).unwrap();
    assert_eq!(grammar_shape(&canonical), grammar_shape(&legacy));
    assert_eq!(canonical.rules.len(), 13);

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
fn grammar_filtering_is_lossless_and_matches_legacy_values() {
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

    let canonical = lower_legacy_grammar(&snapshot).unwrap();
    let legacy = mech_syntax::parse_grammar(text).unwrap();
    assert_eq!(grammar_shape(&canonical), grammar_shape(&legacy));
    assert_eq!(token_text(&canonical.rules[0].name.name), "rule");
    assert_eq!(expression_shape(&canonical.rules[0].expr), "terminal(ab)");
    assert_eq!(token_text(&canonical.rules[2].name.name), "e\u{301}");
    assert_eq!(
        expression_shape(&canonical.rules[3].expr),
        "terminal(a\u{00a0}\u{2009}b)"
    );
}

#[test]
fn grammar_literals_respect_graphemes_across_piece_boundaries() {
    let text = "rule := \"e\u{301}b\u{2764}\u{fe0f}\";";
    let snapshot = parse_canonical_grammar(
        piece_source(&["rule := \"", "e", "\u{301}", "b", "\u{2764}", "\u{fe0f}", "\";"]),
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

    let canonical = lower_legacy_grammar(&snapshot).unwrap();
    let legacy = mech_syntax::parse_grammar(text).unwrap();
    assert_eq!(grammar_shape(&canonical), grammar_shape(&legacy));
    assert_eq!(
        expression_shape(&canonical.rules[0].expr),
        "terminal(e\u{301}b\u{2764}\u{fe0f})"
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
    assert!(lower_legacy_grammar(&snapshot).is_err());
    assert!(mech_syntax::parse_grammar(text).is_err());
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

    let canonical = lower_legacy_grammar(&snapshot).unwrap();
    let legacy = mech_syntax::parse_grammar(text).unwrap();
    assert_eq!(grammar_shape(&canonical), grammar_shape(&legacy));
    assert_eq!(
        expression_shape(&canonical.rules[0].expr),
        "terminal(a\"\u{301}b)"
    );

    let clustered = snapshot
        .syntax()
        .tokens()
        .into_iter()
        .find(|token| token.text().as_deref() == Ok("\"\u{301}"))
        .expect("clustered quote content token");
    assert_eq!(clustered.kind(), SyntaxKind::Any);
}

#[test]
fn dispatcher_supports_only_the_two_phase_2a_pairs() {
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
    for (implementation, root) in [
        (ParserImplementation::Canonical, ParseRoot::Document),
        (ParserImplementation::Prototype, ParseRoot::Grammar),
    ] {
        let error = parse_syntax(source(""), root, implementation, config).unwrap_err();
        assert_eq!(
            error,
            ParseRequestError::Unsupported {
                implementation,
                root,
            }
        );
    }
}
