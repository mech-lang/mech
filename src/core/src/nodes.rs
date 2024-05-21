use crate::*;

use std::cmp::Ordering;

#[derive(Clone, Copy, Ord, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceLocation {
  pub row: usize,
  pub col: usize,
}

impl PartialOrd for SourceLocation {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    if self.row < other.row {
      Some(Ordering::Less)
    } else if self.row > other.row {
      Some(Ordering::Greater)
    } else {
      self.col.partial_cmp(&other.col)
    }
  }
}

impl fmt::Debug for SourceLocation {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}:{}", self.row, self.col);
    Ok(())
  }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceRange {
  pub start: SourceLocation,
  pub end:   SourceLocation,
}

/// Coordinates in SourceRange are 1-indexed, i.e. they directly translate
/// human's view to line and column numbers.  Having value 0 means the 
/// range is not initialized.
impl Default for SourceRange {
  fn default() -> Self {
    SourceRange {
      start: SourceLocation { row: 0, col: 0 },
      end:   SourceLocation { row: 0, col: 0 },
    }
  }
}

impl fmt::Debug for SourceRange {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "[{:?}, {:?})", self.start, self.end);
    Ok(())
  }
}

pub fn merge_src_range(r1: SourceRange, r2: SourceRange) -> SourceRange {
  SourceRange {
    start: r1.start.min(r2.start),
    end:   r2.end.max(r2.end),
  }
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AstNode {
  Root                    { children: Vec<AstNode> },
  Fragment                { children: Vec<AstNode> },
  Head                    { children: Vec<AstNode> },
  Body                    { children: Vec<AstNode> },
  Block                   { children: Vec<AstNode>, src_range: SourceRange },
  Statement               { children: Vec<AstNode>, src_range: SourceRange },
  Expression              { children: Vec<AstNode> },
  MathExpression          { children: Vec<AstNode> },
  SelectExpression        { children: Vec<AstNode> },
  Data                    { children: Vec<AstNode> },
  Whenever                { children: Vec<AstNode> },
  Wait                    { children: Vec<AstNode> },
  Until                   { children: Vec<AstNode> },
  SetData                 { children: Vec<AstNode> },
  SplitData               { children: Vec<AstNode> },
  FlattenData             { children: Vec<AstNode> },
  Binding                 { children: Vec<AstNode> },
  FunctionBinding         { children: Vec<AstNode> },
  UserFunction            { children: Vec<AstNode> },
  FunctionBody            { children: Vec<AstNode> },
  FunctionArgs            { children: Vec<AstNode> },
  FunctionInput           { children: Vec<AstNode> },
  FunctionOutput          { children: Vec<AstNode> },
  DotIndex                { children: Vec<AstNode> },
  Swizzle                 { children: Vec<AstNode> },
  SubscriptIndex          { children: Vec<AstNode> },
  Range,
  VariableDefine          { children: Vec<AstNode> },
  TableDefine             { children: Vec<AstNode> },
  FollowedBy              { children: Vec<AstNode> },
  AsyncAssign             { children: Vec<AstNode> },
  TableSelect             { children: Vec<AstNode> },
  EmptyTable              { children: Vec<AstNode> },
  InlineTable             { children: Vec<AstNode> },
  TableHeader             { children: Vec<AstNode> },
  Attribute               { children: Vec<AstNode> },
  TableRow                { children: Vec<AstNode> },
  Comment                 { children: Vec<AstNode> },
  KindAnnotation          { children: Vec<AstNode> },
  AddRow                  { children: Vec<AstNode> },
  Transformation          { children: Vec<AstNode> },
  Quantity                { children: Vec<AstNode> },
  Token                   { token: TokenKind, chars: Vec<char>, src_range: SourceRange },
  Add,
  Subtract,
  Multiply,
  MatrixMultiply,
  Divide,
  Exponent,
  LessThan,
  GreaterThan,
  GreaterThanEqual,
  LessThanEqual,
  Equal,
  NotEqual,
  And,
  Or,
  Xor,
  AddUpdate,
  SubtractUpdate,
  MultiplyUpdate,
  DivideUpdate,
  ExponentUpdate,
  SelectAll,
  Empty,
  True,
  False,
  ReshapeColumn,
  RationalNumber          { children: Vec<AstNode> },
  Paragraph               { children: Vec<AstNode> },
  UnorderedList           { children: Vec<AstNode> },
  ListItem                { children: Vec<AstNode> },
  InlineCode              { children: Vec<AstNode> },
  CodeBlock               { children: Vec<AstNode> },
  InlineMechCode          { children: Vec<AstNode> },
  MechCodeBlock           { children: Vec<AstNode> },
  Null,
  Transpose,
  Program                 { title: Option<Vec<char>>, children: Vec<AstNode> },
  Section                 { title: Option<Vec<char>>, level: usize, children: Vec<AstNode> },
  WheneverIndex           { children: Vec<AstNode> },
  SelectData              { name: Vec<char>, id: TableId, children: Vec<AstNode>, src_range: SourceRange },
  UpdateData              { name: Vec<char>, children: Vec<AstNode>, src_range: SourceRange },
  TableColumn             { children: Vec<AstNode> },
  Function                { name: Vec<char>, children: Vec<AstNode>, src_range: SourceRange },
  Define                  { name: Vec<char>, id: u64, src_range: SourceRange },
  AnonymousTableDefine    { children: Vec<AstNode> },
  AnonymousMatrixDefine   { children: Vec<AstNode> },
  Identifier              { name: Vec<char>, id: u64, src_range: SourceRange },
  Table                   { name: Vec<char>, id: u64, src_range: SourceRange },
  String                  { text: Vec<char>, src_range: SourceRange },
  NumberLiteral           { kind: u64, bytes: Vec<char>, src_range: SourceRange },
  SectionTitle            { text: Vec<char>, src_range: SourceRange },
  Title                   { text: Vec<char>, level: usize, src_range: SourceRange },
  ParagraphText           { text: Vec<char>, src_range: SourceRange },
  TransposeSelect         { children: Vec<AstNode> },
  Incomplete              { children: Vec<AstNode>, src_range: SourceRange },
  Error,
}

impl fmt::Debug for AstNode {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    print_ast_node(self, 1, f);
    Ok(())
  }
}

