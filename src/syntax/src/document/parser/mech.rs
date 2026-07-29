use alloc::string::String;

use crate::document::{
  DiagnosticAnchor, DiagnosticFix, DiagnosticLabel, ExpectedSyntax, FixApplicability, NodeFlags,
  SyntaxKind, TextEdit, TextRange, TextSize,
};

use super::recovery::{Attempt, RecoveryClass, insert_missing, nesting_limit, skip_error};
use super::terminal::{
  is_horizontal_space, is_identifier_continue, is_identifier_start, is_newline_start,
};
use super::{Cursor, Parser};

pub(crate) fn is_mech_item_start(cursor: &Cursor<'_>) -> bool {
  is_variable_definition_start(cursor) || is_comment_start(cursor)
}

pub(crate) fn parse_mech_item(parser: &mut Parser<'_>) -> Attempt<()> {
  if is_variable_definition_start(parser.cursor()) {
    return parse_variable_definition(parser);
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
  if scan_identifier(&mut lookahead).is_none() {
    return false;
  }
  consume_whitespace_lookahead(&mut lookahead);
  lookahead.starts_with(":=")
}

fn is_comment_start(cursor: &Cursor<'_>) -> bool {
  let mut lookahead = cursor.clone();
  consume_horizontal_lookahead(&mut lookahead);
  lookahead.starts_with("--") || lookahead.starts_with("//")
}

fn parse_variable_definition(parser: &mut Parser<'_>) -> Attempt<()> {
  parser.enter("variable-define");
  let item = parser.start();
  let definition = parser.start();
  let _ = parser.consume_horizontal_space();
  parse_identifier(parser);
  parser.consume_syntax_whitespace();

  let operator = parser.start();
  let _ = parser.bump_bytes_token(1, SyntaxKind::Colon);
  let _ = parser.bump_bytes_token(1, SyntaxKind::Equal);
  operator.complete(parser, SyntaxKind::DefineOperator);
  parser.consume_syntax_whitespace();

  if at_expression_boundary(parser) {
    missing_expression(parser, None);
  } else {
    match parse_expression(parser) {
      Attempt::Match(()) => {}
      Attempt::NoMatch | Attempt::CommittedFailure(_) => {
        let _ = skip_error(
          parser,
          RecoveryClass::MechItem,
          "syntax/unexpected-token",
          "unexpected syntax in variable definition",
        );
      }
    }
  }

  let _ = parser.consume_horizontal_space();
  if !at_code_terminal(parser)
    && !parser.is_strong_document_boundary()
    && !parser.is_halted()
  {
    let _ = skip_error(
      parser,
      RecoveryClass::MechItem,
      "syntax/unexpected-token",
      "unexpected token after expression",
    );
  }
  definition.complete_with_flags(
    parser,
    SyntaxKind::VariableDefine,
    NodeFlags::REPARSE_ROOT,
  );
  parse_code_terminal(parser);
  item.complete_with_flags(parser, SyntaxKind::MechItem, NodeFlags::REPARSE_ROOT);
  parser.leave();
  Attempt::Match(())
}

fn parse_comment_item(parser: &mut Parser<'_>) {
  parser.enter("comment");
  let item = parser.start();
  let _ = parser.consume_horizontal_space();
  parse_comment(parser);
  parse_code_terminal(parser);
  item.complete_with_flags(parser, SyntaxKind::MechItem, NodeFlags::REPARSE_ROOT);
  parser.leave();
}

fn parse_expression(parser: &mut Parser<'_>) -> Attempt<()> {
  parser.enter("expression");
  let checkpoint = parser.checkpoint();
  let expression = parser.start();
  let result = parse_additive(parser);
  match result {
    Attempt::Match(()) => {
      expression.complete(parser, SyntaxKind::Expression);
      parser.leave();
      Attempt::Match(())
    }
    Attempt::NoMatch => {
      parser.rewind(checkpoint);
      Attempt::NoMatch
    }
    Attempt::CommittedFailure(failure) => {
      expression.complete(parser, SyntaxKind::Expression);
      parser.leave();
      Attempt::CommittedFailure(failure)
    }
  }
}

fn parse_additive(parser: &mut Parser<'_>) -> Attempt<()> {
  parser.enter("additive-expression");
  let checkpoint = parser.checkpoint();
  let additive = parser.start();
  if !parse_factor(parser) {
    parser.rewind(checkpoint);
    return Attempt::NoMatch;
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
    if !parse_factor(parser) {
      let _ = skip_error(
        parser,
        RecoveryClass::MechItem,
        "syntax/unexpected-token",
        "unexpected token where an expression was required",
      );
      break;
    }
  }
  additive.complete(parser, SyntaxKind::AdditiveExpression);
  parser.leave();
  Attempt::Match(())
}

fn parse_factor(parser: &mut Parser<'_>) -> bool {
  if parser
    .cursor()
    .peek_char()
    .is_some_and(|character| character.is_ascii_digit())
  {
    parse_integer(parser);
    return true;
  }
  if parser.cursor().starts_with("(") {
    parse_parenthetical(parser);
    return true;
  }
  if parser
    .cursor()
    .peek_char()
    .is_some_and(is_identifier_start)
  {
    parse_identifier(parser);
    return true;
  }
  false
}

fn parse_integer(parser: &mut Parser<'_>) {
  parser.enter("integer-literal");
  let literal = parser.start();
  let start = parser.offset();
  while parser
    .cursor()
    .peek_char()
    .is_some_and(|character| character.is_ascii_digit())
  {
    let _ = parser.bump_char_raw();
  }
  parser.token(
    SyntaxKind::IntegerToken,
    TextRange::new(start, parser.offset()),
  );
  literal.complete(parser, SyntaxKind::IntegerLiteral);
  parser.leave();
}

fn parse_identifier(parser: &mut Parser<'_>) {
  parser.enter("identifier");
  let identifier = parser.start();
  let start = parser.offset();
  let _ = parser.bump_char_raw();
  while parser
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
  parser.leave();
}

fn parse_parenthetical(parser: &mut Parser<'_>) {
  parser.enter("parenthetical-expression");
  let parenthetical = parser.start();
  let opening = parser
    .bump_bytes_token(1, SyntaxKind::LeftParen)
    .unwrap_or_else(|| TextRange::empty(parser.offset()));
  if !parser.push_nesting() {
    nesting_limit(parser);
  } else {
    let _ = parser.consume_horizontal_space();
    if at_expression_boundary(parser) || parser.cursor().starts_with(")") {
      missing_expression(parser, None);
    } else if matches!(parse_additive(parser), Attempt::NoMatch) {
      let _ = skip_error(
        parser,
        RecoveryClass::MechItem,
        "syntax/unexpected-token",
        "unexpected token inside parenthetical expression",
      );
    }
    let _ = parser.consume_horizontal_space();
    if parser.cursor().starts_with(")") {
      let _ = parser.bump_bytes_token(1, SyntaxKind::RightParen);
    } else {
      let missing = insert_missing(
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
      let _ = missing;
    }
    parser.pop_nesting();
  }
  parenthetical.complete_with_flags(
    parser,
    SyntaxKind::ParentheticalExpression,
    NodeFlags::REPARSE_ROOT,
  );
  parser.leave();
}

fn missing_expression(parser: &mut Parser<'_>, operator: Option<TextRange>) {
  let _ = insert_missing(
    parser,
    "syntax/missing-expression",
    "expected an expression",
    ExpectedSyntax::Production(String::from("expression")),
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
  parser.enter("code-terminal");
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
  parser.leave();
}

fn parse_comment(parser: &mut Parser<'_>) {
  let comment = parser.start();
  let start = parser.offset();
  while !parser.is_eof() && !is_newline_start(parser.cursor()) {
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
