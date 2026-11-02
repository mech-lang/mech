//! Adjacent selector chains retain an evaluated primary exactly once.
use mech_syntax::document::*;
#[path = "support/document_stream.rs"]
mod support;

fn parse(text: &str) -> SyntaxSnapshot {
    parse_canonical_document(
        TextSnapshot::new(DocumentId(0x504f5354), Revision(0), text).unwrap(),
        ParseConfig::default(),
    )
}
fn nodes(node: &SyntaxNode, kind: SyntaxKind, output: &mut Vec<SyntaxNode>) {
    if node.kind() == kind {
        output.push(node.clone());
    }
    for child in node.children() {
        nodes(&child, kind, output);
    }
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
fn calls_and_parenthesized_primaries_keep_one_base_and_existing_selectors() {
    for text in [
        "result := f()[1u64]\n",
        "result := f(x)[1u64].field[2u64]\n",
        "result := stats/sum/column(counts)[1u64]\n",
        "result := (f(x))[1u64]\n",
        "result := (\"é👩🏽‍💻Z\")[2u64..=3u64]\n",
        "result := ([1u64 2u64])[2u64]\n",
        "result := ({label: \"ok\"}).label[1u64]\n",
        "result := f()..=3u64\n",
        "Identity(x<u64>) => (out<u64>) := out := x.\n",
    ] {
        let parsed = clean(text);
        if text.contains("[1u64].field") {
            let mut slices = Vec::new();
            nodes(&parsed.syntax(), SyntaxKind::Slice, &mut slices);
            let slice = slices.into_iter().find_map(SliceSyntax::cast).unwrap();
            assert!(matches!(
                slice.stem(),
                Some(SliceStemSyntax::Value(FactorValueSyntax::Call(_)))
            ));
            assert_eq!(slice.subscripts().unwrap().items().len(), 3);
            let mut calls = Vec::new();
            nodes(slice.syntax(), SyntaxKind::FunctionCall, &mut calls);
            assert_eq!(calls.len(), 1);
        }
    }
}

#[test]
fn expression_base_selection_is_not_an_assignment_target() {
    for text in [
        "f()[1u64] = 9u64\n",
        "([1u64 2u64])[1u64] = 9u64\n",
        "({field: 1u64}).field = 9u64\n",
    ] {
        // A document may retain non-executable text as an ordinary paragraph.
        // Its cleanliness does not turn that text into a writable SliceRef.
        let parsed = parse(text);
        let mut assignments = Vec::new();
        nodes(
            &parsed.syntax(),
            SyntaxKind::VariableAssign,
            &mut assignments,
        );
        assert!(assignments.is_empty(), "{text:?}");
    }
    let parsed = clean("~local := [1u64 2u64]\nlocal[1u64] = 9u64\n");
    let mut assignments = Vec::new();
    nodes(
        &parsed.syntax(),
        SyntaxKind::VariableAssign,
        &mut assignments,
    );
    assert_eq!(assignments.len(), 1);
}

#[test]
fn final_call_and_parenthesized_values_leave_the_function_period_unconsumed() {
    for text in [
        "FinalCall(x<u64>) => (out<u64>) := out := f(x).\n",
        "FinalParen(x<u64>) => (out<u64>) := out := (x + 1u64).\n",
        "FinalCall(x<u64>) => (out<u64>) := out := f(x).\n\nNext(x<u64>) => (out<u64>) := out := x.\n",
        "FinalParen(x<u64>) => (out<u64>) := out := (f(x)).\n\nNext(x<u64>) => (out<u64>) := out := x.\n",
    ] {
        let parsed = clean(text);
        let mut slices = Vec::new();
        nodes(&parsed.syntax(), SyntaxKind::Slice, &mut slices);
        assert!(slices.is_empty(), "{text:?}");
        let mut definitions = Vec::new();
        nodes(
            &parsed.syntax(),
            SyntaxKind::FunctionDefine,
            &mut definitions,
        );
        assert_eq!(
            definitions.len(),
            if text.contains("Next(") { 2 } else { 1 }
        );
    }
}

#[test]
fn incomplete_call_result_subscripts_keep_specific_recovery() {
    for (text, code) in [
        ("result := f()[1u64\n", "syntax/missing-delimiter"),
        ("result := f()[1u64,]\n", "syntax/missing-subscript-value"),
    ] {
        let parsed = parse(text);
        assert!(!parsed.is_strictly_clean(), "{text:?}");
        assert!(
            parsed.diagnostics.iter().any(|d| d.code.0 == code),
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
fn postfix_selection_batch_and_stream_owners_agree_at_every_scalar_cut() {
    for text in [
        "result := f()[1u64].field[2u64]\n",
        "result := (\"é👩🏽‍💻Z\")[2u64..=3u64]\n",
        "result := f()..=3u64\n",
        "result := (f())..=3u64\n",
        "FinalCall(x<u64>) => (out<u64>) := out := f(x).\n",
        "FinalParen(x<u64>) => (out<u64>) := out := (f(x)).\n",
        "FinalCall(x<u64>) => (out<u64>) := out := f(x).",
        "FinalParen(x<u64>) => (out<u64>) := out := (x + 1u64).",
        "result := f()..",
        "result := (f())..",
        "result := f()[1u64,]\n",
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
                    DocumentStream::new(DocumentId(0x504f5354), ParseConfig::default());
                support::append(&mut stream, &text[..cut], allowance);
                support::append(&mut stream, &text[cut..], allowance);
                support::equivalent_to(&support::finish(&mut stream, allowance), text, &expected);
            }
        }
    }
}
