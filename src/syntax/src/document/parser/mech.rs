use alloc::string::String;

use crate::document::{
  DiagnosticAnchor, DiagnosticFix, DiagnosticLabel, ExpectedSyntax, FixApplicability, NodeFlags,
  SyntaxKind, TextEdit, TextRange,
};

use super::recovery::{
  Attempt, RecoveryClass, abandon_error, insert_missing, nesting_limit,
};
use super::terminal::{
  is_horizontal_space, is_identifier_continue, is_identifier_start, is_newline_start,
};
use super::{
  Cursor, Parser, canonical_rule_id, parser_context_id,
};

pub(crate) fn is_mech_item_start(cursor: &Cursor<'_>) -> bool {
  is_variable_definition_start(cursor) || is_comment_start(cursor)
}

pub(crate) fn parse_mech_item(parser: &mut Parser<'_>) -> Attempt<()> {
  if is_variable_definition_start(parser.cursor()) {
    return parser.with_rule(
      parser_context_id("prototype-mech-item"),
      None,
      |parser| {
        let item = parser.start();
        let outcome = parse_variable_definition(parser);
        parse_code_terminal(parser);
        item.complete_with_flags(parser, SyntaxKind::MechItem, NodeFlags::REPARSE_ROOT);
        outcome
      },
    );
  }
  if is_comment_start(parser.cursor()) {
    parse_comment_item(parser);
    return Attempt::Match(());
  }
  Attempt::NoMatch
}

fn is_variable_definition_start(cursor: &Cursor<'_>) -> bool {
  let mut lookahead = cursor.clone();
  consume_horizontal_lookahead(&mut lookahead);
  if lookahead.starts_with("~") {
    let _ = lookahead.bump_bytes(1);
  }
  if scan_identifier(&mut lookahead).is_none() {
    return false;
  }
  let _ = scan_kind_annotation(&mut lookahead);
  consume_whitespace_lookahead(&mut lookahead);
  lookahead.starts_with(":=")
}

fn is_comment_start(cursor: &Cursor<'_>) -> bool {
  let mut lookahead = cursor.clone();
  consume_horizontal_lookahead(&mut lookahead);
  lookahead.starts_with("--") || lookahead.starts_with("//")
}

pub(crate) fn parse_variable_definition(parser: &mut Parser<'_>) -> Attempt<()> {
  parser.with_rule(
    parser_context_id("prototype-variable-define"),
    None,
    |parser| {
      let definition = parser.start();
      let _ = parser.consume_horizontal_space();
      if parser.cursor().starts_with("~") {
        let _ = parser.bump_bytes_token(1, SyntaxKind::Tilde);
      }
      parse_identifier(parser);
      parse_kind_annotation(parser);
      parser.consume_syntax_whitespace();

      let operator = parser.start();
      let _ = parser.bump_bytes_token(1, SyntaxKind::Colon);
      let _ = parser.bump_bytes_token(1, SyntaxKind::Equal);
      operator.complete(parser, SyntaxKind::DefineOperator);
      parser.consume_syntax_whitespace();

      let mut outcome = Attempt::Match(());
      if at_expression_boundary(parser) {
        missing_expression(parser, None);
      } else {
        match parse_expression(parser) {
          Attempt::Match(()) => {}
          Attempt::NoMatch => {
            outcome = Attempt::CommittedFailure(abandon_error(
              parser,
              RecoveryClass::MechItem,
              ancestor_restart_rule(),
              "syntax/unexpected-token",
              "abandoned malformed variable definition at the containing section element",
            ));
          }
          Attempt::CommittedFailure(failure) => {
            outcome = Attempt::CommittedFailure(failure);
          }
        }
      }

      let _ = parser.consume_horizontal_space();
      if matches!(outcome, Attempt::Match(()))
        && !at_code_terminal(parser)
        && !parser.is_strong_document_boundary()
        && !parser.is_halted()
      {
        outcome = Attempt::CommittedFailure(abandon_error(
          parser,
          RecoveryClass::MechItem,
          ancestor_restart_rule(),
          "syntax/unexpected-token",
          "abandoned malformed variable definition at the containing section element",
        ));
      }
      definition.complete_with_flags(
        parser,
        SyntaxKind::VariableDefine,
        NodeFlags::REPARSE_ROOT,
      );
      outcome
    },
  )
}

