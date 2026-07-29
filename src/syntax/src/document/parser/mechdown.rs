use alloc::string::String;

use crate::document::{
  DiagnosticAnchor, DiagnosticFix, DiagnosticLabel, ExpectedSyntax, FixApplicability, NodeFlags,
  SyntaxKind, TextEdit, TextRange, TokenFlags,
};

use super::recovery::{RecoveryClass, insert_missing, skip_error};
use super::terminal::{
  is_horizontal_space, is_newline_start, token_kind_for_char,
};
use super::{ContextView, Cursor, Parser, parser_context_id};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FenceDelimiter {
  Grave,
  Tilde,
}

impl FenceDelimiter {
  fn text(self) -> &'static str {
    match self {
      Self::Grave => "```",
      Self::Tilde => "~~~",
    }
  }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FenceStart {
  pub delimiter: FenceDelimiter,
  pub indentation_bytes: u32,
}

pub(crate) fn fence_delimiter(cursor: &Cursor<'_>) -> Option<FenceStart> {
  let mut lookahead = cursor.clone();
  let indentation_start = lookahead.offset();
  while lookahead.peek_char().is_some_and(is_horizontal_space) {
    let _ = lookahead.bump_char();
  }
  let indentation_bytes = lookahead.offset().0 - indentation_start.0;
  let delimiter = if lookahead.starts_with("```") {
    FenceDelimiter::Grave
  } else if lookahead.starts_with("~~~") {
    FenceDelimiter::Tilde
  } else {
    return None;
  };
  Some(FenceStart {
    delimiter,
    indentation_bytes,
  })
}

pub(crate) fn fence_delimiter_context(
  view: ContextView<'_>,
) -> Option<FenceStart> {
  let mut relative = 0_u32;
  while view
    .at_relative(relative)
    .and_then(ContextView::peek_char)
    .is_some_and(is_horizontal_space)
  {
    relative = relative.saturating_add(
      view
        .at_relative(relative)
        .and_then(ContextView::peek_char)?
        .len_utf8() as u32,
    );
  }
  let lookahead = view.at_relative(relative)?;
  let delimiter = if lookahead.starts_with("```") {
    FenceDelimiter::Grave
  } else if lookahead.starts_with("~~~") {
    FenceDelimiter::Tilde
  } else {
    return None;
  };
  Some(FenceStart {
    delimiter,
    indentation_bytes: relative,
  })
}

pub(crate) fn is_ul_subtitle(cursor: &Cursor<'_>) -> bool {
  let mut lookahead = cursor.clone();
  if !lookahead.is_line_start() {
    return false;
  }
  let mut count = 0;
  while lookahead
    .peek_char()
    .is_some_and(|character| character.is_alphanumeric())
  {
    count += 1;
    let _ = lookahead.bump_char();
  }
  if count == 0 || !lookahead.starts_with(".") {
    return false;
  }
  let _ = lookahead.bump_bytes(1);
  consume_horizontal_lookahead(&mut lookahead);
  let title_start = lookahead.offset();
  while !lookahead.is_eof() && !is_newline_start(&lookahead) {
    let _ = lookahead.bump_char();
  }
  if lookahead.offset() == title_start || consume_newline_lookahead(&mut lookahead).is_none() {
    return false;
  }
  let mut dashes = 0;
  while lookahead.starts_with("-") {
    dashes += 1;
    let _ = lookahead.bump_bytes(1);
  }
  if dashes == 0 {
    return false;
  }
  consume_horizontal_lookahead(&mut lookahead);
  consume_newline_lookahead(&mut lookahead).is_some() || lookahead.is_eof()
}

