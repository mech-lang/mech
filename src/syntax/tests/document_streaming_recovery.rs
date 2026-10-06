use mech_syntax::document::*;
#[path = "support/document_stream.rs"]
mod support;
use support::*;

#[test]
fn finite_previews_recover_without_closing_live_alternatives() {
    for (prefix, suffix) in [
        ("x := \"hello", " world\"\n"),
        ("x := [1,", "2]\n"),
        ("x := {a:", "1, b: 2}\n"),
        ("```mech\nx := 1\n", "```\n"),
        ("text {x +", " 1}\n"),
    ] {
        let mut stream = DocumentStream::new(DocumentId(826), ParseConfig::default());
        append(&mut stream, prefix, 31);
        let before = stream.work();
        let preview = stream.preview();
        assert_eq!(preview.identity.kind, StreamInterpretation::FinitePreview);
        equivalent(&preview.snapshot, prefix);
        assert_eq!(stream.work().parser_work, before.parser_work);
        assert!(stream.work().preview_work > before.preview_work);
        let work = stream.work();
        assert!(std::sync::Arc::ptr_eq(
            &preview.snapshot,
            &stream.preview().snapshot
        ));
        assert_eq!(stream.work(), work);
        append(&mut stream, suffix, 31);
        let final_snapshot = finish(&mut stream, 31);
        equivalent(&final_snapshot, &(prefix.to_owned() + suffix));
        assert!(
            preview
                .snapshot
                .nodes
                .nodes()
                .all(|(id, _)| !final_snapshot.nodes.contains_node(id)),
            "preview and live node IDs must not alias"
        );
    }
}
#[test]
fn established_errors_are_published_before_final_eof() {
    let mut stream = DocumentStream::new(DocumentId(826), ParseConfig::default());
    let update = append(&mut stream, "x := [1, +, 2]\ny := 3\nz := ", 127);
    assert!(
        update.view.diagnostic_count() > 0,
        "an established earlier error must not wait for document EOF"
    );
    let diagnostic = update.view.diagnostic(0).unwrap();
    let next = append(&mut stream, "4\n", 127);
    assert_eq!(next.view.diagnostic(0).unwrap().id, diagnostic.id);
    let final_snapshot = finish(&mut stream, 127);
    equivalent(&final_snapshot, "x := [1, +, 2]\ny := 3\nz := 4\n");
    assert!(!final_snapshot.is_strictly_clean());
}
#[test]
fn malformed_documents_and_physical_closers_survive_every_cut() {
    for text in [
        "x := [1, +, 2]\ny := 3\n",
        "x := {a: , b: 2}\ny := 3\n",
        "```mech\nx := [1, +\n```\ny := 3\n",
        "text {x +}\n",
        "x := \"unterminated é👩‍💻",
        "x := foo(1, [2, +, 3], 4)\n",
    ] {
        for split in text
            .char_indices()
            .map(|(at, _)| at)
            .chain(core::iter::once(text.len()))
        {
            let mut stream = DocumentStream::new(DocumentId(826), ParseConfig::default());
            append(&mut stream, &text[..split], 127);
            append(&mut stream, &text[split..], 127);
            equivalent(&finish(&mut stream, 127), text);
        }
    }
}
