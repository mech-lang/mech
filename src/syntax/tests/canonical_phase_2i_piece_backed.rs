use mech_syntax::document::parser::canonical::{
    CanonicalSourceRuleSnapshot, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DocumentId, ParseConfig, Revision, RuleId, SyntaxKind, TextRange, TextSnapshot, TokenFlags,
    compact_debug_tree, normalize_diagnostics,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(0x2c3), Revision(0), text).unwrap()
}

fn piece_source(parts: &[&str]) -> TextSnapshot {
    let mut snapshot = source("");
    for part in parts {
        snapshot = snapshot.append((*part).to_owned()).unwrap();
    }
    assert_eq!(snapshot.piece_count(), parts.len());
    snapshot
}

fn parse(source: TextSnapshot, rule: RuleId) -> CanonicalSourceRuleSnapshot {
    parse_canonical_phase_2i_rule_for_test(source, rule, ParseConfig::default()).unwrap()
}

fn tokens(
    parsed: &CanonicalSourceRuleSnapshot,
) -> Vec<(SyntaxKind, String, TextRange, TokenFlags)> {
    parsed
        .syntax()
        .tokens()
        .into_iter()
        .map(|token| {
            (
                token.kind(),
                token.text().unwrap(),
                token.range(),
                token.flags(),
            )
        })
        .collect()
}

#[test]
fn contiguous_and_piece_backed_recursive_sources_are_identical() {
    let cases: &[(RuleId, &[&str])] = &[
        (rules::KIND_ANNOTATION, &["<", "u", "8", ">"]),
        (rules::RANGE_EXPRESSION, &["1", ".", ".", "1", "0"]),
        (rules::EXPRESSION, &["1 ", "+", " 2 ", "*", " 3"]),
        (rules::VAR, &["@ct", "x/pa", "th"]),
        (rules::FUNCTION_CALL, &["fo", "o(", "1, x", ": 2", ")"]),
        (rules::SLICE, &["fo", "o[", "1", "]"]),
        (
            rules::MATRIX_COMPREHENSION,
            &["[x ", "| x ", "<-", " x", "s]"],
        ),
        (rules::SET_COMPREHENSION, &["{x ", "| x ", "<-", " x", "s}"]),
        (rules::MATRIX, &["[1 ", "2; ", "3 ", "4", "]"]),
        (rules::RECORD, &["{a", ": 1, ", "b: ", "2}"]),
        (rules::PATTERN_ARRAY, &["[head", ", .", ".., ", "tail]"]),
        (rules::FSM_PIPE, &["#ma", "chine ", "->", " :ne", "xt"]),
        (rules::INLINE_TABLE, &["|a", "<u", "8>|", "1", "|"]),
        (rules::EXPRESSION, &["x ", "?", " | * ", "=> 1"]),
        (rules::EXPRESSION, &["{x | x ", "<-", " xs}", " + 1"]),
        (rules::EXPRESSION, &["[x | x ", "<-", " xs]", " + 1"]),
        (rules::MATRIX, &["[\n", "1", "\n]"]),
        (rules::MATRIX, &["╭\n", "1", "\n╯"]),
        (rules::KIND_TABLE, &["|a", "|"]),
        (rules::KIND_MATRIX, &["[u8]", ":", "1,2"]),
        (rules::INLINE_TABLE_ROW, &["1 ", "2", "|"]),
        (rules::EXPRESSION, &["{a: 1, ", "2: 3}"]),
    ];

    for (rule, parts) in cases {
        let joined = parts.concat();
        let contiguous = parse(source(&joined), *rule);
        let piece_backed = parse(piece_source(parts), *rule);
        assert_eq!(contiguous.outcome, piece_backed.outcome, "{rule:?}");
        assert_eq!(contiguous.consumed, piece_backed.consumed, "{rule:?}");
        assert_eq!(
            contiguous.source.full_range().end - contiguous.consumed.end,
            piece_backed.source.full_range().end - piece_backed.consumed.end,
            "remaining range for {rule:?}"
        );
        assert_eq!(
            compact_debug_tree(&contiguous.syntax()),
            compact_debug_tree(&piece_backed.syntax()),
            "{rule:?}"
        );
        assert_eq!(tokens(&contiguous), tokens(&piece_backed), "{rule:?}");
        assert_eq!(
            normalize_diagnostics(
                &contiguous.diagnostics,
                contiguous.source.revision(),
                &contiguous.nodes,
            ),
            normalize_diagnostics(
                &piece_backed.diagnostics,
                piece_backed.source.revision(),
                &piece_backed.nodes,
            ),
            "{rule:?}"
        );
    }
}
