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

  // Phase 2D closed expression-operator values. Keep this append-only:
  // persisted green-tree discriminants depend on every preceding kind.
  AddSubOperator => node,
  MulDivOperator => node,
  PowerOperator => node,
  MatrixOperator => node,
  RangeOperator => node,
  ComparisonOperator => node,
  LogicOperator => node,
  TableOperator => node,
  SetOperator => node,
  AddOperation => node,
  SubtractOperation => node,
  RawSubtractOperation => node,
  SpacedSubtractOperation => node,
  MultiplyOperation => node,
  DivideOperation => node,
  ModulusOperation => node,
  PowerOperation => node,
  MatrixMultiplyOperation => node,
  MatrixSolveOperation => node,
  DotProductOperation => node,
  CrossProductOperation => node,
  RangeInclusiveOperation => node,
  RangeExclusiveOperation => node,
  NotEqualOperation => node,
  EqualToOperation => node,
  StrictNotEqualOperation => node,
  StrictEqualOperation => node,
  GreaterThanOperation => node,
  LessThanOperation => node,
  GreaterThanEqualOperation => node,
  LessThanEqualOperation => node,
  OrOperation => node,
  AndOperation => node,
  NotOperation => node,
  XorOperation => node,
  JoinOperation => node,
  LeftJoinOperation => node,
  RightJoinOperation => node,
  FullJoinOperation => node,
  LeftSemiJoinOperation => node,
  LeftAntiJoinOperation => node,
  UnionOperation => node,
  IntersectionOperation => node,
  DifferenceOperation => node,
  ComplementOperation => node,
  SubsetOperation => node,
  SupersetOperation => node,
  ProperSubsetOperation => node,
  ProperSupersetOperation => node,
  ElementOfOperation => node,
  NotElementOfOperation => node,
  SymmetricDifferenceOperation => node,

  // Phase 2E closed module-import values. Keep this append-only: persisted
  // green-tree discriminants depend on every preceding kind.
  ModuleImportNameSegment => node,
  ModuleImportIntrinsicSegment => node,
  ModuleImportPathSegment => node,
  ModuleImportPath => node,
  ModuleImportAliasSegment => node,
  ModuleImportAliasPath => node,
  ModuleImportValueAlias => node,
  ContextImportAliasSegment => node,
  ModuleImportContextAlias => node,
  ModuleImportAlias => node,
  ModuleRoot => node,
  ImportGroupItem => node,
  ImportGroupItems => node,
  AliasedItemImport => node,
  ModuleSuffixImport => node,
  ModuleOnlyImport => node,
  ModuleImport => node,

  // Phase 2F source-import values. Keep this append-only: persisted green-tree
  // discriminants depend on every preceding kind.
  SourceImportTail => node,
  SourcePathComponent => node,
  SourceMecPath => node,
  RelativeSourceImportSpecifier => node,
  AbsoluteSourceImportSpecifier => node,
  BareSourceImportSpecifier => node,
  SourceImportUriScheme => node,
  UriSourceImportSpecifier => node,
  SourceImportSpecifier => node,
  ImportDeclaration => node,

  // Phase 2F declaration values follow the source-import tail. Keep this
  // append-only: persisted green-tree discriminants depend on every prior kind.
  ExportDeclaration => node,
  ContextDeclaration => node,
  ContextBaseContext => node,
  ContextBaseResourceUri => node,
  ContextCapabilityDeclaration => node,
  ContextCapabilityPath => node,
  ContextCapabilityScope => node,

  // Phase 2G closed executable primitives. Keep this append-only: persisted
  // green-tree discriminants depend on every preceding kind.
  SelectAllSubscript => node,
  SwizzleSubscript => node,
  DotSubscript => node,
  DotSubscriptInt => node,
  WildcardPattern => node,
  OpAssignOperator => node,
  AddAssignOperation => node,
  SubAssignOperation => node,
  MulAssignOperation => node,
  DivAssignOperation => node,
  ExpAssignOperation => node,

  // Phase 2H closed structure shell. Keep this append-only: persisted
  // green-tree discriminants depend on every preceding kind.
  TableRowSeparator => node,
  EmptyMap => node,
  EmptySet => node,

  // Phase 2I-B recursive-core syntax schema. Keep this append-only: persisted
  // green-tree discriminants depend on every preceding kind.
  Literal => node,
  Kind => node,
  KindWithOption => node,
  KindKind => node,
  KindTable => node,
  KindSet => node,
  KindMap => node,
  KindRecord => node,
  KindMatrix => node,
  KindTuple => node,
  KindScalar => node,
  Variable => node,
  Slice => node,
  SubscriptList => node,
  BracketSubscript => node,
  BraceSubscript => node,
  FormulaSubscript => node,
  RangeSubscript => node,
  Structure => node,
  Matrix => node,
  MatrixRow => node,
  MatrixColumn => node,
  Table => node,
  FancyTable => node,
  FancyTableHeader => node,
  FancyTableRow => node,
  InlineTable => node,
  InlineTableHeader => node,
  InlineTableRow => node,
  RegularTable => node,
  TableHeader => node,
  TableRow => node,
  HeaderField => node,
  TableField => node,
  Map => node,
  MapEntry => node,
  Record => node,
  RecordBinding => node,
  Set => node,
  Tuple => node,
  TupleStruct => node,
  FunctionCall => node,
  ArgumentList => node,
  CallArgument => node,
  BoundCallArgument => node,
  Pattern => node,
  ArrayPattern => node,
  ArrayPatternElement => node,
  AtomStructPattern => node,
  TuplePattern => node,
  TupleStructPattern => node,
  ComprehensionQualifier => node,
  Generator => node,
  SetComprehension => node,
  MatrixComprehension => node,
  FsmPipe => node,
  FsmInstance => node,
  FsmArguments => node,
  FsmValue => node,
  FsmStateTransition => node,
  FsmAsyncTransition => node,
  FsmOutput => node,
  Factor => node,
  NegateFactor => node,
  NotFactor => node,
  RangeExpression => node,
  MatchArm => node,
  LogicExpression => node,
  ComparisonExpression => node,
  MultiplicativeExpression => node,
  PowerExpression => node,
  TableExpression => node,
  SetExpression => node,
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
  fn phase_2d_kinds_are_append_only() {
    let appended = [
      SyntaxKind::AddSubOperator,
      SyntaxKind::MulDivOperator,
      SyntaxKind::PowerOperator,
      SyntaxKind::MatrixOperator,
      SyntaxKind::RangeOperator,
      SyntaxKind::ComparisonOperator,
      SyntaxKind::LogicOperator,
      SyntaxKind::TableOperator,
      SyntaxKind::SetOperator,
      SyntaxKind::AddOperation,
      SyntaxKind::SubtractOperation,
      SyntaxKind::RawSubtractOperation,
      SyntaxKind::SpacedSubtractOperation,
      SyntaxKind::MultiplyOperation,
      SyntaxKind::DivideOperation,
      SyntaxKind::ModulusOperation,
      SyntaxKind::PowerOperation,
      SyntaxKind::MatrixMultiplyOperation,
      SyntaxKind::MatrixSolveOperation,
      SyntaxKind::DotProductOperation,
      SyntaxKind::CrossProductOperation,
      SyntaxKind::RangeInclusiveOperation,
      SyntaxKind::RangeExclusiveOperation,
      SyntaxKind::NotEqualOperation,
      SyntaxKind::EqualToOperation,
      SyntaxKind::StrictNotEqualOperation,
      SyntaxKind::StrictEqualOperation,
      SyntaxKind::GreaterThanOperation,
      SyntaxKind::LessThanOperation,
      SyntaxKind::GreaterThanEqualOperation,
      SyntaxKind::LessThanEqualOperation,
      SyntaxKind::OrOperation,
      SyntaxKind::AndOperation,
      SyntaxKind::NotOperation,
      SyntaxKind::XorOperation,
      SyntaxKind::JoinOperation,
      SyntaxKind::LeftJoinOperation,
      SyntaxKind::RightJoinOperation,
      SyntaxKind::FullJoinOperation,
      SyntaxKind::LeftSemiJoinOperation,
      SyntaxKind::LeftAntiJoinOperation,
      SyntaxKind::UnionOperation,
      SyntaxKind::IntersectionOperation,
      SyntaxKind::DifferenceOperation,
      SyntaxKind::ComplementOperation,
      SyntaxKind::SubsetOperation,
      SyntaxKind::SupersetOperation,
      SyntaxKind::ProperSubsetOperation,
      SyntaxKind::ProperSupersetOperation,
      SyntaxKind::ElementOfOperation,
      SyntaxKind::NotElementOfOperation,
      SyntaxKind::SymmetricDifferenceOperation,
    ];
    for (offset, kind) in appended.into_iter().enumerate() {
      assert_eq!(kind as u16, 168 + offset as u16);
      assert!(!kind.is_token());
    }
  }

  #[test]
  fn phase_2e_kinds_are_append_only() {
    let appended = [
      SyntaxKind::ModuleImportNameSegment,
      SyntaxKind::ModuleImportIntrinsicSegment,
      SyntaxKind::ModuleImportPathSegment,
      SyntaxKind::ModuleImportPath,
      SyntaxKind::ModuleImportAliasSegment,
      SyntaxKind::ModuleImportAliasPath,
      SyntaxKind::ModuleImportValueAlias,
      SyntaxKind::ContextImportAliasSegment,
      SyntaxKind::ModuleImportContextAlias,
      SyntaxKind::ModuleImportAlias,
      SyntaxKind::ModuleRoot,
      SyntaxKind::ImportGroupItem,
      SyntaxKind::ImportGroupItems,
      SyntaxKind::AliasedItemImport,
      SyntaxKind::ModuleSuffixImport,
      SyntaxKind::ModuleOnlyImport,
      SyntaxKind::ModuleImport,
    ];
    for (offset, kind) in appended.into_iter().enumerate() {
      assert_eq!(kind as u16, 220 + offset as u16);
      assert!(!kind.is_token());
    }
  }

  #[test]
  fn phase_2f_source_import_kinds_are_append_only() {
    let appended = [
      SyntaxKind::SourceImportTail,
      SyntaxKind::SourcePathComponent,
      SyntaxKind::SourceMecPath,
      SyntaxKind::RelativeSourceImportSpecifier,
      SyntaxKind::AbsoluteSourceImportSpecifier,
      SyntaxKind::BareSourceImportSpecifier,
      SyntaxKind::SourceImportUriScheme,
      SyntaxKind::UriSourceImportSpecifier,
      SyntaxKind::SourceImportSpecifier,
      SyntaxKind::ImportDeclaration,
      SyntaxKind::ExportDeclaration,
      SyntaxKind::ContextDeclaration,
      SyntaxKind::ContextBaseContext,
      SyntaxKind::ContextBaseResourceUri,
      SyntaxKind::ContextCapabilityDeclaration,
      SyntaxKind::ContextCapabilityPath,
      SyntaxKind::ContextCapabilityScope,
    ];
    for (offset, kind) in appended.into_iter().enumerate() {
      assert_eq!(kind as u16, 237 + offset as u16);
      assert!(!kind.is_token());
    }
  }

  #[test]
  fn phase_2g_executable_primitive_kinds_are_append_only() {
    let appended = [
      SyntaxKind::SelectAllSubscript,
      SyntaxKind::SwizzleSubscript,
      SyntaxKind::DotSubscript,
      SyntaxKind::DotSubscriptInt,
      SyntaxKind::WildcardPattern,
      SyntaxKind::OpAssignOperator,
      SyntaxKind::AddAssignOperation,
      SyntaxKind::SubAssignOperation,
      SyntaxKind::MulAssignOperation,
      SyntaxKind::DivAssignOperation,
      SyntaxKind::ExpAssignOperation,
    ];
    for (offset, kind) in appended.into_iter().enumerate() {
      assert_eq!(kind as u16, 254 + offset as u16);
      assert!(!kind.is_token());
    }
  }

  #[test]
  fn phase_2h_structure_shell_kinds_are_append_only() {
    let appended = [
      SyntaxKind::TableRowSeparator,
      SyntaxKind::EmptyMap,
      SyntaxKind::EmptySet,
    ];
    for (offset, kind) in appended.into_iter().enumerate() {
      assert_eq!(kind as u16, 265 + offset as u16);
      assert!(!kind.is_token());
    }
  }

  #[test]
  fn phase_2i_recursive_schema_kinds_are_append_only() {
    let appended = [
      SyntaxKind::Literal,
      SyntaxKind::Kind,
      SyntaxKind::KindWithOption,
      SyntaxKind::KindKind,
      SyntaxKind::KindTable,
      SyntaxKind::KindSet,
      SyntaxKind::KindMap,
      SyntaxKind::KindRecord,
      SyntaxKind::KindMatrix,
      SyntaxKind::KindTuple,
      SyntaxKind::KindScalar,
      SyntaxKind::Variable,
      SyntaxKind::Slice,
      SyntaxKind::SubscriptList,
      SyntaxKind::BracketSubscript,
      SyntaxKind::BraceSubscript,
      SyntaxKind::FormulaSubscript,
      SyntaxKind::RangeSubscript,
      SyntaxKind::Structure,
      SyntaxKind::Matrix,
      SyntaxKind::MatrixRow,
      SyntaxKind::MatrixColumn,
      SyntaxKind::Table,
      SyntaxKind::FancyTable,
      SyntaxKind::FancyTableHeader,
      SyntaxKind::FancyTableRow,
      SyntaxKind::InlineTable,
      SyntaxKind::InlineTableHeader,
      SyntaxKind::InlineTableRow,
      SyntaxKind::RegularTable,
      SyntaxKind::TableHeader,
      SyntaxKind::TableRow,
      SyntaxKind::HeaderField,
      SyntaxKind::TableField,
      SyntaxKind::Map,
      SyntaxKind::MapEntry,
      SyntaxKind::Record,
      SyntaxKind::RecordBinding,
      SyntaxKind::Set,
      SyntaxKind::Tuple,
      SyntaxKind::TupleStruct,
      SyntaxKind::FunctionCall,
      SyntaxKind::ArgumentList,
      SyntaxKind::CallArgument,
      SyntaxKind::BoundCallArgument,
      SyntaxKind::Pattern,
      SyntaxKind::ArrayPattern,
      SyntaxKind::ArrayPatternElement,
      SyntaxKind::AtomStructPattern,
      SyntaxKind::TuplePattern,
      SyntaxKind::TupleStructPattern,
      SyntaxKind::ComprehensionQualifier,
      SyntaxKind::Generator,
      SyntaxKind::SetComprehension,
      SyntaxKind::MatrixComprehension,
      SyntaxKind::FsmPipe,
      SyntaxKind::FsmInstance,
      SyntaxKind::FsmArguments,
      SyntaxKind::FsmValue,
      SyntaxKind::FsmStateTransition,
      SyntaxKind::FsmAsyncTransition,
      SyntaxKind::FsmOutput,
      SyntaxKind::Factor,
      SyntaxKind::NegateFactor,
      SyntaxKind::NotFactor,
      SyntaxKind::RangeExpression,
      SyntaxKind::MatchArm,
      SyntaxKind::LogicExpression,
      SyntaxKind::ComparisonExpression,
      SyntaxKind::MultiplicativeExpression,
      SyntaxKind::PowerExpression,
      SyntaxKind::TableExpression,
      SyntaxKind::SetExpression,
    ];
    for (offset, kind) in appended.into_iter().enumerate() {
      assert_eq!(kind as u16, 268 + offset as u16);
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
