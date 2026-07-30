#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

macro_rules! syntax_kind_is_token {
  (node) => {
    false
  };
  (token) => {
    true
  };
}

macro_rules! define_syntax_kinds {
  ($( $kind:ident => $category:ident, )+) => {
    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
    #[repr(u16)]
    pub enum SyntaxKind {
      $( $kind, )+
    }

    impl SyntaxKind {
      pub const fn is_token(self) -> bool {
        match self {
          $( Self::$kind => syntax_kind_is_token!($category), )+
        }
      }
    }
  };
}

// Keep this as the single authoritative declaration for both discriminant
// order and token classification. Phase 1 kinds retain their original order;
// subsequent phases append without disturbing existing snapshot identities.
define_syntax_kinds! {
  Document => node,
  Body => node,
  Section => node,
  SectionElement => node,
  Title => node,
  Subtitle => node,
  UlSubtitle => node,
  Paragraph => node,
  ParagraphElement => node,
  ParagraphText => node,
  GenericFence => node,
  FenceContent => node,
  MechItem => node,
  VariableDefine => node,
  Identifier => node,
  KindAnnotation => node,
  Expression => node,
  AdditiveExpression => node,
  ParentheticalExpression => node,
  IntegerLiteral => node,
  DefineOperator => node,
  Comment => node,
  Error => node,
  Missing => node,

  Text => token,
  Whitespace => token,
  Newline => token,
  IdentifierToken => token,
  IntegerToken => token,
  Colon => token,
  Equal => token,
  Tilde => token,
  LeftAngle => token,
  RightAngle => token,
  Plus => token,
  LeftParen => token,
  RightParen => token,
  Period => token,
  Dash => token,
  Semicolon => token,
  FenceDelimiter => token,
  CommentToken => token,
  Unknown => token,
  Eof => token,

  // Phase 2A semantic token categories. `DefineOperator` above remains the
  // Phase 1 structural node; the lexical category has an explicit token name.
  AbstractSigil => token,
  Alpha => token,
  Ampersand => token,
  Any => token,
  Apostrophe => token,
  Asterisk => token,
  AssignOperator => token,
  AsyncTransitionOperator => token,
  At => token,
  Backslash => token,
  Bar => token,
  BoxDrawing => token,
  Caret => token,
  CarriageReturn => token,
  Comma => token,
  DefineOperatorToken => token,
  Digit => token,
  Dollar => token,
  Emoji => token,
  EmphasisSigil => token,
  EquationSigil => token,
  ErrorSigil => token,
  EscapedChar => token,
  Exclamation => token,
  False => token,
  FloatLeft => token,
  FloatRight => token,
  FootnotePrefix => token,
  GenOperator => token,
  GeneratorArrow => token,
  Grave => token,
  GraveCodeBlockSigil => token,
  HashTag => token,
  HighlightSigil => token,
  HttpPrefix => token,
  IdeaSigil => token,
  ImgPrefix => token,
  InfoSigil => token,
  LeftBrace => token,
  LeftBracket => token,
  MikaSectionOpen => token,
  MikaSectionClose => token,
  ModuleExportSigil => token,
  ModuleImportSigil => token,
  Not => token,
  OutputOperator => token,
  Percent => token,
  PromptSigil => token,
  Question => token,
  QuestionSigil => token,
  Quote => token,
  QuoteSigil => token,
  RightBrace => token,
  RightBracket => token,
  SectionSigil => token,
  Slash => token,
  SpreadOperator => token,
  StrikeSigil => token,
  StrongSigil => token,
  SuccessSigil => token,
  SynthOperator => token,
  Tab => token,
  TildeCodeBlockSigil => token,
  TransitionOperator => token,
  True => token,
  UnderlineSigil => token,
  Underscore => token,
  WarningSigil => token,

  // Phase 2A canonical grammar and compatibility-value nodes.
  CanonicalFragment => node,
  GrammarDocument => node,
  Grammar => node,
  GrammarRule => node,
  GrammarIdentifier => node,
  GrammarExpression => node,
  GrammarTerm => node,
  GrammarFactor => node,
  GrammarDefinition => node,
  GrammarRepeat0 => node,
  GrammarRepeat1 => node,
  GrammarOptional => node,
  GrammarPeek => node,
  GrammarNot => node,
  GrammarList => node,
  GrammarRange => node,
  GrammarGroup => node,
  GrammarTerminal => node,
  GrammarTerminalToken => node,
  DigitSequence => node,
  IdentifierPathSegment => node,
  EscapedCharacter => node,

  // Phase 2B canonical Mechdown values.
  InlineCode => node,
  InlineEquation => node,
  RawHyperlink => node,
  FootnoteReference => node,
  Reference => node,
  SectionReference => node,
  ThematicBreak => node,
  BlankLine => node,
  Equation => node,

  // Phase 2C closed literal, path, and primitive-kind values. Keep this
  // append-only: persisted green-tree discriminants depend on the prior order.
  EmptyLiteral => node,
  AtomLiteral => node,
  StringLiteral => node,
  Utf8String => node,
  RawString => node,
  Number => node,
  ComplexNumber => node,
  RealNumber => node,
  UntypedRealNumber => node,
  RationalLiteral => node,
  ScientificLiteral => node,
  FloatDecimalStart => node,
  FloatFull => node,
  FloatLiteral => node,
  TypedInteger => node,
  UntypedInteger => node,
  DecimalLiteral => node,
  HexadecimalLiteral => node,
  OctalLiteral => node,
  BinaryLiteral => node,
  ContextAddressPath => node,
  PrefixedContextPath => node,
  KindAny => node,
  KindEmpty => node,
  KindAtom => node,
}

