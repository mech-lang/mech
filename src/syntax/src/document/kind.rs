#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[repr(u16)]
pub enum SyntaxKind {
  Document,
  Body,
  Section,
  SectionElement,
  Title,
  Subtitle,
  UlSubtitle,
  Paragraph,
  ParagraphElement,
  ParagraphText,
  GenericFence,
  FenceContent,
  MechItem,
  VariableDefine,
  Identifier,
  KindAnnotation,
  Expression,
  AdditiveExpression,
  ParentheticalExpression,
  IntegerLiteral,
  DefineOperator,
  Comment,
  Error,
  Missing,

  Text,
  Whitespace,
  Newline,
  IdentifierToken,
  IntegerToken,
  Colon,
  Equal,
  Tilde,
  LeftAngle,
  RightAngle,
  Plus,
  LeftParen,
  RightParen,
  Period,
  Dash,
  Semicolon,
  FenceDelimiter,
  CommentToken,
  Unknown,
  Eof,
}

impl SyntaxKind {
  pub const fn is_token(self) -> bool {
    matches!(
      self,
      Self::Text
        | Self::Whitespace
        | Self::Newline
        | Self::IdentifierToken
        | Self::IntegerToken
        | Self::Colon
        | Self::Equal
        | Self::Tilde
        | Self::LeftAngle
        | Self::RightAngle
        | Self::Plus
        | Self::LeftParen
        | Self::RightParen
        | Self::Period
        | Self::Dash
        | Self::Semicolon
        | Self::FenceDelimiter
        | Self::CommentToken
        | Self::Unknown
        | Self::Eof
    )
  }
}
