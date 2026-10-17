//! Delimited comprehensions compose as ordinary formula operands.
use mech_syntax::document::*;
#[path = "support/document_stream.rs"]
mod support;

const FSM: &str = "#CapturedText(value<string>) => <bool>\n  | :Start\n  | :Later.\n#CapturedText(value) -> :Start\n  :Start ~> :Later\n  :Later => \"é\" ∈ {part | part <- value}.\n#CapturedText(signal<string>)\n";

fn parse(text: &str) -> SyntaxSnapshot {
    parse_canonical_document(
        TextSnapshot::new(DocumentId(0x5345544f), Revision(1), text).unwrap(),
        ParseConfig::default(),
    )
}
fn count(node: &SyntaxNode, kind: SyntaxKind) -> usize {
    usize::from(node.kind() == kind)
        + node
            .children()
            .map(|node| count(&node, kind))
            .sum::<usize>()
}
fn factor_sets(node: &SyntaxNode) -> usize {
    usize::from(FactorSyntax::cast(node.clone()).is_some_and(|factor| {
        matches!(factor.value(), Some(FactorValueSyntax::SetComprehension(_)))
    })) + node
        .children()
        .map(|node| factor_sets(&node))
        .sum::<usize>()
}
fn clean(text: &str) -> SyntaxSnapshot {
    let parsed = parse(text);
    assert!(
        parsed.is_strictly_clean(),
        "{text:?}: {:?}",
        parsed.diagnostics
    );
    validate_lossless(&parsed.root, &parsed.source).unwrap();
    assert_eq!(
        reconstruct_source(&parsed.root, &parsed.source).unwrap(),
        text
    );
    parsed
}

#[test]
fn membership_rhs_and_both_set_operator_operands_have_typed_factor_views() {
    for text in [
        "\"é\" ∈ {part | part <- signal<string>}\n",
        "{part | part <- left<string>} ∪ {part | part <- right<string>}\n",
        "!(\"é\" ∈ {part | part <- signal<string>})\n",
        "keep := \"é\" ∈ {part |\n  part <- signal<string>\n}\n",
        FSM,
    ] {
        let parsed = clean(text);
        assert!(factor_sets(&parsed.syntax()) >= 1, "{text:?}");
        assert_eq!(
            count(&parsed.syntax(), SyntaxKind::Paragraph),
            0,
            "{text:?}"
        );
    }
    // An argument that starts with a comprehension retains its existing
    // expression-body form; it need not be wrapped in a formula factor.
    let argument = clean("has({part | part <- signal<string>})\n");
    assert_eq!(count(&argument.syntax(), SyntaxKind::SetComprehension), 1);
}

#[test]
fn existing_brace_forms_and_matrix_comprehension_operands_are_preserved() {
    for (text, kind) in [
        ("x := {a: 1u64}\n", SyntaxKind::Record),
        ("x := {1u64: 2u64}\n", SyntaxKind::Map),
        ("x := {1u64, 2u64}\n", SyntaxKind::Set),
        ("x := {}\n", SyntaxKind::EmptySet),
        ("x := {:}\n", SyntaxKind::EmptyMap),
        (
            "1u64 + [n | n <- [1u64 2u64]]\n",
            SyntaxKind::MatrixComprehension,
        ),
    ] {
        let parsed = clean(text);
        assert!(count(&parsed.syntax(), kind) >= 1, "{text:?}");
        assert_eq!(
            count(&parsed.syntax(), SyntaxKind::SetComprehension),
            0,
            "{text:?}"
        );
    }
}

#[test]
fn malformed_rhs_comprehensions_keep_their_owned_recovery() {
    for (text, code) in [
        (
            "\"é\" ∈ {part | part <-}\n",
            "syntax/missing-generator-source",
        ),
        (
            "\"é\" ∈ {part | part <- signal,}\n",
            "syntax/missing-comprehension-qualifier",
        ),
        (
            "\"é\" ∈ {part | part <- signal\n",
            "syntax/missing-delimiter",
        ),
    ] {
        let parsed = parse(text);
        assert!(!parsed.is_strictly_clean(), "{text:?}");
        assert!(
            parsed.diagnostics.iter().any(|d| d.code.0 == code),
            "{text:?}: {:?}",
            parsed.diagnostics
        );
        validate_lossless(&parsed.root, &parsed.source).unwrap();
        assert_eq!(
            reconstruct_source(&parsed.root, &parsed.source).unwrap(),
            text
        );
    }
}

#[test]
fn batch_and_stream_agree_at_every_operand_boundary() {
    for text in [
        "\"é\" ∈ {part | part <- value}\n",
        "{a | a <- left} ∪ {b | b <- right}\n",
        "\"é\" ∈ {part | part <-}\n",
        FSM,
    ] {
        let expected = parse(text);
        support::equivalent(&expected, text);
        for allowance in [1, 17] {
            for cut in text
                .char_indices()
                .map(|(at, _)| at)
                .chain(core::iter::once(text.len()))
            {
                let mut stream =
                    DocumentStream::new(DocumentId(0x5345544f), ParseConfig::default());
                support::append(&mut stream, &text[..cut], allowance);
                support::append(&mut stream, &text[cut..], allowance);
                let actual = support::finish(&mut stream, allowance);
                support::equivalent_to(&actual, text, &expected);
            }
        }
    }
}

#[test]
fn operand_nesting_and_work_limits_remain_lossless_under_partitioning() {
    let text = "\"é\" ∈ {part | part <- value}\n";
    let mut configs = Vec::new();
    for fuel in [0, 1, 17, 64, 256] {
        let mut config = ParseConfig::default();
        config.limits.fuel = fuel;
        configs.push(config);
    }
    for max_nesting in [0, 1, 2] {
        let mut config = ParseConfig::default();
        config.limits.max_nesting = max_nesting;
        configs.push(config);
    }
    for config in configs {
        for allowance in [1, 17] {
            let limits = StreamLimits {
                max_parser_work: config.limits.fuel,
                ..StreamLimits::default()
            };
            let mut stream = DocumentStream::with_limits(DocumentId(0x5345544f), config, limits);
            let mut update = stream.append(text, allowance).unwrap();
            let mut calls = 0;
            loop {
                assert!(calls < 100_000);
                assert!(update.work.parser_work <= limits.max_parser_work);
                match update.progress {
                    StreamProgress::NeedsProcessing => update = stream.advance(allowance),
                    StreamProgress::NeedInput => update = stream.finish(allowance),
                    StreamProgress::Finished | StreamProgress::Limited => break,
                    StreamProgress::Cancelled => panic!("uncancelled operand parse was cancelled"),
                }
                calls += 1;
            }
            let parsed = stream.materialize().unwrap();
            validate_lossless(&parsed.root, &parsed.source).unwrap();
            assert_eq!(
                reconstruct_source(&parsed.root, &parsed.source).unwrap(),
                text
            );
            assert!(parsed.stats.parser_steps <= config.limits.fuel);
            assert!(parsed.stats.events_emitted <= u64::from(config.limits.max_events));
        }
    }
}