pub(crate) fn is_ul_subtitle_context(view: ContextView<'_>) -> bool {
  if !view.is_line_start() {
    return false;
  }
  let mut relative = 0_u32;
  let mut count = 0_u32;
  while let Some(character) = view
    .at_relative(relative)
    .and_then(ContextView::peek_char)
    .filter(|character| character.is_alphanumeric())
  {
    count = count.saturating_add(1);
    relative = relative.saturating_add(character.len_utf8() as u32);
  }
  let Some(dot) = view.at_relative(relative) else {
    return false;
  };
  if count == 0 || !dot.starts_with(".") {
    return false;
  }
  relative = relative.saturating_add(1);
  relative = consume_horizontal_context(view, relative);
  let title_start = relative;
  relative = consume_until_newline_context(view, relative);
  if relative == title_start {
    return false;
  }
  let Some(after_title) = consume_newline_context(view, relative) else {
    return false;
  };
  relative = after_title;
  let mut dashes = 0_u32;
  while view
    .at_relative(relative)
    .is_some_and(|lookahead| lookahead.starts_with("-"))
  {
    dashes = dashes.saturating_add(1);
    relative = relative.saturating_add(1);
  }
  if dashes == 0 {
    return false;
  }
  relative = consume_horizontal_context(view, relative);
  consume_newline_context(view, relative).is_some()
    || view
      .at_relative(relative)
      .is_some_and(|lookahead| lookahead.offset() == lookahead.end())
}

pub(crate) fn is_subtitle(cursor: &Cursor<'_>) -> bool {
  let mut lookahead = cursor.clone();
  consume_horizontal_lookahead(&mut lookahead);
  if !lookahead.starts_with("(") {
    return false;
  }
  let _ = lookahead.bump_bytes(1);
  if !consume_subtitle_segment(&mut lookahead) {
    return false;
  }
  while lookahead.starts_with(".") {
    let _ = lookahead.bump_bytes(1);
    if !consume_subtitle_segment(&mut lookahead) {
      return false;
    }
  }
  if !lookahead.starts_with(")") {
    return false;
  }
  let _ = lookahead.bump_bytes(1);
  consume_horizontal_lookahead(&mut lookahead);
  let title_start = lookahead.offset();
  while !lookahead.is_eof() && !is_newline_start(&lookahead) {
    let _ = lookahead.bump_char();
  }
  lookahead.offset() != title_start
    && (consume_newline_lookahead(&mut lookahead).is_some() || lookahead.is_eof())
}

pub(crate) fn parse_ul_subtitle(parser: &mut Parser<'_>) {
  parser.with_rule(
    parser_context_id("prototype-ul-subtitle"),
    None,
    |parser| {
      let heading = parser.start();
      consume_line_tokens(parser);
      consume_line_tokens(parser);
      heading.complete_with_flags(
        parser,
        SyntaxKind::UlSubtitle,
        NodeFlags::REPARSE_ROOT,
      );
    },
  );
}

pub(crate) fn parse_subtitle(parser: &mut Parser<'_>) {
  parser.with_rule(
    parser_context_id("prototype-subtitle"),
    None,
    |parser| {
      let heading = parser.start();
      consume_line_tokens(parser);
      heading.complete_with_flags(
        parser,
        SyntaxKind::Subtitle,
        NodeFlags::REPARSE_ROOT,
      );
    },
  );
}

pub(crate) fn parse_generic_fence(parser: &mut Parser<'_>) {
  parser.with_rule(
    parser_context_id("prototype-code-block"),
    None,
    |parser| {
      let fence_start = fence_delimiter(parser.cursor()).expect("fence was checked");
      let fence = parser.start();
      let _ = parser.consume_horizontal_space();
      let opening = parser
        .bump_bytes_token(3, SyntaxKind::FenceDelimiter)
        .unwrap_or_else(|| TextRange::empty(parser.offset()));
      consume_line_tokens(parser);

      let content = parser.start();
      while !parser.is_eof() && !parser.is_halted() {
        if parser.cursor().is_line_start()
          && matching_fence(parser.cursor(), fence_start.delimiter)
        {
          break;
        }
        consume_content_token(parser);
      }
      content.complete(parser, SyntaxKind::FenceContent);

      if matching_fence(parser.cursor(), fence_start.delimiter) {
        let _ = parser.consume_horizontal_space();
        let _ = parser.bump_bytes_token(3, SyntaxKind::FenceDelimiter);
        consume_line_tokens(parser);
      } else if !parser.is_halted() {
        let _ = insert_missing(
          parser,
          "syntax/unclosed-fence",
          "missing closing code fence",
          ExpectedSyntax::Token(SyntaxKind::FenceDelimiter),
          Some(SyntaxKind::FenceDelimiter),
        );
        let revision = parser.source().revision();
        let offset = parser.offset();
        if let Some(diagnostic) = parser.last_diagnostic_mut() {
          diagnostic.labels.push(DiagnosticLabel {
            anchor: DiagnosticAnchor::Absolute {
              revision,
              range: opening,
            },
            message: String::from("opening fence is here"),
          });
          diagnostic.fixes.push(DiagnosticFix {
            title: String::from("Insert the matching closing fence"),
            applicability: FixApplicability::MachineApplicable,
            edits: alloc::vec![TextEdit::insert(
              offset,
              String::from(fence_start.delimiter.text()),
            )],
          });
        }
      }
      fence.complete_with_flags(
        parser,
        SyntaxKind::GenericFence,
        NodeFlags::REPARSE_ROOT,
      );
    },
  );
}