fn parse_comment_item(parser: &mut Parser<'_>) {
  parser.with_rule(
    parser_context_id("prototype-comment"),
    None,
    |parser| {
      let item = parser.start();
      let _ = parser.consume_horizontal_space();
      parse_comment(parser);
      parse_code_terminal(parser);
      item.complete_with_flags(parser, SyntaxKind::MechItem, NodeFlags::REPARSE_ROOT);
    },
  );
}

pub(crate) fn parse_expression(parser: &mut Parser<'_>) -> Attempt<()> {
  let checkpoint = parser.checkpoint();
  parser.with_rule(
    parser_context_id("prototype-expression"),
    None,
    |parser| {
      let expression = parser.start();
      match parse_additive(parser) {
        Attempt::Match(()) => {
          expression.complete(parser, SyntaxKind::Expression);
          Attempt::Match(())
        }
        Attempt::NoMatch => {
          parser.rewind(checkpoint);
          Attempt::NoMatch
        }
        Attempt::CommittedFailure(failure) => {
          expression.complete(parser, SyntaxKind::Expression);
          Attempt::CommittedFailure(failure)
        }
      }
    },
  )
}

fn parse_additive(parser: &mut Parser<'_>) -> Attempt<()> {
  let checkpoint = parser.checkpoint();
  parser.with_rule(
    parser_context_id("prototype-l3"),
    None,
    |parser| {
      let additive = parser.start();
      match parse_factor(parser) {
        Attempt::Match(()) => {}
        Attempt::NoMatch => {
          parser.rewind(checkpoint);
          return Attempt::NoMatch;
        }
        Attempt::CommittedFailure(failure) => {
          additive.complete(parser, SyntaxKind::AdditiveExpression);
          return Attempt::CommittedFailure(failure);
        }
      }

      loop {
        let operator_checkpoint = parser.checkpoint();
        let _ = parser.consume_horizontal_space();
        if !parser.cursor().starts_with("+") {
          parser.rewind(operator_checkpoint);
          break;
        }
        let plus = parser
          .bump_bytes_token(1, SyntaxKind::Plus)
          .unwrap_or_else(|| TextRange::empty(parser.offset()));
        let _ = parser.consume_horizontal_space();
        if at_expression_boundary(parser) {
          missing_expression(parser, Some(plus));
          break;
        }
        match parse_factor(parser) {
          Attempt::Match(()) => {}
          Attempt::NoMatch => {
            let failure = abandon_error(
              parser,
              RecoveryClass::MechItem,
              ancestor_restart_rule(),
              "syntax/unexpected-token",
              "abandoned malformed additive expression at the containing section element",
            );
            additive.complete(parser, SyntaxKind::AdditiveExpression);
            return Attempt::CommittedFailure(failure);
          }
          Attempt::CommittedFailure(failure) => {
            additive.complete(parser, SyntaxKind::AdditiveExpression);
            return Attempt::CommittedFailure(failure);
          }
        }
      }
      additive.complete(parser, SyntaxKind::AdditiveExpression);
      Attempt::Match(())
    },
  )
}

fn parse_factor(parser: &mut Parser<'_>) -> Attempt<()> {
  if parser
    .cursor()
    .peek_char()
    .is_some_and(|character| character.is_numeric())
  {
    parse_integer(parser);
    return Attempt::Match(());
  }
  if parser.cursor().starts_with("(") {
    return parse_parenthetical(parser);
  }
  if parser
    .cursor()
    .peek_char()
    .is_some_and(is_identifier_start)
  {
    parse_identifier(parser);
    return Attempt::Match(());
  }
  Attempt::NoMatch
}

