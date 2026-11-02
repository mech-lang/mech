//! Multiline comprehensions preserve physical rows in ordinary matrices.
use mech_syntax::document::*;
#[path = "support/document_stream.rs"]
mod support;

fn parse(text: &str) -> SyntaxSnapshot {
    parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4d554c54), Revision(0), text).unwrap(),
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

fn clean(text: &str) -> SyntaxSnapshot {
    let parsed = parse(text);
    assert!(
        parsed.is_strictly_clean(),
        "{text:?}\n{:?}",
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
fn multiline_head_bar_and_tail_select_comprehensions() {
    for text in [
        "values := [\n  i | i <- 0u64..3u64]\n",
        "values := [i\n  | i <- 0u64..3u64]\n",
        "values := [i |\n  i <- 0u64..3u64\n]\n",
        "values := [\n  i\n  | i <- 0u64..3u64,\n    i > 0u64\n]\n",
        "values := [\r\n  i\r\n  | i ← 0u64..3u64,\r\n    i > 0u64\r\n]\r\n",
    ] {
        let parsed = clean(text);
        assert_eq!(
            count(&parsed.syntax(), SyntaxKind::MatrixComprehension),
            1,
            "{text:?}"
        );
        assert_eq!(
            count(&parsed.syntax(), SyntaxKind::Generator),
            1,
            "{text:?}"
        );
        assert_eq!(
            count(&parsed.syntax(), SyntaxKind::MatrixRow),
            0,
            "{text:?}"
        );
    }
}

#[test]
fn existing_range_string_slice_and_map_result_signature_compose() {
    let text = "histogram(s<string>, width<u64>) => (counts<{string:u64}>) :=\n  n := sequence/length(s)\n  words := [\n    s[(i + 1u64)..=(i + width)]\n    | i <- 0u64..n,\n      width > 0u64,\n      width <= n - i\n  ]\n  counts := map/histogram(words).\n\nfrequencies(h<{string:u64}>) => (output<string>) :=\n  output := \"example\".\n";
    let parsed = clean(text);
    for (kind, expected) in [
        (SyntaxKind::FunctionDefineStatements, 2),
        (SyntaxKind::KindMap, 2),
        (SyntaxKind::MatrixComprehension, 1),
        (SyntaxKind::RangeSubscript, 1),
        (SyntaxKind::Generator, 1),
    ] {
        assert_eq!(count(&parsed.syntax(), kind), expected, "{kind:?}");
    }
    // The single-line forms remain accepted independently of multiline layout.
    clean("values := [1u64 2u64]\n");
    clean("indices := [i | i <- 0u64..n]\n");
    clean("pieces := [s[(i + 1u64)..=(i + width)] | i <- 0u64..n]\n");
}

#[test]
fn shared_set_tail_accepts_line_breaks_without_changing_the_head() {
    let text = "values := {i |\n  i <- 0u64..3u64,\n  i > 0u64\n}\n";
    let parsed = clean(text);
    assert_eq!(count(&parsed.syntax(), SyntaxKind::SetComprehension), 1);
    assert_eq!(count(&parsed.syntax(), SyntaxKind::Generator), 1);
}

#[test]
fn failed_comprehension_probes_preserve_ordinary_matrix_rows() {
    for (text, rows) in [
        ("values := [1u64 2u64; 3u64 4u64]\n", 2),
        ("values := [\n  1u64 2u64\n  3u64 4u64\n]\n", 2),
        ("values := [1u64\n  2u64]\n", 2),
        ("values := [\n  1u64\n]\n", 1),
        ("values := [|1u64]\n", 1),
        ("values := [1u64\n  |2u64]\n", 2),
        ("values := [1u64\n  |2u64,]\n", 2),
        ("values := [\n  |1u64\n  |2u64\n]\n", 2),
        ("values := [1u64\n  │2u64]\n", 2),
        ("values := [1u64\n  |true]\n", 2),
        ("values := ╭\n  1u64\n╯\n", 1),
    ] {
        let parsed = clean(text);
        assert_eq!(
            count(&parsed.syntax(), SyntaxKind::MatrixComprehension),
            0,
            "{text:?}"
        );
        assert_eq!(
            count(&parsed.syntax(), SyntaxKind::MatrixRow),
            rows,
            "{text:?}"
        );
    }
    let framed = clean("values := ╭─\n│a│\n│1│\n");
    assert_eq!(count(&framed.syntax(), SyntaxKind::FancyTable), 1);
    assert_eq!(count(&framed.syntax(), SyntaxKind::MatrixComprehension), 0);
}

#[test]
fn malformed_multiline_tails_retain_comprehension_diagnostics() {
    for (text, code) in [
        (
            "values := [\n  i\n  | i <- 0u64..3u64\n",
            "syntax/missing-delimiter",
        ),
        (
            "values := [\n  i\n  | i <-\n]\n",
            "syntax/missing-generator-source",
        ),
        (
            "values := [\n  i\n  | i <- 0u64..3u64,\n]\n",
            "syntax/missing-comprehension-qualifier",
        ),
    ] {
        let parsed = parse(text);
        assert!(!parsed.is_strictly_clean(), "{text:?}");
        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code.0 == code),
            "{text:?}\n{:?}",
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
fn batch_and_stream_agree_across_multiline_selection_and_recovery_cuts() {
    for text in [
        "values := [\n  i\n  | i <- 0u64..3u64,\n    i > 0u64\n]\n",
        "pieces := [\r\n  s[i..=i]\r\n  | i ← 1u64..=3u64\r\n]\r\n",
        "values := [\n  1u64 2u64\n  3u64 4u64\n]\n",
        "values := [1u64\n  |2u64]\n",
        "values := [1u64\n  │2u64]\n",
        "values := [\n  i\n  | i <-\n]\n",
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
                    DocumentStream::new(DocumentId(0x4d554c54), ParseConfig::default());
                support::append(&mut stream, &text[..cut], allowance);
                support::append(&mut stream, &text[cut..], allowance);
                let actual = support::finish(&mut stream, allowance);
                support::equivalent_to(&actual, text, &expected);
            }
        }
    }
}

#[test]
fn nested_multiline_matrix_heads_are_retained_without_recursive_replay() {
    let depth = 32;
    let text = format!(
        "values := {}1u64{}\n",
        "[\n".repeat(depth),
        "\n]".repeat(depth)
    );
    let parsed = clean(&text);
    assert_eq!(count(&parsed.syntax(), SyntaxKind::Matrix), depth);
    assert_eq!(count(&parsed.syntax(), SyntaxKind::MatrixComprehension), 0);
}
