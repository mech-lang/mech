use mech_core::*;
use mech_core::nodes::{Kind, Matrix};
use std::collections::{HashMap, HashSet};
use colored::Colorize;
use std::io::{Read, Write, Cursor};
use crate::*;

mod document;
mod mechdown;
mod state_machines;
mod functions;
mod statements;
mod expressions;
mod structures;
mod operators;
mod literals;

/// Trait-based formatter interface used by consumers that only need the
/// top-level formatting entry points.
pub trait MechFormatter {
  /// Format a parsed program into source text.
  fn format_src(&mut self, tree: &Program) -> String;

  /// Format a parsed program into HTML using the provided style and page shim.
  fn format_html(&mut self, tree: &Program, style: String, shim: String) -> String;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Formatter{
  identifiers: HashMap<u64, String>,
  rows: usize,
  cols: usize,
  indent: usize,
  pub html: bool,
  nested: bool,
  toc: bool,
  figure_num: usize,
  h2_num: usize,
  h3_num: usize,
  h4_num: usize,
  h5_num: usize,
  h6_num: usize,
  citation_num: usize,
  citation_map: HashMap<u64, usize>,
  citations: Vec<String>,
  interpreter_id: u64,
}


impl Formatter {

  pub fn new() -> Formatter {
    Formatter {
      identifiers: HashMap::new(),
      rows: 0,
      cols: 0,
      indent: 0,
      h2_num: 0,
      h3_num: 0,
      h4_num: 0,
      h5_num: 0,
      h6_num: 0,
      citation_num: 0,
      citation_map: HashMap::new(),
      citations: Vec::new(),
      figure_num: 0,
      html: false,
      nested: false,
      toc: false,
      interpreter_id: 0,
    }
  }

  pub fn format(&mut self, tree: &Program) -> String {
    self.html = false;
    self.program(tree)
  }

  /*pub fn format_grammar(&mut self, tree: &Gramamr) -> String {
    self.html = false;
    self.grammar(tree)
  }*/

  pub fn reset_numbering(&mut self) {
    self.h2_num = 0;
    self.h3_num = 0;
    self.h4_num = 0;
    self.h5_num = 0;
    self.h6_num = 0;
    self.figure_num = 0;
  }

  pub fn works_cited(&mut self) -> String {
    if self.citations.is_empty() {
      return "".to_string();
    }
    let id = hash_str("works-cited");

    let gs = graphemes::init_source("Works Cited"); 
    let (_,text) = paragraph(ParseString::new(&gs)).unwrap();
    let h2 = self.subtitle(&Subtitle { level: 2, text });

    let mut src = format!(r#"<section id="67320967384727436" class="mech-works-cited">"#);
    src.push_str(&h2);
    for citation in &self.citations {
      src.push_str(citation);
    }
    src.push_str("</section>\n");
    src
  }

  pub fn format_html(&mut self, tree: &Program, style: String, shim: String) -> String {
    self.html = true;

    let toc = tree.table_of_contents();
    let formatted_src = self.program(tree);
    self.reset_numbering();
    let formatted_toc = self.table_of_contents(&toc);

    let title = match toc.title {
      Some(title) => title.to_string(),
      None => "Mech Program".to_string(),
    };
    
    #[cfg(feature = "serde")]
    let encoded_tree = match compress_and_encode(&tree) {
        Ok(encoded) => encoded,
        Err(e) => todo!(),
    };
    #[cfg(not(feature = "serde"))]
    let encoded_tree = String::new();

    shim.replace("{{STYLESHEET}}", &style)
        .replace("{{TOC}}", &formatted_toc)
        .replace("{{CONTENT}}", &formatted_src)
        .replace("{{CODE}}", &encoded_tree)
        .replace("{{TITLE}}", &title)
  }


}

impl MechFormatter for Formatter {
  fn format_src(&mut self, tree: &Program) -> String {
    self.format(tree)
  }

  fn format_html(&mut self, tree: &Program, style: String, shim: String) -> String {
    Formatter::format_html(self, tree, style, shim)
  }
}