fn parse_integer(parser: &mut Parser<'_>) {
  parser.with_rule(
    parser_context_id("prototype-integer-literal"),
    None,
    |parser| {
      let literal = parser.start();
      let start = parser.offset();
      let mut previous_was_digit = false;
      while !parser.is_halted()
        && let Some(character) = parser.cursor().peek_char()
      {
        if character.is_numeric() {
          previous_was_digit = true;
          let _ = parser.bump_char_raw();
          continue;
        }
        if character == '_'
          && previous_was_digit
          && parser
            .cursor()
            .byte_at(1)
            .and_then(|_| {
              let mut lookahead = parser.cursor().clone();
              let _ = lookahead.bump_bytes(1);
              lookahead.peek_char()
            })
            .is_some_and(|next| next.is_numeric())
        {
          previous_was_digit = false;
          let _ = parser.bump_char_raw();
          continue;
        }
        break;
      }
      for suffix in [
        "u128", "i128", "u64", "i64", "u32", "i32", "f64", "f32", "u16", "i16", "u8",
        "i8",
      ] {
        if parser.cursor().starts_with(suffix) {
          for _ in suffix.bytes() {
            let _ = parser.bump_char_raw();
          }
          break;
        }
      }
      parser.token(
        SyntaxKind::IntegerToken,
        TextRange::new(start, parser.offset()),
      );
      literal.complete(parser, SyntaxKind::IntegerLiteral);
    },
  );
}

fn parse_identifier(parser: &mut Parser<'_>) {
  parser.with_rule(
    parser_context_id("prototype-identifier"),
    None,
    |parser| {
      let identifier = parser.start();
      let start = parser.offset();
      let _ = parser.bump_char_raw();
      while !parser.is_halted()
        && parser
        .cursor()
        .peek_char()
        .is_some_and(is_identifier_continue)
      {
        let _ = parser.bump_char_raw();
      }
      parser.token(
        SyntaxKind::IdentifierToken,
        TextRange::new(start, parser.offset()),
      );
      identifier.complete(parser, SyntaxKind::Identifier);
    },
  );
}

fn parse_kind_annotation(parser: &mut Parser<'_>) {
  if !parser.cursor().starts_with("<") {
    return;
  }
  parser.with_rule(
    parser_context_id("prototype-kind-annotation"),
    None,
    |parser| {
      let annotation = parser.start();
      let _ = parser.bump_bytes_token(1, SyntaxKind::LeftAngle);
      let content_start = parser.offset();
      let mut depth = 1_u32;
      while !parser.is_eof()
        && !parser.is_halted()
        && !is_newline_start(parser.cursor())
      {
        if parser.cursor().starts_with("<") {
          depth = depth.saturating_add(1);
        } else if parser.cursor().starts_with(">") {
          depth = depth.saturating_sub(1);
          if depth == 0 {
            break;
          }
        }
        let _ = parser.bump_char_raw();
      }
      if parser.offset() > content_start {
        parser.token(
          SyntaxKind::Text,
          TextRange::new(content_start, parser.offset()),
        );
      }
      if parser.cursor().starts_with(">") {
        let _ = parser.bump_bytes_token(1, SyntaxKind::RightAngle);
      } else {
        let _ = insert_missing(
          parser,
          "syntax/unclosed-kind-annotation",
          "missing `>` to close prototype kind annotation",
          ExpectedSyntax::Token(SyntaxKind::RightAngle),
          Some(SyntaxKind::RightAngle),
        );
      }
      annotation.complete(parser, SyntaxKind::KindAnnotation);
    },
  );
}

