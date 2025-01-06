use mech_core::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Formatter{
  code: String,
  identifiers: HashMap<u64, String>,
  rows: usize,
  cols: usize,
  indent: usize,
  html: bool,
  nested: bool,
}

impl Formatter {

  pub fn new() -> Formatter {
    Formatter {
      code: String::new(),
      identifiers: HashMap::new(),
      rows: 0,
      cols: 0,
      indent: 0,
      html: false,
      nested: false,
    }
  }

  pub fn format(&mut self, tree: &Program) -> String {
    self.html = false;
    let src = self.write_program(tree,&"".to_string());
    src
  }

  pub fn write_program(&mut self, node: &Program, src: &String) -> String {
    let src = self.write_title(&node.title, src);
    src
  }

  pub fn write_title(&mut self, node: &Option<Title>, src: &String) -> String {
    if let Some(title) = node {
      format!("{}{}\n==============",src,title.to_string())
    } else {
      "".to_string()
    }
  }

}