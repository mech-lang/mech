use mech_syntax::document::{
  DocumentId, ParseConfig, ParseLimits, Revision, TextSnapshot, parse_document,
  reconstruct_source, validate_lossless,
};

fn parse_with_limits(text: &str, limits: ParseLimits) -> mech_syntax::document::SyntaxSnapshot {
  parse_document(
    TextSnapshot::new(DocumentId(88), Revision(0), text).unwrap(),
    ParseConfig { limits },
  )
}

#[test]
fn nesting_limit_returns_a_complete_snapshot() {
  let text = "x := (((((((1\nlater prose\n";
  let snapshot = parse_with_limits(
    text,
    ParseLimits {
      max_nesting: 2,
      ..ParseLimits::default()
    },
  );
  validate_lossless(&snapshot.root, &snapshot.source).unwrap();
  assert_eq!(reconstruct_source(&snapshot.root, &snapshot.source).unwrap(), text);
  assert!(snapshot
    .diagnostics
    .iter()
    .any(|diagnostic| diagnostic.code.as_str() == "syntax/nesting-limit"));
}

#[test]
fn fuel_limit_consumes_remainder_and_never_panics() {
  let text = "x := 123456789\nlater prose\n";
  let snapshot = parse_with_limits(
    text,
    ParseLimits {
      fuel: 3,
      ..ParseLimits::default()
    },
  );
  validate_lossless(&snapshot.root, &snapshot.source).unwrap();
  assert_eq!(reconstruct_source(&snapshot.root, &snapshot.source).unwrap(), text);
  assert_eq!(snapshot.diagnostics.len(), 1);
  assert_eq!(
    snapshot.diagnostics.iter().next().unwrap().code.as_str(),
    "syntax/recovery-limit"
  );
}

#[test]
fn diagnostic_limit_suppresses_cascades_and_later_excess() {
  let text = "x :=\n1. Next\n--------\ny :=\n";
  let snapshot = parse_with_limits(
    text,
    ParseLimits {
      max_diagnostics: 1,
      ..ParseLimits::default()
    },
  );
  validate_lossless(&snapshot.root, &snapshot.source).unwrap();
  assert_eq!(snapshot.diagnostics.len(), 1);
}

#[test]
fn event_limit_produces_structural_resource_error() {
  let text = "ordinary prose with many tokens\nx := 1\n";
  let snapshot = parse_with_limits(
    text,
    ParseLimits {
      max_events: 8,
      ..ParseLimits::default()
    },
  );
  validate_lossless(&snapshot.root, &snapshot.source).unwrap();
  assert_eq!(reconstruct_source(&snapshot.root, &snapshot.source).unwrap(), text);
  assert!(snapshot
    .diagnostics
    .iter()
    .any(|diagnostic| diagnostic.code.as_str() == "syntax/recovery-limit"));
}

#[test]
fn recovery_byte_limit_abandons_to_resource_error_without_losing_bytes() {
  let text = "x := @@@@@@@@@@\nlater prose\n";
  let snapshot = parse_with_limits(
    text,
    ParseLimits {
      max_recovery_bytes: 2,
      ..ParseLimits::default()
    },
  );
  validate_lossless(&snapshot.root, &snapshot.source).unwrap();
  assert_eq!(reconstruct_source(&snapshot.root, &snapshot.source).unwrap(), text);
  assert!(snapshot
    .diagnostics
    .iter()
    .any(|diagnostic| diagnostic.code.as_str() == "syntax/recovery-limit"));
}
