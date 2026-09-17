use alloc::vec::Vec;

use crate::document::red::{AstNode, ParagraphSyntax, SectionSyntax, SyntaxNode};
use crate::document::SyntaxKind;

macro_rules! mechdown_ast_node {
  ($name:ident, $kind:ident) => {
    #[derive(Clone, Debug)]
    pub struct $name(pub(crate) SyntaxNode);

    impl AstNode for $name {
      fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::$kind
      }

      fn cast(syntax: SyntaxNode) -> Option<Self> {
        Self::can_cast(syntax.kind()).then_some(Self(syntax))
      }

      fn syntax(&self) -> &SyntaxNode {
        &self.0
      }
    }
  };
}

mechdown_ast_node!(InlineCodeSyntax, InlineCode);
mechdown_ast_node!(InlineEquationSyntax, InlineEquation);
mechdown_ast_node!(RawHyperlinkSyntax, RawHyperlink);
mechdown_ast_node!(FootnoteReferenceSyntax, FootnoteReference);
mechdown_ast_node!(ReferenceSyntax, Reference);
mechdown_ast_node!(SectionReferenceSyntax, SectionReference);
mechdown_ast_node!(ParagraphTextSyntax, ParagraphText);
mechdown_ast_node!(ThematicBreakSyntax, ThematicBreak);
mechdown_ast_node!(EquationSyntax, Equation);
mechdown_ast_node!(CommentSyntax, Comment);
mechdown_ast_node!(BlankLineSyntax, BlankLine);

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