pub(crate) fn parse_paragraph(parser: &mut Parser<'_>) {
  parser.with_rule(
    parser_context_id("prototype-paragraph"),
    None,
    |parser| {
      let paragraph = parser.start();
      while !parser.is_eof() && !is_newline_start(parser.cursor()) && !parser.is_halted() {
        if parser.cursor().starts_with(":=") {
          let _ = skip_exact_paragraph_error(
            parser,
            2,
            "syntax/invalid-paragraph-element",
            "define operator is not valid paragraph text",
          );
          continue;
        }
        if parser
          .cursor()
          .peek_char()
          .is_some_and(is_horizontal_space)
        {
          let _ = parser.consume_horizontal_space();
          continue;
        }
        if parser
          .cursor()
          .peek_char()
          .is_some_and(|character| matches!(character, '`' | '[' | ']' | '{' | '}' | '*' | '_' | '~'))
        {
          let _ = skip_error(
            parser,
            RecoveryClass::Paragraph,
            "syntax/invalid-paragraph-element",
            "invalid or incomplete paragraph element",
          );
          continue;
        }

        let element = parser.start();
        let text = parser.start();
        let start = parser.offset();
        while !parser.is_eof()
          && !parser.is_halted()
          && !is_newline_start(parser.cursor())
          && !parser.cursor().starts_with(":=")
          && !parser
            .cursor()
            .peek_char()
            .is_some_and(is_horizontal_space)
          && !parser
            .cursor()
            .peek_char()
            .is_some_and(|character| matches!(character, '`' | '[' | ']' | '{' | '}' | '*' | '_' | '~'))
        {
          let _ = parser.bump_char_raw();
        }
        if parser.offset() == start {
          let _ = parser.bump_char_raw();
        }
        parser.token(SyntaxKind::Text, TextRange::new(start, parser.offset()));
        text.complete(parser, SyntaxKind::ParagraphText);
        element.complete(parser, SyntaxKind::ParagraphElement);
      }
      let _ = parser.consume_newline();
      paragraph.complete_with_flags(
        parser,
        SyntaxKind::Paragraph,
        NodeFlags::REPARSE_ROOT,
      );
    },
  );
}

fn matching_fence(cursor: &Cursor<'_>, delimiter: FenceDelimiter) -> bool {
  let Some(found) = fence_delimiter(cursor) else {
    return false;
  };
  found.delimiter == delimiter
}

fn consume_content_token(parser: &mut Parser<'_>) {
  if parser.consume_newline().is_some() {
    return;
  }
  if parser
    .cursor()
    .peek_char()
    .is_some_and(is_horizontal_space)
  {
    let _ = parser.consume_horizontal_space();
    return;
  }
  let start = parser.offset();
  while !parser.is_eof()
    && !parser.is_halted()
    && !is_newline_start(parser.cursor())
    && !parser
      .cursor()
      .peek_char()
      .is_some_and(is_horizontal_space)
  {
    let _ = parser.bump_char_raw();
  }
  if parser.offset() > start {
    parser.token(SyntaxKind::Text, TextRange::new(start, parser.offset()));
  }
}

fn consume_line_tokens(parser: &mut Parser<'_>) {
  while !parser.is_eof() && !is_newline_start(parser.cursor()) && !parser.is_halted() {
    if parser
      .cursor()
      .peek_char()
      .is_some_and(is_horizontal_space)
    {
      let _ = parser.consume_horizontal_space();
      continue;
    }
    let Some((character, range)) = parser.bump_char_raw() else {
      break;
    };
    parser.token(token_kind_for_char(character), range);
  }
  let _ = parser.consume_newline();
}