pub fn print_ast_node(node: &AstNode, level: usize, f: &mut fmt::Formatter) {
  spacer(level, f);
  let children: Option<&Vec<AstNode>> = match node {
    AstNode::Root{children}                            => { write!(f, "Root\n").ok(); Some(children) },
    AstNode::Fragment{children}                        => { write!(f, "Fragment\n").ok(); Some(children) },
    AstNode::Program{title, children}                  => { write!(f, "Program({:?})\n", title).ok(); Some(children) },
    AstNode::Head{children}                            => { write!(f, "Head\n").ok(); Some(children) },
    AstNode::Body{children}                            => { write!(f, "Body\n").ok(); Some(children) },
    AstNode::VariableDefine{children}                  => { write!(f, "VariableDefine\n").ok(); Some(children) },
    AstNode::TableColumn{children}                     => { write!(f, "TableColumn\n").ok(); Some(children) },
    AstNode::Binding{children}                         => { write!(f, "Binding\n").ok(); Some(children) },
    AstNode::FunctionBinding{children}                 => { write!(f, "FunctionBinding\n").ok(); Some(children) },
    AstNode::TableDefine{children}                     => { write!(f, "TableDefine\n").ok(); Some(children) },
    AstNode::FollowedBy{children}                      => { write!(f, "FollowedBy\n").ok(); Some(children) },
    AstNode::AsyncAssign{children}                     => { write!(f, "AsyncAssign\n").ok(); Some(children) },
    AstNode::TableSelect{children}                     => { write!(f, "TableSelect\n").ok(); Some(children) },
    AstNode::AnonymousTableDefine{children}            => { write!(f, "AnonymousTableDefine\n").ok(); Some(children) },
    AstNode::EmptyTable{children}                      => { write!(f, "EmptyTable\n").ok(); Some(children) },
    AstNode::InlineTable{children}                     => { write!(f, "InlineTable\n").ok(); Some(children) },
    AstNode::TableHeader{children}                     => { write!(f, "TableHeader\n").ok(); Some(children) },
    AstNode::Attribute{children}                       => { write!(f, "Attribute\n").ok(); Some(children) },
    AstNode::TableRow{children}                        => { write!(f, "TableRow\n").ok(); Some(children) },
    AstNode::AddRow{children}                          => { write!(f, "AddRow\n").ok(); Some(children) },
    AstNode::Section{title, level, children}           => { write!(f, "Section({:?} {})\n", title, level).ok(); Some(children)},
    AstNode::Block{children, src_range}                => { write!(f, "Block(@ {:?})\n", src_range).ok(); Some(children) },
    AstNode::Statement{children, src_range}            => { write!(f, "Statement(@ {:?})\n", src_range).ok(); Some(children) },
    AstNode::SetData{children}                         => { write!(f, "SetData\n").ok(); Some(children) },
    AstNode::UpdateData{name, children, src_range}     => { write!(f, "UpdateData({:?} @ {:?})\n", name, src_range).ok(); Some(children) },
    AstNode::SplitData{children}                       => { write!(f, "SplitData\n").ok(); Some(children) },
    AstNode::FlattenData{children}                     => { write!(f, "FlattenData\n").ok(); Some(children) },
    AstNode::Data{children}                            => { write!(f, "Data\n").ok(); Some(children) },
    AstNode::KindAnnotation{children}                  => { write!(f, "KindAnnotation\n").ok(); Some(children) },
    AstNode::Whenever{children}                        => { write!(f, "Whenever\n").ok(); Some(children) },
    AstNode::WheneverIndex{children}                   => { write!(f, "WheneverIndex\n").ok(); Some(children) },
    AstNode::Wait{children}                            => { write!(f, "Wait\n").ok(); Some(children) },
    AstNode::Until{children}                           => { write!(f, "Until\n").ok(); Some(children) },
    AstNode::SelectData{name, id, children, src_range} => { write!(f, "SelectData({:?} {:?} @ {:?}))\n", name, id, src_range).ok(); Some(children) },
    AstNode::DotIndex{children}                        => { write!(f, "DotIndex\n").ok(); Some(children) },
    AstNode::Swizzle{children}                         => { write!(f, "Swizzle\n").ok(); Some(children) },
    AstNode::SubscriptIndex{children}                  => { write!(f, "SubscriptIndex\n").ok(); Some(children) },
    AstNode::Range                                     => { write!(f, "Range\n").ok(); None },
    AstNode::Expression{children}                      => { write!(f, "Expression\n").ok(); Some(children) },
    AstNode::Function{name, children, src_range}       => { write!(f, "Function({:?} @ {:?})\n", name, src_range).ok(); Some(children) },
    AstNode::UserFunction{children}                    => { write!(f, "UserFunction\n").ok(); Some(children) },
    AstNode::FunctionBody{children}                    => { write!(f, "FunctionBody\n").ok(); Some(children) },
    AstNode::FunctionArgs{children}                    => { write!(f, "FunctionArgs\n").ok(); Some(children) },
    AstNode::UserFunction{children}                    => { write!(f, "Expression\n").ok(); Some(children) },
    AstNode::FunctionInput{children}                   => { write!(f, "FunctionInput\n").ok(); Some(children) },
    AstNode::FunctionOutput{children}                  => { write!(f, "FunctionOutput\n").ok(); Some(children) },
    AstNode::MathExpression{children}                  => { write!(f, "MathExpression\n").ok(); Some(children) },
    AstNode::Comment{children}                         => { write!(f, "Comment\n").ok(); Some(children) },
    AstNode::SelectExpression{children}                => { write!(f, "SelectExpression\n").ok(); Some(children) },
    AstNode::Transformation{children}                  => { write!(f, "Transformation\n").ok(); Some(children) },
    AstNode::Identifier{name, id, src_range}           => { write!(f, "Identifier({:?}({}) @ {:?})\n", name, humanize(id), src_range).ok(); None },
    AstNode::String{text, src_range}                   => { write!(f, "String({:?} @ {:?})\n", text, src_range).ok(); None },
    AstNode::RationalNumber{children}                  => { write!(f, "RationalNumber\n").ok(); Some(children) },
    AstNode::NumberLiteral{kind, bytes, src_range}     => { write!(f, "NumberLiteral({:?} @ {:?})\n", bytes, src_range).ok(); None },
    AstNode::Quantity{children}                        => { write!(f, "Quantity\n").ok(); Some(children) },
    AstNode::Table{name, id, src_range}                => { write!(f, "Table(#{:?}({:#x}) @ {:?})\n", name, id, src_range).ok(); None },
    AstNode::Define{name, id, src_range}               => { write!(f, "Define(#{:?}({:?}) @ {:?})\n", name, id, src_range).ok(); None },
    AstNode::Token{token, chars, src_range}            => { write!(f, "Token({:?} @ {:?})\n", token, src_range).ok(); None },
    AstNode::SelectAll                                 => { write!(f, "SelectAll\n").ok(); None },
    AstNode::LessThan                                  => { write!(f, "LessThan\n").ok(); None },
    AstNode::GreaterThan                               => { write!(f, "GreaterThan\n").ok(); None },
    AstNode::GreaterThanEqual                          => { write!(f, "GreaterThanEqual\n").ok(); None },
    AstNode::LessThanEqual                             => { write!(f, "LessThanEqual\n").ok(); None },
    AstNode::Equal                                     => { write!(f, "Equal\n").ok(); None },
    AstNode::NotEqual                                  => { write!(f, "NotEqual\n").ok(); None },
    AstNode::Empty                                     => { write!(f, "Empty\n").ok(); None },
    AstNode::True                                      => { write!(f, "True\n").ok(); None },
    AstNode::False                                     => { write!(f, "False\n").ok(); None },
    AstNode::Null                                      => { write!(f, "Null\n").ok(); None },
    AstNode::ReshapeColumn                             => { write!(f, "ReshapeColumn\n").ok(); None },
    AstNode::Add                                       => { write!(f, "Add\n").ok(); None },
    AstNode::Subtract                                  => { write!(f, "Subtract\n").ok(); None },
    AstNode::Multiply                                  => { write!(f, "Multiply\n").ok(); None },
    AstNode::MatrixMultiply                            => { write!(f, "MatrixMultiply\n").ok(); None },
    AstNode::Divide                                    => { write!(f, "Divide\n").ok(); None },
    AstNode::Exponent                                  => { write!(f, "Exponent\n").ok(); None },
    AstNode::AddUpdate                                 => { write!(f, "AddUpdate\n").ok(); None },
    AstNode::SubtractUpdate                            => { write!(f, "SubtractUpdate\n").ok(); None },
    AstNode::MultiplyUpdate                            => { write!(f, "MultiplyUpdate\n").ok(); None },
    AstNode::DivideUpdate                              => { write!(f, "DivideUpdate\n").ok(); None },
    AstNode::ExponentUpdate                            => { write!(f, "ExponentUpdate\n").ok(); None },
    AstNode::Transpose                                 => { write!(f, "Transpose\n").ok(); None },
    AstNode::TransposeSelect{children}                 => { write!(f, "TransposeSelect\n").ok(); Some(children) },
    AstNode::Title{text, level, src_range}             => { write!(f, "Title({:?} {} @ {:?})\n", text, level, src_range).ok(); None },
    AstNode::ParagraphText{text, src_range}            => { write!(f, "ParagraphText({:?} @ {:?})\n", text, src_range).ok(); None },
    AstNode::UnorderedList{children}                   => { write!(f, "UnorderedList\n").ok(); Some(children) },
    AstNode::ListItem{children}                        => { write!(f, "ListItem\n").ok(); Some(children) },
    AstNode::Paragraph{children}                       => { write!(f, "Paragraph\n").ok(); Some(children) },
    AstNode::InlineCode{children}                      => { write!(f, "InlineCode\n").ok(); Some(children) },
    AstNode::CodeBlock{children}                       => { write!(f, "CodeBlock\n").ok(); Some(children) },
    AstNode::InlineMechCode{children}                  => { write!(f, "InlineMechCode\n").ok(); Some(children) },
    AstNode::MechCodeBlock{children}                   => { write!(f, "MechCodeBlock\n").ok(); Some(children) },
    AstNode::Incomplete{children, src_range}           => { write!(f, "Incomplete(@ {:?})\n", src_range).ok(); Some(children) },
    AstNode::Error                                     => { write!(f, "Error\n").ok(); None },
    _                                                  => { write!(f, "Unhandled Compiler Node").ok(); None },
  };

  match children {
    Some(childs) => {
      for child in childs {
        print_ast_node(child, level + 1,f)
      }
    },
    _ => (),
  }
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ParserNode {
  Root                    { children: Vec<ParserNode> },
  Fragment                { children: Vec<ParserNode> },
  Head                    { children: Vec<ParserNode> },
  Body                    { children: Vec<ParserNode> },
  Block                   { children: Vec<ParserNode>, src_range: SourceRange },
  Statement               { children: Vec<ParserNode>, src_range: SourceRange },
  Expression              { children: Vec<ParserNode> },
  MathExpression          { children: Vec<ParserNode> },
  SelectExpression        { children: Vec<ParserNode> },
  Data                    { children: Vec<ParserNode> },
  Whenever                { children: Vec<ParserNode> },
  Wait                    { children: Vec<ParserNode> },
  Until                   { children: Vec<ParserNode> },
  SetData                 { children: Vec<ParserNode> },
  SplitData               { children: Vec<ParserNode> },
  FlattenData             { children: Vec<ParserNode> },
  Binding                 { children: Vec<ParserNode> },
  FunctionBinding         { children: Vec<ParserNode> },
  UserFunction            { children: Vec<ParserNode> },
  FunctionBody            { children: Vec<ParserNode> },
  FunctionArgs            { children: Vec<ParserNode> },
  FunctionInput           { children: Vec<ParserNode> },
  FunctionOutput          { children: Vec<ParserNode> },
  DotIndex                { children: Vec<ParserNode> },
  Swizzle                 { children: Vec<ParserNode> },
  SubscriptIndex          { children: Vec<ParserNode> },
  Range,
  VariableDefine          { children: Vec<ParserNode> },
  EnumDefine              { children: Vec<ParserNode> },
  TableDefine             { children: Vec<ParserNode> },
  FollowedBy              { children: Vec<ParserNode> },
  AsyncAssign             { children: Vec<ParserNode> },
  TableSelect             { children: Vec<ParserNode> },
  EmptyTable              { children: Vec<ParserNode> },
  InlineTable             { children: Vec<ParserNode> },
  TableHeader             { children: Vec<ParserNode> },
  Attribute               { children: Vec<ParserNode> },
  TableRow                { children: Vec<ParserNode> },
  Comment                 { children: Vec<ParserNode> },
  KindAnnotation          { children: Vec<ParserNode> },
  AddRow                  { children: Vec<ParserNode> },
  Transformation          { children: Vec<ParserNode> },
  Quantity                { children: Vec<ParserNode> },
  Token                   { token: TokenKind, chars: Vec<char>, src_range: SourceRange },
  Add,
  Subtract,
  Multiply,
  MatrixMultiply,
  Divide,
  Exponent,
  LessThan,
  GreaterThan,
  GreaterThanEqual,
  LessThanEqual,
  Equal,
  NotEqual,
  And,
  Or,
  Xor,
  AddUpdate,
  SubtractUpdate,
  MultiplyUpdate,
  DivideUpdate,
  ExponentUpdate,
  SelectAll,
  Empty,
  True,
  False,
  ReshapeColumn,
  RationalNumber          { children: Vec<ParserNode> },
  Paragraph               { children: Vec<ParserNode> },
  UnorderedList           { children: Vec<ParserNode> },
  ListItem                { children: Vec<ParserNode> },
  InlineCode              { children: Vec<ParserNode> },
  CodeBlock               { children: Vec<ParserNode> },
  InlineMechCode          { children: Vec<ParserNode> },
  MechCodeBlock           { children: Vec<ParserNode> },
  Null,
  Transpose,
  Select                  { children: Vec<ParserNode> },
  Insert                  { children: Vec<ParserNode> },
  Column                  { children: Vec<ParserNode> },
  IdentifierOrConstant    { children: Vec<ParserNode> },
  Table                   { children: Vec<ParserNode> },
  Number                  { children: Vec<ParserNode> },
  DigitOrComma            { children: Vec<ParserNode> },
  FilterExpression        { children: Vec<ParserNode> },
  Comparator              { children: Vec<ParserNode> },
  InfixOperation          { children: Vec<ParserNode> },
  Repeat                  { children: Vec<ParserNode> },
  TableIdentifier         { children: Vec<ParserNode> },
  Identifier              { children: Vec<ParserNode> },
  Alpha                   { children: Vec<ParserNode> },
  SubscriptList           { children: Vec<ParserNode> },
  Subscript               { children: Vec<ParserNode> },
  LogicOperator           { children: Vec<ParserNode> },
  LogicExpression         { children: Vec<ParserNode> },
  Index                   { children: Vec<ParserNode> },
  UpdateData              { children: Vec<ParserNode> },
  SetOperator             { children: Vec<ParserNode> },
  AddOperator             { children: Vec<ParserNode> },
  WatchOperator           { children: Vec<ParserNode> },
  Equality                { children: Vec<ParserNode> },
  AnonymousTable          { children: Vec<ParserNode> },
  AnonymousMatrix         { children: Vec<ParserNode> },
  Constant                { children: Vec<ParserNode> },
  Infix                   { children: Vec<ParserNode> },
  Program                 { children: Vec<ParserNode> },
  Title                   { children: Vec<ParserNode> },
  Subtitle                { children: Vec<ParserNode>, level: usize },
  StatementOrExpression   { children: Vec<ParserNode> },
  DataOrConstant          { children: Vec<ParserNode> },
  IdentifierCharacter     { children: Vec<ParserNode> },
  Node                    { children: Vec<ParserNode> },
  NewLineOrEnd            { children: Vec<ParserNode> },
  Alphanumeric            { children: Vec<ParserNode> },
  ParagraphText           { children: Vec<ParserNode> },
  FormattedText           { children: Vec<ParserNode> },
  Bold                    { children: Vec<ParserNode> },
  Italic                  { children: Vec<ParserNode> },
  Hyperlink               { children: Vec<ParserNode> },
  BlockQuote              { children: Vec<ParserNode> },
  String                  { children: Vec<ParserNode> },
  StringInterpolation     { children: Vec<ParserNode> },
  Word                    { children: Vec<ParserNode> },
  Emoji                   { children: Vec<ParserNode> },
  Section                 { children: Vec<ParserNode>, title: Option<Vec<ParserNode>> },
  ProseOrCode             { children: Vec<ParserNode> },
  Whitespace              { children: Vec<ParserNode> },
  SpaceOrTab              { children: Vec<ParserNode> },
  NewLine                 { children: Vec<ParserNode> },
  Text                    { children: Vec<ParserNode> },
  Punctuation             { children: Vec<ParserNode> },
  L0Infix                 { children: Vec<ParserNode> },
  L1Infix                 { children: Vec<ParserNode> },
  L2Infix                 { children: Vec<ParserNode> },
  L3Infix                 { children: Vec<ParserNode> },
  L4Infix                 { children: Vec<ParserNode> },
  L5Infix                 { children: Vec<ParserNode> },
  L0                      { children: Vec<ParserNode> },
  L1                      { children: Vec<ParserNode> },
  L2                      { children: Vec<ParserNode> },
  L3                      { children: Vec<ParserNode> },
  L4                      { children: Vec<ParserNode> },
  L5                      { children: Vec<ParserNode> },
  L6                      { children: Vec<ParserNode> },
  Function                { children: Vec<ParserNode> },
  Negation                { children: Vec<ParserNode> },
  Not                     { children: Vec<ParserNode> },
  ParentheticalExpression { children: Vec<ParserNode> },
  CommentSigil            { children: Vec<ParserNode> },
  Any                     { children: Vec<ParserNode> },
  Symbol                  { children: Vec<ParserNode> },
  StateMachine            { children: Vec<ParserNode> },
  StateTransition         { children: Vec<ParserNode> },
  Value                   { children: Vec<ParserNode> },
  BooleanLiteral          { children: Vec<ParserNode> },
  NumberLiteral           { children: Vec<ParserNode> },
  FloatLiteral            { chars: Vec<char>, src_range: SourceRange },
  DecimalLiteral          { chars: Vec<char>, src_range: SourceRange },
  HexadecimalLiteral      { chars: Vec<char>, src_range: SourceRange },
  OctalLiteral            { chars: Vec<char>, src_range: SourceRange },
  BinaryLiteral           { chars: Vec<char>, src_range: SourceRange },
  Incomplete              { children: Vec<ParserNode>, src_range: SourceRange },
  Error,
}

impl fmt::Debug for ParserNode {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    print_parser_node(self, 1, f);
    Ok(())
  }
}

