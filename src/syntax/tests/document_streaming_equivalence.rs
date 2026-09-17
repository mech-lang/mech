use mech_syntax::document::*;
fn same(left: &GreenNode, right: &GreenNode) {
    assert_eq!(
        (left.kind, left.flags, left.text_len, left.structural_hash),
        (
            right.kind,
            right.flags,
            right.text_len,
            right.structural_hash
        )
    );
    assert_eq!(left.children.len(), right.children.len());
    for (left, right) in left.children.iter().zip(right.children.iter()) {
        match (left, right) {
            (GreenElement::Node(left), GreenElement::Node(right)) => same(left, right),
            (GreenElement::Token(left), GreenElement::Token(right)) => assert_eq!(
                (left.kind, left.flags, left.text_len, left.text_hash),
                (right.kind, right.flags, right.text_len, right.text_hash)
            ),
            _ => panic!("canonical child shape"),
        }
    }
}
fn drain(stream: &mut DocumentStream, mut progress: StreamProgress) {
    let mut polls = 0;
    while progress == StreamProgress::NeedsProcessing {
        progress = stream.advance(13).progress;
        polls += 1;
        assert!(polls < 2_000_000);
    }
    assert_ne!(progress, StreamProgress::Limited);
}
#[test]
fn finalized_streams_match_one_shot_at_every_scalar_cut() {
    for text in [
        "",
        "hello world\n",
        "x := [1,2]\ny := x + 3\n",
        "x := \"á👩‍💻\"\n",
        "```mech\nx := 1\n```\n",
        "~~~text\na\r\nb\n~~~\n",
        "Title\n=====\ntext {x + 1}\n",
        "x := [1,+,2]\n",
        "```mech\nx := [1,\n",
        "╭◉╮\n(◉ ◯ ◉)\n",
    ] {
        for at in text
            .char_indices()
            .map(|(at, _)| at)
            .chain(core::iter::once(text.len()))
        {
            let mut stream = DocumentStream::new(DocumentId(826), ParseConfig::default());
            for chunk in [&text[..at], &text[at..]] {
                let update = stream.append(chunk, 13).unwrap();
                drain(&mut stream, update.progress);
            }
            let progress = stream.finish(13).progress;
            drain(&mut stream, progress);
            let actual = stream.materialize().unwrap();
            let expected = parse_canonical_document(actual.source.clone(), ParseConfig::default());
            same(&actual.root, &expected.root);
            assert_eq!(
                reconstruct_source(&actual.root, &actual.source).unwrap(),
                text
            );
            assert_eq!(
                normalize_diagnostics(&actual.diagnostics, actual.revision, &actual.nodes),
                normalize_diagnostics(&expected.diagnostics, expected.revision, &expected.nodes),
                "{text:?} cut {at}"
            );
        }
    }
}
