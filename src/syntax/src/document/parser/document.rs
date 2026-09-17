use crate::document::{NodeFlags, SyntaxKind};

use super::mech;
use super::mechdown;
use super::recovery::Attempt;
use super::terminal::is_newline_start;
use super::Parser;

pub(crate) fn parse_document_root(parser: &mut Parser<'_>) {
  parser.enter("document");
  let document = parser.start();
  let body = parser.start();
  let mut section = parser.start();
  let mut section_has_content = false;

  while !parser.is_eof() {
    if parser.is_halted() {
      parser.consume_resource_remainder();
      section_has_content = true;
      break;
    }
    let before = parser.offset();

    if parser.cursor().is_line_start() && mechdown::is_ul_subtitle(parser.cursor()) {
      if section_has_content {
        section.complete_with_flags(
          parser,
          SyntaxKind::Section,
          NodeFlags::REPARSE_ROOT,
        );
        section = parser.start();
      }
      mechdown::parse_ul_subtitle(parser);
      section_has_content = true;
    } else if is_newline_start(parser.cursor()) {
      let _ = parser.consume_newline();
      section_has_content = true;
    } else {
      let element = parser.start();
      if mechdown::is_subtitle(parser.cursor()) {
        mechdown::parse_subtitle(parser);
      } else if parser.is_fence_start() {
        mechdown::parse_generic_fence(parser);
      } else {
        match mech::parse_mech_item(parser) {
          Attempt::Match(()) | Attempt::CommittedFailure(_) => {}
          Attempt::NoMatch => mechdown::parse_paragraph(parser),
        }
      }
      element.complete_with_flags(
        parser,
        SyntaxKind::SectionElement,
        NodeFlags::REPARSE_ROOT,
      );
      section_has_content = true;
    }

    if parser.offset() == before && !parser.is_halted() {
      parser.consume_resource_remainder();
      break;
    }
  }

  if parser.is_halted() {
    parser.consume_resource_remainder();
  }

  section.complete_with_flags(
    parser,
    SyntaxKind::Section,
    NodeFlags::REPARSE_ROOT,
  );
  body.complete(parser, SyntaxKind::Body);
  document.complete_with_flags(
    parser,
    SyntaxKind::Document,
    NodeFlags::REPARSE_ROOT,
  );
  parser.leave();
}
