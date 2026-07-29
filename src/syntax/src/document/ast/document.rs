use alloc::vec::Vec;

use crate::document::red::{AstNode, DocumentSyntax, SectionSyntax, SyntaxNode};
use crate::document::SyntaxKind;

impl DocumentSyntax {
  pub fn sections(&self) -> Vec<SectionSyntax> {
    let mut sections = Vec::new();
    collect_sections(self.syntax(), &mut sections);
    sections
  }
}

fn collect_sections(node: &SyntaxNode, output: &mut Vec<SectionSyntax>) {
  for child in node.children() {
    if let Some(section) = SectionSyntax::cast(child.clone()) {
      output.push(section);
    } else if child.kind() == SyntaxKind::Body {
      collect_sections(&child, output);
    }
  }
}
