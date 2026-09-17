use crate::document::{NodeFlags, SyntaxKind};

use super::mech;
use super::mechdown;
use super::recovery::Attempt;
use super::terminal::is_newline_start;
use super::{Parser, parser_context_id};

pub(crate) fn parse_document_root(parser: &mut Parser<'_>) {
  parser.with_rule(
    parser_context_id("document"),
    None,
    |parser| {
      let document = parser.start();
      let body = parser.start();
      if parser.is_eof() {
        parse_section(parser);
      } else {
        while !parser.is_eof() {
          if parser.is_halted() {
            parser.consume_resource_remainder();
            break;
          }
          let before = parser.offset();
          parse_section(parser);
          if parser.offset() == before && !parser.is_halted() {
            parser.consume_resource_remainder();
            break;
          }
        }
      }
      if parser.is_halted() {
        parser.consume_resource_remainder();
      }
      body.complete(parser, SyntaxKind::Body);
      document.complete_with_flags(
        parser,
        SyntaxKind::Document,
        NodeFlags::REPARSE_ROOT,
      );
    },
  );
}

pub(crate) fn parse_section(parser: &mut Parser<'_>) {
  parser.with_rule(
    parser_context_id("prototype-section"),
    None,
    |parser| {
      let section = parser.start();
      let mut has_content = false;
      if parser.cursor().is_line_start() && mechdown::is_ul_subtitle(parser.cursor()) {
        mechdown::parse_ul_subtitle(parser);
        has_content = true;
      }

      while !parser.is_eof() && !parser.is_halted() {
        if has_content
          && parser.cursor().is_line_start()
          && mechdown::is_ul_subtitle(parser.cursor())
        {
          break;
        }
        let before = parser.offset();
        if is_newline_start(parser.cursor()) {
          let _ = parser.consume_newline();
        } else {
          let _ = parse_section_element(parser);
        }
        has_content = true;
        if parser.offset() == before && !parser.is_halted() {
          parser.consume_resource_remainder();
          break;
        }
      }
      section.complete_with_flags(
        parser,
        SyntaxKind::Section,
        NodeFlags::REPARSE_ROOT,
      );
    },
  );
}

pub(crate) fn parse_section_element(parser: &mut Parser<'_>) -> Attempt<()> {
  parser.with_rule(
    parser_context_id("prototype-section-element"),
    None,
    |parser| {
      if parser.is_eof()
        || is_newline_start(parser.cursor())
        || (parser.cursor().is_line_start() && mechdown::is_ul_subtitle(parser.cursor()))
      {
        return Attempt::NoMatch;
      }
      let element = parser.start();
      let outcome = if mechdown::is_subtitle(parser.cursor()) {
        mechdown::parse_subtitle(parser);
        Attempt::Match(())
      } else if parser.is_fence_start() {
        mechdown::parse_generic_fence(parser);
        Attempt::Match(())
      } else {
        match mech::parse_mech_item(parser) {
          Attempt::Match(()) => Attempt::Match(()),
          Attempt::CommittedFailure(failure) => Attempt::CommittedFailure(failure),
          Attempt::NoMatch => {
            mechdown::parse_paragraph(parser);
            Attempt::Match(())
          }
        }
      };
      element.complete_with_flags(
        parser,
        SyntaxKind::SectionElement,
        NodeFlags::REPARSE_ROOT,
      );
      outcome
    },
  )
}
