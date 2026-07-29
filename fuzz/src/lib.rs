use mech_syntax::document::{
    DocumentId, ParseConfig, ParseLimits, Revision, SyntaxSnapshot, TextSnapshot,
    compact_debug_tree, parse_document, reconstruct_source, validate_lossless,
};

pub fn bounded_config(source_len: usize) -> ParseConfig {
    ParseConfig {
        limits: ParseLimits {
            max_nesting: 64,
            max_diagnostics: 64,
            max_events: (source_len as u32).saturating_mul(32).saturating_add(1_024),
            max_recovery_bytes: (source_len as u32).saturating_add(1),
            fuel: (source_len as u64)
                .saturating_mul(128)
                .saturating_add(4_096),
        },
    }
}

pub fn parse_bounded(source: &str) -> SyntaxSnapshot {
    parse_document(
        TextSnapshot::new(DocumentId(1), Revision(0), source).unwrap(),
        bounded_config(source.len()),
    )
}

pub fn assert_snapshot(source: &str, snapshot: &SyntaxSnapshot) {
    validate_lossless(&snapshot.root, &snapshot.source).unwrap();
    assert_eq!(
        reconstruct_source(&snapshot.root, &snapshot.source).unwrap(),
        source
    );
    assert!(snapshot.stats.parser_steps <= bounded_config(source.len()).limits.fuel);
    assert!(
        snapshot.diagnostics.len() <= bounded_config(source.len()).limits.max_diagnostics as usize
    );
    for diagnostic in snapshot.diagnostics.iter() {
        let range = diagnostic
            .primary
            .resolve(snapshot.revision, &snapshot.nodes)
            .unwrap();
        assert!(range.end.0 <= source.len() as u32);
    }
}

pub fn normalized(snapshot: &SyntaxSnapshot) -> (String, Vec<String>) {
    let diagnostics = snapshot
        .diagnostics
        .iter()
        .map(|diagnostic| {
            format!(
                "{}|{:?}|{:?}|{:?}|{:?}",
                diagnostic.code.as_str(),
                diagnostic.rule,
                diagnostic
                    .primary
                    .resolve(snapshot.revision, &snapshot.nodes),
                diagnostic.expected,
                diagnostic.recovery
            )
        })
        .collect();
    (compact_debug_tree(&snapshot.syntax()), diagnostics)
}

pub fn boundaries(source: &str) -> Vec<usize> {
    source
        .char_indices()
        .map(|(offset, _)| offset)
        .chain(core::iter::once(source.len()))
        .collect()
}
