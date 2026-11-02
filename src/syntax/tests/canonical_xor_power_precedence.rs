//! Power leaves both carets to the documented logical XOR operator.
use mech_syntax::document::*;
#[path = "support/document_stream.rs"]
mod support;

fn parse(text: &str) -> SyntaxSnapshot {
    parse_canonical_document(
        TextSnapshot::new(DocumentId(0x584f5250), Revision(1), text).unwrap(),
        ParseConfig::default(),
    )
}

fn count(node: &SyntaxNode, kind: SyntaxKind) -> usize {
    usize::from(node.kind() == kind)
        + node
            .children()
            .map(|child| count(&child, kind))
            .sum::<usize>()
}

#[test]
fn ascii_and_unicode_xor_own_the_complete_operator_in_formulas() {
    for operator in ["^^", "⊕", "⊻"] {
        for text in [
            format!("left<bool> {operator} right<bool>\n"),
            format!("left<bool>{operator}right<bool>\n"),
        ] {
            let parsed = parse(&text);
            assert!(
                parsed.is_strictly_clean(),
                "{text}: {:?}",
                parsed.diagnostics
            );
            assert_eq!(count(&parsed.syntax(), SyntaxKind::XorOperation), 1);
            assert_eq!(count(&parsed.syntax(), SyntaxKind::PowerOperation), 0);
            support::equivalent(&parsed, &text);
        }
    }
    for (text, powers, xors) in [
        ("a ^ b ^ c\n", 2, 0),
        ("2u64 ^ 3u64 == 8u64 ^^ left<bool> && right<bool>\n", 1, 1),
    ] {
        let parsed = parse(text);
        assert!(
            parsed.is_strictly_clean(),
            "{text}: {:?}",
            parsed.diagnostics
        );
        assert_eq!(count(&parsed.syntax(), SyntaxKind::PowerOperation), powers);
        assert_eq!(count(&parsed.syntax(), SyntaxKind::XorOperation), xors);
    }
}

#[test]
fn xor_and_power_disambiguate_at_every_stream_boundary() {
    for text in ["a^^b\n", "a⊻b\n", "a^b^^c\n", "a ^^\n"] {
        let expected = parse(text);
        for allowance in [1, 17] {
            for cut in text
                .char_indices()
                .map(|(at, _)| at)
                .chain(core::iter::once(text.len()))
            {
                let mut stream =
                    DocumentStream::new(DocumentId(0x584f5250), ParseConfig::default());
                support::append(&mut stream, &text[..cut], allowance);
                support::append(&mut stream, &text[cut..], allowance);
                let actual = support::finish(&mut stream, allowance);
                support::equivalent_to(&actual, text, &expected);
            }
        }
    }
}

#[test]
fn xor_guard_remains_lossless_under_finite_work_and_nesting_limits() {
    let text = "left<bool> ^^ right<bool>\n";
    let mut configs = Vec::new();
    for fuel in [0, 1, 17, 64, 256] {
        let mut config = ParseConfig::default();
        config.limits.fuel = fuel;
        configs.push(config);
    }
    let mut nesting = ParseConfig::default();
    nesting.limits.max_nesting = 1;
    configs.push(nesting);
    for config in configs {
        for allowance in [1, 17] {
            let limits = StreamLimits {
                max_parser_work: config.limits.fuel,
                ..StreamLimits::default()
            };
            let mut stream = DocumentStream::with_limits(DocumentId(0x584f5250), config, limits);
            let mut update = stream.append(text, allowance).unwrap();
            let mut calls = 0;
            loop {
                assert!(calls < 100_000);
                assert!(update.work.parser_work <= limits.max_parser_work);
                match update.progress {
                    StreamProgress::NeedsProcessing => update = stream.advance(allowance),
                    StreamProgress::NeedInput => update = stream.finish(allowance),
                    StreamProgress::Finished | StreamProgress::Limited => break,
                    StreamProgress::Cancelled => panic!("uncancelled operator parse was cancelled"),
                }
                calls += 1;
            }
            let parsed = stream.materialize().unwrap();
            validate_lossless(&parsed.root, &parsed.source).unwrap();
            assert_eq!(
                reconstruct_source(&parsed.root, &parsed.source).unwrap(),
                text
            );
        }
    }
}