pub(crate) fn parse_parenthetical(parser: &mut Parser<'_>) -> Attempt<()> {
  parser.with_rule(
    parser_context_id("prototype-parenthetical-term"),
    None,
    |parser| {
      let parenthetical = parser.start();
      let opening = parser
        .bump_bytes_token(1, SyntaxKind::LeftParen)
        .unwrap_or_else(|| TextRange::empty(parser.offset()));
      let mut outcome = Attempt::Match(());
      if !parser.push_nesting() {
        nesting_limit(parser);
        let _ = parser.consume_horizontal_space();
        if parser.cursor().starts_with(")") {
          let _ = parser.bump_bytes_token(1, SyntaxKind::RightParen);
        }
      } else {
        let _ = parser.consume_horizontal_space();
        if at_expression_boundary(parser) || parser.cursor().starts_with(")") {
          missing_expression(parser, None);
        } else {
          match parse_additive(parser) {
            Attempt::Match(()) => {}
            Attempt::NoMatch => {
              outcome = Attempt::CommittedFailure(abandon_error(
                parser,
                RecoveryClass::MechItem,
                ancestor_restart_rule(),
                "syntax/unexpected-token",
                "abandoned malformed parenthetical term at the containing section element",
              ));
            }
            Attempt::CommittedFailure(failure) => {
              outcome = Attempt::CommittedFailure(failure);
            }
          }
        }
        let _ = parser.consume_horizontal_space();
        if parser.cursor().starts_with(")") {
          let _ = parser.bump_bytes_token(1, SyntaxKind::RightParen);
        } else if !parser.is_halted() {
          let _ = insert_missing(
            parser,
            "syntax/unclosed-delimiter",
            "missing `)` to close parenthetical expression",
            ExpectedSyntax::Token(SyntaxKind::RightParen),
            Some(SyntaxKind::RightParen),
          );
          let revision = parser.source().revision();
          let offset = parser.offset();
          if let Some(diagnostic) = parser.last_diagnostic_mut() {
            diagnostic.labels.push(DiagnosticLabel {
              anchor: DiagnosticAnchor::Absolute {
                revision,
                range: opening,
              },
              message: String::from("opening `(` is here"),
            });
            diagnostic.fixes.push(DiagnosticFix {
              title: String::from("Insert `)`"),
              applicability: FixApplicability::MachineApplicable,
              edits: alloc::vec![TextEdit::insert(offset, ")")],
            });
            diagnostic.primary = DiagnosticAnchor::Absolute {
              revision,
              range: TextRange::empty(offset),
            };
          }
        }
        parser.pop_nesting();
      }
      parenthetical.complete_with_flags(
        parser,
        SyntaxKind::ParentheticalExpression,
        NodeFlags::REPARSE_ROOT,
      );
      outcome
    },
  )
}

fn missing_expression(parser: &mut Parser<'_>, operator: Option<TextRange>) {
  if operator.is_none() {
    parser.with_rule(
      parser_context_id("prototype-expression"),
      None,
      |parser| emit_missing_expression(parser, None),
    );
  } else {
    emit_missing_expression(parser, operator);
  }
}

fn emit_missing_expression(parser: &mut Parser<'_>, operator: Option<TextRange>) {
  let _ = insert_missing(
    parser,
    "syntax/missing-expression",
    "expected an expression",
    ExpectedSyntax::Production(String::from("prototype-expression")),
    None,
  );
  let revision = parser.source().revision();
  let offset = parser.offset();
  if let Some(diagnostic) = parser.last_diagnostic_mut() {
    if let Some(operator) = operator {
      diagnostic.labels.push(DiagnosticLabel {
        anchor: DiagnosticAnchor::Absolute {
          revision,
          range: operator,
        },
        message: String::from("`+` requires a right operand"),
      });
    }
    diagnostic.fixes.push(DiagnosticFix {
      title: String::from("Insert an expression"),
      applicability: FixApplicability::HasPlaceholders,
      edits: alloc::vec![TextEdit::insert(offset, " _")],
    });
  }
}

fn parse_code_terminal(parser: &mut Parser<'_>) {
  parser.with_rule(
    parser_context_id("prototype-code-terminal"),
    None,
    |parser| {
      let _ = parser.consume_horizontal_space();
      if parser.cursor().starts_with(";") {
        let _ = parser.bump_bytes_token(1, SyntaxKind::Semicolon);
        let _ = parser.consume_horizontal_space();
      }
      if parser.cursor().starts_with("--") || parser.cursor().starts_with("//") {
        parse_comment(parser);
      }
      let _ = parser.consume_newline();

      loop {
        let checkpoint = parser.checkpoint();
        let _ = parser.consume_horizontal_space();
        if parser.consume_newline().is_none() {
          parser.rewind(checkpoint);
          break;
        }
      }
    },
  );
}

fn parse_comment(parser: &mut Parser<'_>) {
  let comment = parser.start();
  let start = parser.offset();
  while !parser.is_eof()
    && !parser.is_halted()
    && !is_newline_start(parser.cursor())
  {
    let _ = parser.bump_char_raw();
  }
  parser.token(
    SyntaxKind::CommentToken,
    TextRange::new(start, parser.offset()),
  );
  comment.complete(parser, SyntaxKind::Comment);
}

