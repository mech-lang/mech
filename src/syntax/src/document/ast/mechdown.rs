use alloc::vec::Vec;

use crate::document::red::{AstNode, ParagraphSyntax, SectionSyntax, SyntaxNode};
use crate::document::SyntaxKind;

impl SectionSyntax {
  pub fn items(&self) -> Vec<SyntaxNode> {
    self
      .syntax()
      .children()
      .filter(|child| child.kind() == SyntaxKind::SectionElement)
      .collect()
  }

  pub fn paragraphs(&self) -> Vec<ParagraphSyntax> {
    let mut paragraphs = Vec::new();
    for item in self.items() {
      for child in item.children() {
        if let Some(paragraph) = ParagraphSyntax::cast(child) {
          paragraphs.push(paragraph);
        }
      }
    }
    paragraphs
  }
}