fn skip_exact_paragraph_error(
  parser: &mut Parser<'_>,
  bytes: u32,
  code: &str,
  message: &str,
) -> Option<()> {
  let marker = parser.start();
  let start = parser.offset();
  for _ in 0..bytes {
    let Some((character, range)) = parser.bump_char_raw() else {
      break;
    };
    parser.token_with_flags(
      token_kind_for_char(character),
      range,
      TokenFlags::ERROR,
    );
  }
  if parser.offset() == start {
    marker.abandon(parser);
    return None;
  }
  let error = marker.complete_with_flags(parser, SyntaxKind::Error, NodeFlags::ERROR);
  let range = TextRange::new(start, parser.offset());
  let diagnostic = crate::document::Diagnostic {
    id: parser.next_diagnostic_id(),
    code: crate::document::DiagnosticCode::from(code),
    phase: crate::document::DiagnosticPhase::Syntax,
    severity: crate::document::Severity::Error,
    rule: parser.current_rule(),
    context: parser.current_context(),
    primary: DiagnosticAnchor::Absolute {
      revision: parser.source().revision(),
      range,
    },
    labels: alloc::vec![],
    expected: alloc::vec![ExpectedSyntax::Production(String::from(
      "paragraph-element",
    ))],
    found: Some(crate::document::FoundSyntax {
      kind: Some(SyntaxKind::DefineOperator),
      text: parser.source().text(range).ok(),
    }),
    fixes: alloc::vec![],
    related: alloc::vec![],
    recovery: Some(crate::document::RecoveryAction::Skip { range }),
    tags: crate::document::DiagnosticTags::NONE,
    message: String::from(message),
  };
  parser.push_diagnostic(
    diagnostic,
    Some(error.position()),
    TextRange::new(crate::document::TextSize::ZERO, range.len()),
  );
  Some(())
}

fn consume_horizontal_lookahead(cursor: &mut Cursor<'_>) {
  while cursor.peek_char().is_some_and(is_horizontal_space) {
    let _ = cursor.bump_char();
  }
}

fn consume_newline_lookahead(cursor: &mut Cursor<'_>) -> Option<TextRange> {
  match (cursor.byte(), cursor.byte_at(1)) {
    (Some(b'\r'), Some(b'\n')) => cursor.bump_bytes(2),
    (Some(b'\r' | b'\n'), _) => cursor.bump_bytes(1),
    _ => None,
  }
}

fn consume_subtitle_segment(cursor: &mut Cursor<'_>) -> bool {
  let Some(first) = cursor.peek_char() else {
    return false;
  };
  let alphabetic = first.is_alphabetic();
  let numeric = first.is_numeric();
  if !alphabetic && !numeric {
    return false;
  }
  let mut consumed = false;
  while cursor.peek_char().is_some_and(|character| {
    (alphabetic && character.is_alphabetic())
      || (numeric && character.is_numeric())
  }) {
    consumed = true;
    let _ = cursor.bump_char();
  }
  consumed
}

fn consume_horizontal_context(
  view: ContextView<'_>,
  mut relative: u32,
) -> u32 {
  while let Some(character) = view
    .at_relative(relative)
    .and_then(ContextView::peek_char)
    .filter(|character| is_horizontal_space(*character))
  {
    relative = relative.saturating_add(character.len_utf8() as u32);
  }
  relative
}

fn consume_until_newline_context(
  view: ContextView<'_>,
  mut relative: u32,
) -> u32 {
  while let Some(character) = view
    .at_relative(relative)
    .and_then(ContextView::peek_char)
  {
    if matches!(character, '\r' | '\n') {
      break;
    }
    relative = relative.saturating_add(character.len_utf8() as u32);
  }
  relative
}

fn consume_newline_context(
  view: ContextView<'_>,
  relative: u32,
) -> Option<u32> {
  let lookahead = view.at_relative(relative)?;
  if lookahead.starts_with("\r\n") {
    Some(relative.saturating_add(2))
  } else if lookahead.starts_with("\r") || lookahead.starts_with("\n") {
    Some(relative.saturating_add(1))
  } else {
    None
  }
}
