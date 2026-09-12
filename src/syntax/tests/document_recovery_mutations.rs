use std::fs;
use std::path::{Path, PathBuf};

use mech_syntax::document::{
    DiagnosticAnchor, DocumentId, ParseConfig, ParseLimits, Revision, TextSize, TextSnapshot,
    compact_debug_tree, parse_document, reconstruct_source, validate_lossless,
};
use serde_json::{Value, json};

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/document")
}

fn fixture_files(directory: &str) -> Vec<PathBuf> {
    let mut paths = fs::read_dir(fixture_root().join(directory))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("mec"))
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn parse(text: &str) -> mech_syntax::document::SyntaxSnapshot {
    parse_document(
        TextSnapshot::new(DocumentId(200), Revision(0), text).unwrap(),
        ParseConfig {
            limits: ParseLimits {
                max_nesting: 64,
                max_diagnostics: 32,
                max_events: 100_000,
                max_recovery_bytes: 8_192,
                fuel: 500_000,
            },
        },
    )
}

fn assert_general_invariants(text: &str) {
    let snapshot = parse(text);
    validate_lossless(&snapshot.root, &snapshot.source).unwrap();
    assert_eq!(
        reconstruct_source(&snapshot.root, &snapshot.source).unwrap(),
        text
    );
    assert!(snapshot.stats.parser_steps <= 500_000);
    assert!(snapshot.diagnostics.len() <= 32);
    for diagnostic in snapshot.diagnostics.iter() {
        let range = diagnostic
            .primary
            .resolve(snapshot.revision, &snapshot.nodes)
            .expect("diagnostic must resolve");
        assert!(range.end.0 <= text.len() as u32);
        for fix in &diagnostic.fixes {
            for edit in &fix.edits {
                assert!(edit.delete.end.0 <= text.len() as u32);
            }
        }
    }
}

fn char_boundaries(text: &str) -> Vec<usize> {
    text.char_indices()
        .map(|(offset, _)| offset)
        .chain(core::iter::once(text.len()))
        .collect()
}

#[test]
fn accepted_fixture_mutations_preserve_totality_and_bytes() {
    let mut mutation_cases = 0_usize;
    for path in fixture_files("accepted") {
        let source = fs::read_to_string(&path).unwrap();
        assert_general_invariants(&source);
        let boundaries = char_boundaries(&source);

        for window in boundaries.windows(2) {
            let start = window[0];
            let end = window[1];

            let mut deleted = source.clone();
            deleted.replace_range(start..end, "");
            assert_general_invariants(&deleted);
            mutation_cases += 1;

            let original = &source[start..end];
            let mut duplicated = source.clone();
            duplicated.insert_str(end, original);
            assert_general_invariants(&duplicated);
            mutation_cases += 1;

            let mut punctuated = source.clone();
            punctuated.replace_range(start..end, "@");
            assert_general_invariants(&punctuated);
            mutation_cases += 1;
        }

        for boundary in boundaries {
            for insertion in ["\n", ":=", "💡"] {
                let mut mutated = source.clone();
                mutated.insert_str(boundary, insertion);
                assert_general_invariants(&mutated);
                mutation_cases += 1;
            }
        }

        for (needle, replacement) in [(")", ""), (" + 2", " +"), ("```", ""), ("~~~", "")] {
            if source.contains(needle) {
                assert_general_invariants(&source.replacen(needle, replacement, 1));
                mutation_cases += 1;
            }
        }
    }
    assert!(mutation_cases >= 500);
}

#[test]
fn malformed_and_promoted_regression_fixtures_remain_lossless() {
    let malformed = fixture_files("malformed");
    let regressions = fixture_files("promoted-regressions");
    assert_eq!(malformed.len(), 4);
    assert_eq!(regressions.len(), 8);
    for path in malformed.into_iter().chain(regressions) {
        let source = fs::read_to_string(path).unwrap();
        assert_general_invariants(&source);
    }
}

#[test]
fn compact_tree_fixture_is_stable() {
    let source = fs::read_to_string(fixture_root().join("malformed/missing-rhs.mec")).unwrap();
    let expected = fs::read_to_string(fixture_root().join("trees/missing-rhs.tree")).unwrap();
    let snapshot = parse(&source);
    assert_eq!(compact_debug_tree(&snapshot.syntax()), expected);
}

#[test]
fn structured_diagnostic_fixture_is_stable() {
    let source = fs::read_to_string(fixture_root().join("malformed/missing-rhs.mec")).unwrap();
    let expected: Value = serde_json::from_str(
        &fs::read_to_string(fixture_root().join("diagnostics/missing-rhs.json")).unwrap(),
    )
    .unwrap();
    let snapshot = parse(&source);
    let diagnostic = snapshot.diagnostics.iter().next().unwrap();
    let range = diagnostic
        .primary
        .resolve(snapshot.revision, &snapshot.nodes)
        .unwrap();
    let found = diagnostic
        .found
        .as_ref()
        .and_then(|found| found.kind)
        .map(|kind| format!("{kind:?}"))
        .unwrap_or_else(|| String::from("None"));
    let recovery = diagnostic
        .recovery
        .as_ref()
        .map(|recovery| format!("{recovery:?}"))
        .unwrap_or_else(|| String::from("None"));
    let actual = json!({
      "code": diagnostic.code.as_str(),
      "rule": diagnostic.rule.map(|rule| rule.0),
      "context": if diagnostic.context
        == Some(mech_syntax::document::parser::parser_context_id(
          "prototype-expression"
        ))
      {
        "prototype-expression"
      } else {
        "other"
      },
      "range": {
        "start": range.start.0,
        "end": range.end.0,
      },
      "expected": "prototype-expression",
      "found": found,
      "recovery": if recovery.starts_with("Insert") { "Insert" } else { "Other" },
    });
    assert_eq!(actual, expected);
}

#[test]
fn missing_and_error_ranges_remain_inside_revision() {
    for path in fixture_files("malformed") {
        let source = fs::read_to_string(path).unwrap();
        let snapshot = parse(&source);
        for diagnostic in snapshot.diagnostics.iter() {
            match diagnostic.primary {
                DiagnosticAnchor::Element { .. } | DiagnosticAnchor::Absolute { .. } => {
                    let range = diagnostic
                        .primary
                        .resolve(snapshot.revision, &snapshot.nodes)
                        .unwrap();
                    assert!(range.end <= TextSize(source.len() as u32));
                }
            }
        }
    }
}
