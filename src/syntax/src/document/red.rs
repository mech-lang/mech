use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt::Write;

use super::edit::{SourceError, TextRange, TextSize};
use super::flags::{NodeFlags, TokenFlags};
use super::green::{GreenElement, GreenNode, GreenToken};
use super::ids::{NodeId, TokenId};
use super::kind::SyntaxKind;
use super::source::TextSnapshot;

#[derive(Clone, Debug)]
pub struct SyntaxNode {
  green: Arc<GreenNode>,
  source: TextSnapshot,
  offset: TextSize,
}

impl SyntaxNode {
  pub fn new_root(green: Arc<GreenNode>, source: TextSnapshot) -> Self {
    Self {
      green,
      source,
      offset: TextSize::ZERO,
    }
  }

  pub fn id(&self) -> NodeId {
    self.green.id
  }

  pub fn kind(&self) -> SyntaxKind {
    self.green.kind
  }

  pub fn flags(&self) -> NodeFlags {
    self.green.flags
  }

  pub fn range(&self) -> TextRange {
    TextRange::at(self.offset, self.green.text_len)
  }

  pub fn text(&self) -> Result<String, SourceError> {
    self.source.text(self.range())
  }

  pub fn green(&self) -> &Arc<GreenNode> {
    &self.green
  }

  pub fn source(&self) -> &TextSnapshot {
    &self.source
  }

  pub fn children_with_tokens(&self) -> Vec<SyntaxElement> {
    let mut children = Vec::with_capacity(self.green.children.len());
    let mut offset = self.offset;
    for child in self.green.children.iter() {
      match child {
        GreenElement::Node(node) => {
          children.push(SyntaxElement::Node(Self {
            green: node.clone(),
            source: self.source.clone(),
            offset,
          }));
          offset += node.text_len;
        }
        GreenElement::Token(token) => {
          children.push(SyntaxElement::Token(SyntaxToken {
            green: *token,
            source: self.source.clone(),
            offset,
          }));
          offset += token.text_len;
        }
      }
    }
    children
  }

  pub fn children(&self) -> impl Iterator<Item = SyntaxNode> {
    self.children_with_tokens().into_iter().filter_map(|child| match child {
      SyntaxElement::Node(node) => Some(node),
      SyntaxElement::Token(_) => None,
    })
  }

  pub fn tokens(&self) -> Vec<SyntaxToken> {
    let mut tokens = Vec::new();
    self.collect_tokens(&mut tokens);
    tokens
  }

  pub fn first_child(&self, kind: SyntaxKind) -> Option<SyntaxNode> {
    self.children().find(|child| child.kind() == kind)
  }

  fn collect_tokens(&self, output: &mut Vec<SyntaxToken>) {
    for child in self.children_with_tokens() {
      match child {
        SyntaxElement::Node(node) => node.collect_tokens(output),
        SyntaxElement::Token(token) => output.push(token),
      }
    }
  }
}

#[derive(Clone, Debug)]
pub struct SyntaxToken {
  green: GreenToken,
  source: TextSnapshot,
  offset: TextSize,
}

impl SyntaxToken {
  pub fn id(&self) -> TokenId {
    self.green.id
  }

  pub fn kind(&self) -> SyntaxKind {
    self.green.kind
  }

  pub fn flags(&self) -> TokenFlags {
    self.green.flags
  }

  pub fn range(&self) -> TextRange {
    TextRange::at(self.offset, self.green.text_len)
  }

  pub fn text(&self) -> Result<String, SourceError> {
    self.source.text(self.range())
  }
}

#[derive(Clone, Debug)]
pub enum SyntaxElement {
  Node(SyntaxNode),
  Token(SyntaxToken),
}

pub fn compact_debug_tree(root: &SyntaxNode) -> String {
  let mut output = String::new();
  write_debug_node(root, 0, &mut output);
  output
}

fn write_debug_node(node: &SyntaxNode, depth: usize, output: &mut String) {
  for _ in 0..depth {
    output.push_str("  ");
  }
  let _ = writeln!(output, "{:?}", node.kind());
  for child in node.children_with_tokens() {
    match child {
      SyntaxElement::Node(child) => write_debug_node(&child, depth + 1, output),
      SyntaxElement::Token(token) => {
        for _ in 0..=depth {
          output.push_str("  ");
        }
        if token.flags().contains(TokenFlags::MISSING) {
          let _ = writeln!(output, "{:?} <missing>", token.kind());
        } else {
          let text = token.text().unwrap_or_default();
          let _ = writeln!(output, "{:?} {text:?}", token.kind());
        }
      }
    }
  }
}

pub trait AstNode: Clone {
  fn can_cast(kind: SyntaxKind) -> bool;
  fn cast(syntax: SyntaxNode) -> Option<Self>;
  fn syntax(&self) -> &SyntaxNode;
}

macro_rules! ast_node {
  ($name:ident, $($kind:pat_param)|+) => {
    #[derive(Clone, Debug)]
    pub struct $name(pub(crate) SyntaxNode);

    impl AstNode for $name {
      fn can_cast(kind: SyntaxKind) -> bool {
        matches!(kind, $($kind)|+)
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

ast_node!(DocumentSyntax, SyntaxKind::Document);
ast_node!(SectionSyntax, SyntaxKind::Section);
ast_node!(ParagraphSyntax, SyntaxKind::Paragraph);
ast_node!(MechItemSyntax, SyntaxKind::MechItem);
ast_node!(VariableDefineSyntax, SyntaxKind::VariableDefine);
ast_node!(IdentifierSyntax, SyntaxKind::Identifier);
ast_node!(
  ExpressionSyntax,
  SyntaxKind::Expression
    | SyntaxKind::AdditiveExpression
    | SyntaxKind::ParentheticalExpression
    | SyntaxKind::IntegerLiteral
    | SyntaxKind::Missing
);

impl VariableDefineSyntax {
  pub fn name(&self) -> Option<IdentifierSyntax> {
    self.0
      .first_child(SyntaxKind::Identifier)
      .and_then(IdentifierSyntax::cast)
  }

  pub fn define_operator(&self) -> Option<SyntaxToken> {
    self
      .0
      .first_child(SyntaxKind::DefineOperator)
      .and_then(|node| node.tokens().into_iter().next())
  }

  pub fn value(&self) -> Option<ExpressionSyntax> {
    self
      .0
      .children()
      .find_map(ExpressionSyntax::cast)
  }
}
