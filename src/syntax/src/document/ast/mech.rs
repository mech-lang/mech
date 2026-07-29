use crate::document::red::{AstNode, MechItemSyntax, VariableDefineSyntax};
use crate::document::SyntaxKind;

impl MechItemSyntax {
  pub fn variable_definition(&self) -> Option<VariableDefineSyntax> {
    self
      .syntax()
      .first_child(SyntaxKind::VariableDefine)
      .and_then(VariableDefineSyntax::cast)
  }
}
