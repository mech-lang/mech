use alloc::string::String;

use crate::document::{
  Diagnostic, DiagnosticAnchor, DiagnosticCode, DiagnosticPhase, DiagnosticTags, ExpectedSyntax,
  FoundSyntax, NodeFlags, ParserContextId, RecoveryAction, RuleId, Severity, SyntaxKind, TextRange,
  TokenFlags,
};

use super::terminal::{is_newline_start, token_kind_for_char};
use super::{CompletedMarker, Parser};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseFailure {
  pub context: ParserContextId,
  pub range: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Attempt<T> {
  NoMatch,
  Match(T),
  CommittedFailure(ParseFailure),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryClass {
  MechItem,
  Paragraph,
  Fence,
}

pub(crate) fn skip_error(
  parser: &mut Parser<'_>,
  class: RecoveryClass,
  code: &str,
  message: &str,
) -> Option<CompletedMarker> {
  let start = parser.offset();
  let marker = parser.start();
  let mut recovered = 0_u32;
  while !parser.is_eof() && recovered < parser.config().limits.max_recovery_bytes {
    if should_stop(parser, class, start) {
      break;
    }
    let Some((character, range)) = parser.bump_char_raw() else {
      break;
    };
    recovered = recovered.saturating_add(range.len().0);
    parser.token_with_flags(
      token_kind_for_char(character),
      range,
      TokenFlags::ERROR,
    );
  }
  if recovered >= parser.config().limits.max_recovery_bytes
    && !parser.is_eof()
    && !should_stop(parser, class, start)
  {
    parser.halt();
  }
  if parser.offset() == start {
    marker.abandon(parser);
    return None;
  }
  parser.stats_mut().recovery_bytes = parser
    .stats()
    .recovery_bytes
    .saturating_add(u64::from(recovered));
  let error = marker.complete_with_flags(parser, SyntaxKind::Error, NodeFlags::ERROR);
  let range = TextRange::new(start, parser.offset());
  let found = parser
    .source()
    .text(range)
    .ok()
    .map(|text| FoundSyntax {
      kind: Some(SyntaxKind::Unknown),
      text: Some(text),
    });
  let diagnostic = Diagnostic {
    id: parser.next_diagnostic_id(),
    code: DiagnosticCode::from(code),
    phase: DiagnosticPhase::Syntax,
    severity: Severity::Error,
    rule: parser.current_rule(),
    context: parser.current_context(),
    primary: DiagnosticAnchor::Absolute {
      revision: parser.source().revision(),
      range,
    },
    labels: alloc::vec![],
    expected: alloc::vec![],
    found,
    fixes: alloc::vec![],
    related: alloc::vec![],
    recovery: Some(RecoveryAction::Skip { range }),
    tags: DiagnosticTags::NONE,
    message: String::from(message),
  };
  parser.push_diagnostic(
    diagnostic,
    Some(error.position()),
    TextRange::new(crate::document::TextSize::ZERO, range.len()),
  );
  Some(error)
}

pub(crate) fn insert_missing(
  parser: &mut Parser<'_>,
  code: &str,
  message: &str,
  expected: ExpectedSyntax,
  token: Option<SyntaxKind>,
) -> CompletedMarker {
  let at = parser.offset();
  let marker = parser.start();
  if let Some(token) = token {
    parser.missing_token(token);
  }
  let missing =
    marker.complete_with_flags(parser, SyntaxKind::Missing, NodeFlags::MISSING);
  let range = TextRange::empty(at);
  let diagnostic = Diagnostic {
    id: parser.next_diagnostic_id(),
    code: DiagnosticCode::from(code),
    phase: DiagnosticPhase::Syntax,
    severity: Severity::Error,
    rule: parser.current_rule(),
    context: parser.current_context(),
    primary: DiagnosticAnchor::Absolute {
      revision: parser.source().revision(),
      range,
    },
    labels: alloc::vec![],
    expected: alloc::vec![expected.clone()],
    found: Some(parser.found_syntax()),
    fixes: alloc::vec![],
    related: alloc::vec![],
    recovery: Some(RecoveryAction::Insert {
      syntax: expected,
      at,
    }),
    tags: DiagnosticTags::NONE,
    message: String::from(message),
  };
  parser.push_diagnostic(
    diagnostic,
    Some(missing.position()),
    TextRange::empty(crate::document::TextSize::ZERO),
  );
  missing
}

pub(crate) fn abandon_error(
  parser: &mut Parser<'_>,
  class: RecoveryClass,
  target: RuleId,
  code: &str,
  message: &str,
) -> ParseFailure {
  let start = parser.offset();
  let _ = skip_error(parser, class, code, message);
  let range = TextRange::new(start, parser.offset());
  let at = parser.offset();
  if let Some(diagnostic) = parser.last_diagnostic_mut() {
    diagnostic.recovery = Some(RecoveryAction::Abandon {
      rule: target,
      at,
    });
  }
  ParseFailure {
    context: parser
      .current_context()
      .expect("abandon recovery requires a parser context"),
    range,
  }
}

fn should_stop(
  parser: &Parser<'_>,
  class: RecoveryClass,
  start: crate::document::TextSize,
) -> bool {
  if parser.offset() == start {
    return false;
  }
  match class {
    RecoveryClass::MechItem => {
      is_newline_start(parser.cursor())
        || parser.cursor().starts_with(";")
        || parser.is_strong_document_boundary()
    }
    RecoveryClass::Paragraph => {
      is_newline_start(parser.cursor()) || parser.is_context_fence_start()
    }
    RecoveryClass::Fence => false,
  }
}

pub(crate) fn nesting_limit(parser: &mut Parser<'_>) {
  let start = parser.offset();
  let marker = parser.start();
  let mut recovered = 0_u32;
  let mut nested = 0_u32;
  while !parser.is_eof() && recovered < parser.config().limits.max_recovery_bytes {
    if nested == 0
      && (parser.cursor().starts_with(")")
        || parser.cursor().starts_with(";")
        || parser.cursor().starts_with("--")
        || parser.cursor().starts_with("//")
        || is_newline_start(parser.cursor())
        || parser.is_strong_document_boundary())
    {
      break;
    }
    let Some((character, range)) = parser.bump_char_raw() else {
      break;
    };
    if character == '(' {
      nested = nested.saturating_add(1);
    } else if character == ')' {
      nested = nested.saturating_sub(1);
    }
    recovered = recovered.saturating_add(range.len().0);
    parser.token_with_flags(
      token_kind_for_char(character),
      range,
      TokenFlags::ERROR,
    );
  }
  if recovered >= parser.config().limits.max_recovery_bytes
    && !parser.is_eof()
  {
    parser.halt();
  }
  if start == parser.offset() {
    marker.abandon(parser);
    let _ = insert_missing(
      parser,
      "syntax/nesting-limit",
      "syntax nesting limit reached",
      ExpectedSyntax::Production(String::from("expression")),
      None,
    );
    return;
  }
  parser.stats_mut().recovery_bytes = parser
    .stats()
    .recovery_bytes
    .saturating_add(u64::from(recovered));
  let error = marker.complete_with_flags(parser, SyntaxKind::Error, NodeFlags::ERROR);
  let range = TextRange::new(start, parser.offset());
  let found = parser
    .source()
    .text(range)
    .ok()
    .map(|text| FoundSyntax {
      kind: Some(SyntaxKind::Unknown),
      text: Some(text),
    });
  let diagnostic = Diagnostic {
    id: parser.next_diagnostic_id(),
    code: DiagnosticCode::syntax("nesting-limit"),
    phase: DiagnosticPhase::Syntax,
    severity: Severity::Error,
    rule: parser.current_rule(),
    context: parser.current_context(),
    primary: DiagnosticAnchor::Absolute {
      revision: parser.source().revision(),
      range,
    },
    labels: alloc::vec![],
    expected: alloc::vec![],
    found,
    fixes: alloc::vec![],
    related: alloc::vec![],
    recovery: Some(RecoveryAction::Skip { range }),
    tags: DiagnosticTags::NONE,
    message: String::from("syntax nesting limit reached"),
  };
  parser.push_diagnostic(
    diagnostic,
    Some(error.position()),
    TextRange::new(crate::document::TextSize::ZERO, range.len()),
  );
}