fn print_parser_node(node: &ParserNode, level: usize, f: &mut fmt::Formatter) {
  spacer(level, f);
  let children: Option<&Vec<ParserNode>> = match node {
    ParserNode::Root{children}                       => { write!(f, "Root\n"); Some(children) },
    ParserNode::Block{children, src_range}           => { write!(f, "Block(@ {:?})\n", src_range); Some(children) },
    ParserNode::Transformation{children}             => { write!(f, "Transformation\n"); Some(children) },
    ParserNode::Select{children}                     => { write!(f, "Select\n"); Some(children) },
    ParserNode::Whenever{children}                   => { write!(f, "Whenever\n"); Some(children) },
    ParserNode::Insert{children}                     => { write!(f, "Insert\n"); Some(children) },
    ParserNode::MathExpression{children}             => { write!(f, "MathExpression\n"); Some(children) },
    ParserNode::SelectExpression{children}           => { write!(f, "SelectExpression\n"); Some(children) },
    ParserNode::Comparator{children}                 => { write!(f, "Comparator\n"); Some(children) },
    ParserNode::FilterExpression{children}           => { write!(f, "FilterExpression\n"); Some(children) },
    ParserNode::AnonymousTable{children}             => { write!(f, "AnonymousTable\n"); Some(children) },
    ParserNode::EmptyTable{children}                 => { write!(f, "EmptyTable\n"); Some(children) },
    ParserNode::AnonymousMatrix{children}            => { write!(f, "AnonymousMatrix\n"); Some(children) },
    ParserNode::TableRow{children}                   => { write!(f, "TableRow\n"); Some(children) },
    ParserNode::Table{children}                      => { write!(f, "Table\n"); Some(children) },
    ParserNode::Number{children}                     => { write!(f, "Number\n"); Some(children) },
    ParserNode::DigitOrComma{children}               => { write!(f, "DigitOrComma\n"); Some(children) },
    ParserNode::Alphanumeric{children}               => { write!(f, "Alphanumeric\n"); Some(children) },
    ParserNode::Word{children}                       => { write!(f, "Word\n"); Some(children) },
    ParserNode::Emoji{children}                      => { write!(f, "Emoji\n"); Some(children) },
    ParserNode::Paragraph{children}                  => { write!(f, "Paragraph\n"); Some(children) },
    ParserNode::ParagraphText{children}              => { write!(f, "ParagraphText\n"); Some(children) },
    ParserNode::FormattedText{children}              => { write!(f, "FormattedText\n"); Some(children) },
    ParserNode::InlineMechCode{children}             => { write!(f, "InlineMechCode\n"); Some(children) },
    ParserNode::InlineCode{children}                 => { write!(f, "InlineCode\n"); Some(children) },
    ParserNode::MechCodeBlock{children}              => { write!(f, "MechCodeBlock\n"); Some(children) },
    ParserNode::Bold{children}                       => { write!(f, "Bold\n"); Some(children) },
    ParserNode::Italic{children}                     => { write!(f, "Italic\n"); Some(children) },
    ParserNode::Hyperlink{children}                  => { write!(f, "Hyperlink\n"); Some(children) },
    ParserNode::BlockQuote{children}                 => { write!(f, "BlockQuote\n"); Some(children) },
    ParserNode::CodeBlock{children}                  => { write!(f, "CodeBlock\n"); Some(children) },
    ParserNode::UnorderedList{children}              => { write!(f, "UnorderedList\n"); Some(children) },
    ParserNode::ListItem{children}                   => { write!(f, "ListItem\n"); Some(children) },
    ParserNode::String{children}                     => { write!(f, "String\n"); Some(children) },
    ParserNode::StringInterpolation{children}        => { write!(f, "StringInterpolation\n"); Some(children) },
    ParserNode::VariableDefine{children}             => { write!(f, "VariableDefine\n"); Some(children) },
    ParserNode::EnumDefine{children}                 => { write!(f, "EnumDefine\n"); Some(children) },
    ParserNode::TableDefine{children}                => { write!(f, "TableDefine\n"); Some(children) },
    ParserNode::FollowedBy{children}                 => { write!(f, "FollowedBy\n"); Some(children) },
    ParserNode::AsyncAssign{children}                => { write!(f, "AsyncAssign\n"); Some(children) },
    ParserNode::TableSelect{children}                => { write!(f, "TableSelect\n"); Some(children) },
    ParserNode::AddRow{children}                     => { write!(f, "AddRow\n"); Some(children) },
    ParserNode::Column{children}                     => { write!(f, "Column\n"); Some(children) },
    ParserNode::Binding{children}                    => { write!(f, "Binding\n"); Some(children) },
    ParserNode::FunctionBinding{children}            => { write!(f, "FunctionBinding\n"); Some(children) },
    ParserNode::InlineTable{children}                => { write!(f, "InlineTable\n"); Some(children) },
    ParserNode::TableHeader{children}                => { write!(f, "TableHeader\n"); Some(children) },
    ParserNode::Attribute{children}                  => { write!(f, "Attribute\n"); Some(children) },
    ParserNode::IdentifierOrConstant{children}       => { write!(f, "IdentifierOrConstant\n"); Some(children) },
    ParserNode::InfixOperation{children}             => { write!(f, "Infix\n"); Some(children) },
    ParserNode::Repeat{children}                     => { write!(f, "Repeat\n"); Some(children) },
    ParserNode::Identifier{children}                 => { write!(f, "Identifier\n"); Some(children) },
    ParserNode::TableIdentifier{children}            => { write!(f, "TableIdentifier\n"); Some(children) },
    ParserNode::DotIndex{children}                   => { write!(f, "DotIndex\n"); Some(children) },
    ParserNode::Swizzle{children}                    => { write!(f, "Swizzle\n"); Some(children) },
    ParserNode::SubscriptIndex{children}             => { write!(f, "SubscriptIndex\n"); Some(children) },
    ParserNode::SubscriptList{children}              => { write!(f, "SubscriptList\n"); Some(children) },
    ParserNode::Subscript{children}                  => { write!(f, "Subscript\n"); Some(children) },
    ParserNode::LogicOperator{children}              => { write!(f, "LogicOperator\n"); Some(children) },
    ParserNode::LogicExpression{children}            => { write!(f, "LogicExpression\n"); Some(children) },
    ParserNode::Range                                => { write!(f, "Range\n"); None },
    ParserNode::SelectAll                            => { write!(f, "SelectAll\n"); None },
    ParserNode::Index{children}                      => { write!(f, "Index\n"); Some(children) },
    ParserNode::Equality{children}                   => { write!(f, "Equality\n"); Some(children) },
    ParserNode::Data{children}                       => { write!(f, "Data\n"); Some(children) },
    ParserNode::SetData{children}                    => { write!(f, "SetData\n"); Some(children) },
    ParserNode::UpdateData{children}                 => { write!(f, "UpdateData\n"); Some(children) },
    ParserNode::SplitData{children}                  => { write!(f, "SplitData\n"); Some(children) },
    ParserNode::FlattenData{children}                => { write!(f, "FlattenData\n"); Some(children) },
    ParserNode::Wait{children}                       => { write!(f, "Wait\n"); Some(children) },
    ParserNode::Until{children}                      => { write!(f, "Until\n"); Some(children) },
    ParserNode::SetOperator{children}                => { write!(f, "SetOperator\n"); Some(children) },
    ParserNode::AddOperator{children}                => { write!(f, "AddOperator\n"); Some(children) },
    ParserNode::WatchOperator{children}              => { write!(f, "WatchOperator\n"); Some(children) },
    ParserNode::Infix{children}                      => { write!(f, "Infix\n"); Some(children) },
    ParserNode::Expression{children}                 => { write!(f, "Expression\n"); Some(children) },
    ParserNode::Constant{children}                   => { write!(f, "Constant\n"); Some(children) },
    ParserNode::Program{children}                    => { write!(f, "Program\n"); Some(children) },
    ParserNode::IdentifierCharacter{children}        => { write!(f, "IdentifierCharacter\n"); Some(children) },
    ParserNode::Title{children}                      => { write!(f, "Title\n"); Some(children) },
    ParserNode::Subtitle{children, level}            => { write!(f, "Subtitle {}\n", level); Some(children) },
    ParserNode::Section{title, children}             => { write!(f, "Section {:?}\n", title); Some(children) },
    ParserNode::Statement{children, src_range}       => { write!(f, "Statement(@ {:?})\n", src_range); Some(children) },
    ParserNode::StatementOrExpression{children}      => { write!(f, "StatementOrExpression\n"); Some(children) },
    ParserNode::DataOrConstant{children}             => { write!(f, "DataOrConstant\n"); Some(children) },
    ParserNode::NewLineOrEnd{children}               => { write!(f, "NewLineOrEnd\n"); Some(children) },
    ParserNode::Fragment{children}                   => { write!(f, "Fragment\n"); Some(children) },
    ParserNode::Body{children}                       => { write!(f, "Body\n"); Some(children) },
    ParserNode::Head{children}                       => { write!(f, "Head\n"); Some(children) },
    ParserNode::Node{children}                       => { write!(f, "Node\n"); Some(children) },
    ParserNode::Text{children}                       => { write!(f, "Text\n"); Some(children) },
    ParserNode::Punctuation{children}                => { write!(f, "Punctuation\n"); Some(children) },
    ParserNode::L0Infix{children}                    => { write!(f, "L0Infix\n"); Some(children) },
    ParserNode::L1Infix{children}                    => { write!(f, "L1Infix\n"); Some(children) },
    ParserNode::L2Infix{children}                    => { write!(f, "L2Infix\n"); Some(children) },
    ParserNode::L3Infix{children}                    => { write!(f, "L3Infix\n"); Some(children) },
    ParserNode::L4Infix{children}                    => { write!(f, "L4Infix\n"); Some(children) },
    ParserNode::L5Infix{children}                    => { write!(f, "L5Infix\n"); Some(children) },
    ParserNode::L0{children}                         => { write!(f, "L0\n"); Some(children) },
    ParserNode::L1{children}                         => { write!(f, "L1\n"); Some(children) },
    ParserNode::L2{children}                         => { write!(f, "L2\n"); Some(children) },
    ParserNode::L3{children}                         => { write!(f, "L3\n"); Some(children) },
    ParserNode::L4{children}                         => { write!(f, "L4\n"); Some(children) },
    ParserNode::L5{children}                         => { write!(f, "L5\n"); Some(children) },
    ParserNode::L6{children}                         => { write!(f, "L6\n"); Some(children) },
    ParserNode::Function{children}                   => { write!(f, "Function\n"); Some(children) },
    ParserNode::UserFunction{children}               => { write!(f, "UserFunction\n"); Some(children) },
    ParserNode::FunctionBody{children}               => { write!(f, "FunctionBody\n"); Some(children) },
    ParserNode::FunctionArgs{children}               => { write!(f, "FunctionArgs\n"); Some(children) },
    ParserNode::FunctionInput{children}              => { write!(f, "FunctionInput\n"); Some(children) },
    ParserNode::FunctionOutput{children}             => { write!(f, "FunctionOutput\n"); Some(children) },
    ParserNode::Negation{children}                   => { write!(f, "Negation\n"); Some(children) },
    ParserNode::Not{children}                        => { write!(f, "Not\n"); Some(children) },
    ParserNode::ParentheticalExpression{children}    => { write!(f, "ParentheticalExpression\n"); Some(children) },
    ParserNode::ProseOrCode{children}                => { write!(f, "ProseOrCode\n"); Some(children) },
    ParserNode::Whitespace{children}                 => { write!(f, "Whitespace\n"); Some(children) },
    ParserNode::SpaceOrTab{children}                 => { write!(f, "SpaceOrTab\n"); Some(children) },
    ParserNode::NewLine{children}                    => { write!(f, "NewLine\n"); Some(children) },
    ParserNode::Token{token, chars, src_range}       => { write!(f, "Token({:?} ({:?}) @ {:?})\n", token, chars, src_range); None },
    ParserNode::CommentSigil{children}               => { write!(f, "CommentSigil\n"); Some(children) },
    ParserNode::Comment{children}                    => { write!(f, "Comment\n"); Some(children) },
    ParserNode::Any{children}                        => { write!(f, "Any\n"); Some(children) },
    ParserNode::Symbol{children}                     => { write!(f, "Symbol\n"); Some(children) },
    ParserNode::Quantity{children}                   => { write!(f, "Quantity\n"); Some(children) },
    ParserNode::NumberLiteral{children}              => { write!(f, "NumberLiteral\n"); Some(children) },
    ParserNode::FloatLiteral{chars, src_range}       => { write!(f, "FloatLiteral({:?} @ {:?})\n", chars, src_range); None },
    ParserNode::DecimalLiteral{chars, src_range}     => { write!(f, "DecimalLiteral({:?} @ {:?})\n", chars, src_range); None },
    ParserNode::HexadecimalLiteral{chars, src_range} => { write!(f, "HexadecimalLiteral({:?} @ {:?})\n", chars, src_range); None },
    ParserNode::OctalLiteral{chars, src_range}       => { write!(f, "OctalLiteral({:?} @ {:?})\n", chars, src_range); None },
    ParserNode::BinaryLiteral{chars, src_range}      => { write!(f, "BinaryLiteral({:?} @ {:?})\n", chars, src_range); None },
    ParserNode::RationalNumber{children}             => { write!(f, "RationalNumber\n"); Some(children) },
    ParserNode::StateMachine{children}               => { write!(f, "StateMachine\n"); Some(children) },
    ParserNode::StateTransition{children}            => { write!(f, "StateTransition\n"); Some(children) },
    ParserNode::Value{children}                      => { write!(f, "Value\n"); Some(children) },
    ParserNode::KindAnnotation{children}             => { write!(f, "KindAnnotation\n"); Some(children) },
    ParserNode::BooleanLiteral{children}             => { write!(f, "BooleanLiteral\n"); Some(children) },
    ParserNode::Add                                  => { write!(f, "Add\n",); None },
    ParserNode::Subtract                             => { write!(f, "Subtract\n",); None },
    ParserNode::Multiply                             => { write!(f, "Multiply\n",); None },
    ParserNode::Divide                               => { write!(f, "Divide\n",); None },
    ParserNode::Exponent                             => { write!(f, "Exponent\n",); None },
    ParserNode::LessThan                             => { write!(f, "LessThan\n",); None },
    ParserNode::GreaterThan                          => { write!(f, "GreaterThan\n",); None },
    ParserNode::GreaterThanEqual                     => { write!(f, "GreaterThanEqual\n",); None },
    ParserNode::LessThanEqual                        => { write!(f, "LessThanEqual\n",); None },
    ParserNode::Equal                                => { write!(f, "Equal\n",); None },
    ParserNode::NotEqual                             => { write!(f, "NotEqual\n",); None },
    ParserNode::And                                  => { write!(f, "And\n",); None },
    ParserNode::Or                                   => { write!(f, "Or\n",); None },
    ParserNode::Xor                                  => { write!(f, "Xor\n",); None },
    ParserNode::AddUpdate                            => { write!(f, "AddUpdate\n",); None },
    ParserNode::SubtractUpdate                       => { write!(f, "SubtractUpdate\n",); None },
    ParserNode::MultiplyUpdate                       => { write!(f, "MultiplyUpdate\n",); None },
    ParserNode::DivideUpdate                         => { write!(f, "DivideUpdate\n",); None },
    ParserNode::ExponentUpdate                       => { write!(f, "ExponentUpdate\n",); None },
    ParserNode::Empty                                => { write!(f, "Empty\n",); None },
    ParserNode::Null                                 => { write!(f, "Null\n",); None },
    ParserNode::ReshapeColumn                        => { write!(f, "ReshapeColumn\n",); None },
    ParserNode::False                                => { write!(f, "False\n",); None },
    ParserNode::True                                 => { write!(f, "True\n",); None },
    ParserNode::Alpha{children}                      => { write!(f, "Alpha\n"); Some(children) },
    ParserNode::Transpose                            => { write!(f, "Transpose\n",); None },
    ParserNode::MatrixMultiply                       => { write!(f, "MatrixMultiply\n",); None },
    ParserNode::Incomplete{children, src_range}      => { write!(f, "Incomplete(@ {:?})\n", src_range); Some(children) },
    ParserNode::Error                                => { write!(f, "Error\n"); None },
  };

  match children {
    Some(childs) => {
      for child in childs {
        print_parser_node(child, level + 1, f)
      }
    },
    _ => (),
  }
}