#[cfg(test)]
mod tests {
  use super::SyntaxKind;

  #[test]
  fn phase_2a_kinds_are_append_only() {
    assert_eq!(SyntaxKind::Document as u16, 0);
    assert_eq!(SyntaxKind::DefineOperator as u16, 20);
    assert_eq!(SyntaxKind::Text as u16, 24);
    assert_eq!(SyntaxKind::Eof as u16, 43);
    assert_eq!(SyntaxKind::AbstractSigil as u16, 44);
    assert!(
      SyntaxKind::GrammarDocument as u16 > SyntaxKind::WarningSigil as u16
    );
    assert_eq!(SyntaxKind::EscapedCharacter as u16, 133);
  }

  #[test]
  fn phase_2b_kinds_are_append_only() {
    let appended = [
      SyntaxKind::InlineCode,
      SyntaxKind::InlineEquation,
      SyntaxKind::RawHyperlink,
      SyntaxKind::FootnoteReference,
      SyntaxKind::Reference,
      SyntaxKind::SectionReference,
      SyntaxKind::ThematicBreak,
      SyntaxKind::BlankLine,
      SyntaxKind::Equation,
    ];
    for (offset, kind) in appended.into_iter().enumerate() {
      assert_eq!(kind as u16, 134 + offset as u16);
      assert!(!kind.is_token());
    }
  }

  #[test]
  fn phase_2c_kinds_are_append_only() {
    let appended = [
      SyntaxKind::EmptyLiteral,
      SyntaxKind::AtomLiteral,
      SyntaxKind::StringLiteral,
      SyntaxKind::Utf8String,
      SyntaxKind::RawString,
      SyntaxKind::Number,
      SyntaxKind::ComplexNumber,
      SyntaxKind::RealNumber,
      SyntaxKind::UntypedRealNumber,
      SyntaxKind::RationalLiteral,
      SyntaxKind::ScientificLiteral,
      SyntaxKind::FloatDecimalStart,
      SyntaxKind::FloatFull,
      SyntaxKind::FloatLiteral,
      SyntaxKind::TypedInteger,
      SyntaxKind::UntypedInteger,
      SyntaxKind::DecimalLiteral,
      SyntaxKind::HexadecimalLiteral,
      SyntaxKind::OctalLiteral,
      SyntaxKind::BinaryLiteral,
      SyntaxKind::ContextAddressPath,
      SyntaxKind::PrefixedContextPath,
      SyntaxKind::KindAny,
      SyntaxKind::KindEmpty,
      SyntaxKind::KindAtom,
    ];
    // `IntegerLiteral` already exists at its Phase 1 discriminant and is the
    // canonical node reused by this closed island. Every genuinely new Phase
    // 2C kind follows the existing Phase 2B tail in the specified order.
    assert_eq!(SyntaxKind::IntegerLiteral as u16, 19);
    for (offset, kind) in appended.into_iter().enumerate() {
      assert_eq!(kind as u16, 143 + offset as u16);
      assert!(!kind.is_token());
    }
  }

  #[test]
  fn token_classification_comes_from_the_kind_declaration() {
    assert!(!SyntaxKind::DefineOperator.is_token());
    assert!(SyntaxKind::DefineOperatorToken.is_token());
    assert!(SyntaxKind::AbstractSigil.is_token());
    assert!(SyntaxKind::Alpha.is_token());
    assert!(SyntaxKind::Ampersand.is_token());
    assert!(SyntaxKind::Any.is_token());
    assert!(SyntaxKind::ErrorSigil.is_token());
    assert!(SyntaxKind::WarningSigil.is_token());
    assert!(!SyntaxKind::CanonicalFragment.is_token());
    assert!(!SyntaxKind::GrammarDocument.is_token());
    assert!(!SyntaxKind::GrammarTerminalToken.is_token());
  }
}