fn at_code_terminal(parser: &Parser<'_>) -> bool {
  let mut lookahead = parser.cursor().clone();
  consume_horizontal_lookahead(&mut lookahead);
  lookahead.is_eof()
    || is_newline_start(&lookahead)
    || lookahead.starts_with(";")
    || lookahead.starts_with("--")
    || lookahead.starts_with("//")
    || lookahead.starts_with(")")
}

fn at_expression_boundary(parser: &Parser<'_>) -> bool {
  parser.is_eof()
    || parser.is_strong_document_boundary()
    || is_newline_start(parser.cursor())
    || parser.cursor().starts_with(";")
    || parser.cursor().starts_with(")")
    || parser.cursor().starts_with("--")
    || parser.cursor().starts_with("//")
}

fn ancestor_restart_rule() -> crate::document::RuleId {
  canonical_rule_id("section-element")
    .expect("Phase 0 inventory must contain section-element")
}

fn consume_horizontal_lookahead(cursor: &mut Cursor<'_>) {
  while cursor.peek_char().is_some_and(is_horizontal_space) {
    let _ = cursor.bump_char();
  }
}

fn consume_whitespace_lookahead(cursor: &mut Cursor<'_>) {
  loop {
    consume_horizontal_lookahead(cursor);
    let consumed = match (cursor.byte(), cursor.byte_at(1)) {
      (Some(b'\r'), Some(b'\n')) => cursor.bump_bytes(2).is_some(),
      (Some(b'\r' | b'\n'), _) => cursor.bump_bytes(1).is_some(),
      _ => false,
    };
    if !consumed {
      break;
    }
  }
}

fn scan_identifier(cursor: &mut Cursor<'_>) -> Option<TextRange> {
  let start = cursor.offset();
  if !cursor.peek_char().is_some_and(is_identifier_start) {
    return None;
  }
  let _ = cursor.bump_char();
  while cursor.peek_char().is_some_and(is_identifier_continue) {
    let _ = cursor.bump_char();
  }
  Some(TextRange::new(start, cursor.offset()))
}

fn scan_kind_annotation(cursor: &mut Cursor<'_>) -> bool {
  if !cursor.starts_with("<") {
    return false;
  }
  let _ = cursor.bump_bytes(1);
  let mut depth = 1_u32;
  while !cursor.is_eof() && !is_newline_start(cursor) {
    if cursor.starts_with("<") {
      depth = depth.saturating_add(1);
    } else if cursor.starts_with(">") {
      depth = depth.saturating_sub(1);
      let _ = cursor.bump_bytes(1);
      if depth == 0 {
        return true;
      }
      continue;
    }
    let _ = cursor.bump_char();
  }
  false
}

#[cfg(test)]
mod tests {
  use crate::document::{DocumentId, IdGenerator, ParseConfig, Revision, TextSnapshot};

  use super::*;

  #[test]
  fn failed_speculation_restores_rule_depth() {
    let source = TextSnapshot::new(DocumentId(1), Revision(0), "@").unwrap();
    let mut ids = IdGenerator::new();
    let mut parser = Parser::new(
      &source,
      crate::document::parser::LexicalMode::PrototypeDocument,
      ParseConfig::default(),
      &mut ids,
    );
    let depth = parser.rule_depth();
    assert!(matches!(parse_expression(&mut parser), Attempt::NoMatch));
    assert_eq!(parser.rule_depth(), depth);
  }

  #[test]
  fn distinctive_prefix_returns_committed_failure_and_restores_rule_depth() {
    let source = TextSnapshot::new(DocumentId(1), Revision(0), "x := @").unwrap();
    let mut ids = IdGenerator::new();
    let mut parser = Parser::new(
      &source,
      crate::document::parser::LexicalMode::PrototypeDocument,
      ParseConfig::default(),
      &mut ids,
    );
    let depth = parser.rule_depth();
    assert!(matches!(
      parse_mech_item(&mut parser),
      Attempt::CommittedFailure(_)
    ));
    assert_eq!(parser.rule_depth(), depth);
  }
}