fn spacer(width: usize, f: &mut fmt::Formatter) {
  let limit = if width > 0 {
    width - 1
  } else {
    width
  };
  for _ in 0..limit {
    write!(f,"│").ok();
  }
  write!(f,"├").ok();
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TokenKind {
  Alpha,
  Digit,
  HashTag,
  LeftBracket,
  RightBracket,
  LeftParenthesis,
  RightParenthesis,
  LeftBrace,
  RightBrace,
  Caret,
  Semicolon,
  Space,
  Plus,
  Dash,
  Underscore,
  At,
  Asterisk,
  Slash,
  Apostrophe,
  Equal,
  LeftAngle,
  RightAngle,
  Exclamation,
  Question,
  Period,
  Colon,
  Comma,
  Tilde,
  Grave,
  Bar,
  Backslash,
  Quote,
  Ampersand,
  Percent,
  Newline,
  CarriageReturn,
  CarriageReturnNewLine,
  Tab,
  Emoji,
  Text,
  True,
  False,
  Number,
  String,
  Title,
  Identifier,
  Empty
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Token { 
  pub kind: TokenKind, 
  pub chars: Vec<char>, 
  pub src_range: SourceRange 
}

impl fmt::Debug for Token {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{:?}:{:?}:{:?}", self.kind, String::from_iter(self.chars.iter().cloned()), self.src_range);
    Ok(())
  }
}

impl Default for Token {
  fn default() -> Self {
    Token{
      kind: TokenKind::Empty,
      chars: vec![],
      src_range: SourceRange::default(),
    }
  }
}

pub fn merge_tokens(tokens: &mut Vec<Token>) -> Option<Token> {
  if tokens.len() == 0 {
    None
  } else if tokens.len() == 1 {
    Some(tokens[0].clone())
  } else {
    let first = tokens[0].src_range.clone();
    let kind = tokens[0].kind.clone();
    let last = tokens.last().unwrap().src_range.clone();
    let src_range = merge_src_range(first, last);
    let chars: Vec<char> = tokens.iter_mut().fold(vec![],|mut m, ref mut t| {m.append(&mut t.chars.clone()); m});
    let merged_token = Token{kind, chars, src_range};
    Some(merged_token)
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Program {
  pub title: Option<Title>,
  pub body: Body,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Title {
  pub text: Token,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Body {
  pub sections: Vec<Section>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Subtitle {
  pub text: Token,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Section {
  pub subtitle: Option<Subtitle>,
  pub elements: Vec<SectionElement>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SectionElement {
  Section(Box<Section>),
  Comment(Comment),
  Paragraph(Paragraph),
  MechCode(MechCode),
  CodeBlock,
  UnorderedList(UnorderedList),
  OrderedList,
  BlockQuote,
  ThematicBreak,
}

pub type ListItem = Paragraph;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnorderedList {
  pub items: Vec<ListItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MechCode {
  Expression(Expression),
  Statement(Statement),
  FunctionDefine,
  FsmSpecification(FsmSpecification),
  FsmImplementation(FsmImplementation),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FsmImplementation {
  pub name: Identifier,
  pub input: Vec<Identifier>,
  pub start: Pattern,
  pub arms: Vec<FsmArm>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FsmArm {
  pub start: Pattern, 
  pub transitions: Vec<Transition>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Transition {
  Next(Pattern),
  Output(Pattern),
  Guard(Guard),
  TransitionBlock(Vec<MechCode>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Guard {
  Wildcard,
  Expression(Expression),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Pattern {
  Wildcard,
  Identifier(Identifier),
  Literal(Literal),
  Table(Table),
  TupleStruct(TupleStruct),
  Tuple(Vec<Pattern>)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TupleStruct {
  pub name: Identifier,
  pub patterns: Vec<Pattern>,
}

pub type Tuple = Vec<Pattern>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FsmSpecification {
  pub name: Identifier,
  pub input: Vec<Identifier>,
  pub output: Identifier,
  pub states: Vec<StateDefinition>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateDefinition {
  pub name: Identifier,
  pub state_variables: Option<Vec<Identifier>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Statement {
  VariableDefine(VariableDefine),
  SplitTable,
  FlattenTable,
  SetData,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TableDefine {
  pub name: Identifier,
  pub table: Table,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
  pub bindings: Vec<Binding>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Table {
  Empty,
  Record(Record),
  Matrix,
  Anonymous(AnonymousTable),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnonymousTable {
  pub rows: Vec<TableRow>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TableColumn {
  pub element: Expression,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TableRow {
  pub columns: Vec<TableColumn>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VariableDefine {
  pub name: Identifier,
  pub expression: Expression,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Identifier {
  pub name: Token,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Emoji {
  pub tokens: Vec<Token>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Word {
  pub tokens: Vec<Token>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Expression {
  Data(Identifier),
  Slice((Identifier,Vec<Expression>)),
  Formula(Formula),
  Table(Table),
  Literal(Literal),
  Transpose(Box<Expression>)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Formula {
  pub lhs: Literal,
  pub operator: Token,
  pub rhs: Literal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InlineTable {
  pub binding: Vec<Binding>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Binding {
  pub name: Identifier,
  pub kind: Option<KindAnnotation>,
  pub value: Expression,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KindAnnotation {
  pub name: Identifier,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Literal {
  Empty(Token),
  Boolean(Token),
  Number(Number),
  String(MechString),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MechString {
  pub text: Token,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ParagraphElement {
  Start(Token),
  Text(Token),
  Bold(Vec<Token>),
  Italic(Vec<Token>),
  Underline(Vec<Token>),
  Strike(Vec<Token>),
  InlineCode
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Paragraph {
  pub elements: Vec<ParagraphElement>,
}

type Sign = bool;
type Numerator = Token;
type Denominator = Token;
type Whole = Token;
type Part = Token;
type Base = (Whole, Part);
type Exponent = (Sign, Whole, Part);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Number {
  Negated(Box<Number>),
  Integer(Token),
  Float((Whole,Part)),
  Decimal(Token),
  Hexadecimal(Token),
  Octal(Token),
  Binary(Token),
  Scientific((Base,Exponent)),
  Rational((Numerator,Denominator))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Comment {
  pub text: Vec<Token>
}
