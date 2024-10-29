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
  BoxDrawing,
  Dollar,
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

impl Program {
  pub fn tokens(&self) -> Vec<Token> {
    /*let mut title_tokens = match self.title.tokens() {
      Some(tkns) => tkns,
      None => vec![],
    };*/
    let body_tokens = self.body.tokens();
    body_tokens
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Title {
  pub text: Token,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Body {
  pub sections: Vec<Section>,
}

impl Body {
  pub fn tokens(&self) -> Vec<Token> {
    let mut out = vec![];
    for s in &self.sections {
      let mut tkns = s.tokens();
      out.append(&mut tkns);
    }
    out
  }
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

impl Section {
  pub fn tokens(&self) -> Vec<Token> {
    let mut out = vec![];
    for s in &self.elements {
      let mut tkns = s.tokens();
      out.append(&mut tkns);
    }
    out
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SectionElement {
  Section(Box<Section>),
  Comment(Comment),
  Paragraph(Paragraph),
  MechCode(MechCode),
  UnorderedList(UnorderedList),
  CodeBlock,       // todo
  OrderedList,     // todo
  BlockQuote,      // todo
  ThematicBreak,   // todo
  Image,           // todo
}

impl SectionElement {
  pub fn tokens(&self) -> Vec<Token> {
    match self {
      SectionElement::MechCode(code) => code.tokens(),
      _ => todo!(),
    }
  }
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
  FsmSpecification(FsmSpecification),
  FsmImplementation(FsmImplementation),
  FunctionDefine(FunctionDefine),
}

impl MechCode {
  pub fn tokens(&self) -> Vec<Token> {
    match self {
      MechCode::Expression(x) => x.tokens(),
      _ => todo!(),
      //Statement(x) => x.tokens(),
      //FsmSpecification(x) => x.tokens(),
      //FsmImplementation(x) => x.tokens(),
      //FunctionDefine(x) => x.tokens(),
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionDefine {
  pub name: Identifier,
  pub input: Vec<FunctionArgument>,
  pub output: Vec<FunctionArgument>,
  pub statements: Vec<Statement>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionArgument {
  pub name: Identifier,
  pub kind: KindAnnotation,
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
  Async(Pattern),
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
  Formula(Factor),
  Expression(Expression),
  TupleStruct(PatternTupleStruct),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatternTupleStruct {
  pub name: Identifier,
  pub patterns: Vec<Pattern>,
}

pub type PatternTuple = Vec<Pattern>;

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
  VariableAssign(VariableAssign),
  KindDefine(KindDefine),
  EnumDefine(EnumDefine),
  FsmDeclare(FsmDeclare),     
  SplitTable,     // todo
  FlattenTable,   // todo
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FsmPipe {
  pub start: FsmInstance,
  pub transitions: Vec<Transition>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PipeElement {
  Expression(Expression),
  FsmInstance(FsmInstance),
  Timer // todo
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FsmDeclare {
  pub fsm: Fsm,
  pub pipe: FsmPipe,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fsm {
  pub name: Identifier,
  pub args: Option<ArgumentList>,
  pub kind: Option<KindAnnotation>
}

pub type FsmArgs = Vec<(Option<Identifier>,Expression)>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FsmInstance {
  pub name: Identifier,
  pub args: Option<FsmArgs>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnumDefine {
  pub name: Identifier,
  pub variants: Vec<EnumVariant>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnumVariant {
  pub name: Identifier,
  pub value: Option<KindAnnotation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KindDefine {
  pub name: Identifier,
  pub kind: KindAnnotation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
  pub bindings: Vec<Binding>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Structure {
  Empty,
  Record(Record),
  Matrix(Matrix),
  Table(Table),
  Tuple(Tuple),
  TupleStruct(TupleStruct),
  Set(Set),
  Map(Map),
}

impl Structure {
  pub fn tokens(&self) -> Vec<Token> {
    match self {
      Structure::Matrix(mat) => mat.tokens(),
      _ => todo!(),
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Map {
  pub elements: Vec<Mapping>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mapping {
  pub key: Expression,
  pub value: Expression,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Set {
  pub elements: Vec<Expression>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Atom {
  pub name: Identifier,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TupleStruct {
  pub name: Identifier,
  pub value: Box<Expression>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Matrix {
  pub rows: Vec<MatrixRow>,
}

impl Matrix {
  pub fn tokens(&self) -> Vec<Token> {
    let mut tkns = vec![];
    for r in &self.rows {
      let mut t = r.tokens();
      tkns.append(&mut t);
    }
    tkns
  }
}

pub type TableHeader = Vec<Field>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Table {
  pub header: TableHeader,
  pub rows: Vec<TableRow>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Field {
  pub name: Identifier,
  pub kind: KindAnnotation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TableColumn {
  pub element: Expression,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatrixColumn {
  pub element: Expression,
}

impl MatrixColumn {
  pub fn tokens(&self) -> Vec<Token> {
    self.element.tokens()
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TableRow {
  pub columns: Vec<TableColumn>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatrixRow {
  pub columns: Vec<MatrixColumn>,
}

impl MatrixRow {
  pub fn tokens(&self) -> Vec<Token> {
    let mut tkns = vec![];
    for r in &self.columns {
      let mut t = r.tokens();
      tkns.append(&mut t);
    }
    tkns
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VariableDefine {
  pub var: Var,
  pub expression: Expression,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Var {
  pub name: Identifier,
  pub kind: Option<KindAnnotation>,
}

impl Var {
  pub fn tokens(&self) -> Vec<Token> {
    let mut tkns = self.name.tokens();
    if let Some(knd) = &self.kind {
      let mut t = knd.tokens();
      tkns.append(&mut t);
    }
    tkns
  }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VariableAssign {
  pub target: SliceRef,
  pub expression: Expression,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Identifier {
  pub name: Token,
}

impl Identifier {
  pub fn tokens(&self) -> Vec<Token> {
    vec![self.name.clone()]
  }

  pub fn to_string(&self) -> String {
    self.name.chars.iter().collect()
  }

}


impl Identifier {
  pub fn hash(&self) -> u64 {
    hash_chars(&self.name.chars)
  }
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
pub struct Slice {
  pub name: Identifier,
  pub subscript: Vec<Subscript>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SliceRef {
  pub name: Identifier,
  pub subscript: Option<Vec<Subscript>>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Subscript {
  Dot(Identifier),          // a.b
  Swizzle(Vec<Identifier>), // a.b,c
  Range(RangeExpression),   // a[1 + 1]
  Formula(Factor),          // a[1 + 1]
  All,                      // a[:]
  Bracket(Vec<Subscript>),  // a[1,2,3]
  Brace(Vec<Subscript>),    // a{"foo"}
  DotInt(RealNumber)        // a.1
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Expression {
  Var(Var),
  Range(Box<RangeExpression>),
  Slice(Slice),
  Formula(Factor),
  Structure(Structure),
  Literal(Literal),
  FunctionCall(FunctionCall),
  FsmPipe(FsmPipe),
}

impl Expression {
  pub fn tokens(&self) -> Vec<Token> {
    match self {
      Expression::Var(v) => v.tokens(),
      Expression::Literal(ltrl) => ltrl.tokens(),
      Expression::Structure(strct) => strct.tokens(),
      Expression::Formula(fctr) => fctr.tokens(),
      _ => todo!(),
    }
  }
}

pub type ArgumentList = Vec<(Option<Identifier>,Expression)>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionCall {
  pub name: Identifier,
  pub args: ArgumentList,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tuple {
  pub elements: Vec<Expression>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Binding {
  pub name: Identifier,
  pub kind: Option<KindAnnotation>,
  pub value: Expression,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct KindAnnotation {
  pub kind: Kind
}

impl KindAnnotation {

  pub fn hash(&self) -> u64 {
    match &self.kind {
      Kind::Scalar(id) => id.hash(),
      _ => todo!(),
    }
  }

  pub fn tokens(&self) -> Vec<Token> {
    self.kind.tokens()
  }
}

#[derive(Clone, Debug, Serialize, Deserialize,Eq, PartialEq)]
pub enum Kind {
  Tuple(Vec<Kind>),
  Bracket((Vec<Kind>,Vec<Literal>)),
  Brace((Vec<Kind>,Vec<Literal>)),
  Map(Box<Kind>,Box<Kind>),
  Scalar(Identifier),
  Atom(Identifier),
  Function(Vec<Kind>,Vec<Kind>),
  Fsm(Vec<Kind>,Vec<Kind>),
  Empty,
}

impl Kind {
  pub fn tokens(&self) -> Vec<Token> {
    match self {
      Kind::Tuple(x) => todo!(),
      Kind::Bracket(x) => todo!(),
      Kind::Brace(x) => todo!(),
      Kind::Map(x,y) => todo!(),
      Kind::Scalar(x) => x.tokens(),
      Kind::Atom(x) => x.tokens(),
      Kind::Function(x,y) => todo!(),
      Kind::Fsm(x,y) => todo!(),
      Kind::Empty => vec![],
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Literal {
  Empty(Token),
  Boolean(Token),
  Number(Number),
  String(MechString),
  Atom(Atom),
  TypedLiteral((Box<Literal>,KindAnnotation))
}

impl Literal {
  pub fn tokens(&self) -> Vec<Token> {
    match self {
      Literal::Number(x) => x.tokens(),
      Literal::Boolean(tkn) => vec![tkn.clone()],
      Literal::String(strng) => vec![strng.text.clone()],
      _ => todo!(),
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MechString {
  pub text: Token,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ParagraphElement {
  Start(Token),
  Text(Token),
  Bold,            // todo
  Italic,          // todo
  Underline,       // todo
  Strike,          // todo
  InlineCode,      // todo           
  Link,            // todo
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Paragraph {
  pub elements: Vec<ParagraphElement>,
}

pub type Sign = bool;
pub type Numerator = Token;
pub type Denominator = Token;
pub type Whole = Token;
pub type Part = Token;
pub type Real = Box<Number>;
pub type Imaginary = Box<Number>;
pub type Base = (Whole, Part);
pub type Exponent = (Sign, Whole, Part);

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Number {
  Real(RealNumber),
  Imaginary(ComplexNumber),
}

impl Number {
  pub fn tokens(&self) -> Vec<Token> {
    match self {
      Number::Real(x) => x.tokens(),
      _ => todo!(),
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum RealNumber {
  Negated(Box<RealNumber>),
  Integer(Token),
  Float((Whole,Part)),
  Decimal(Token),
  Hexadecimal(Token),
  Octal(Token),
  Binary(Token),
  Scientific((Base,Exponent)),
  Rational((Numerator,Denominator)),
}

impl RealNumber {
  pub fn tokens(&self) -> Vec<Token> {
    match self {
      RealNumber::Integer(tkn) => vec![tkn.clone()],
      _ => todo!(),
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ImaginaryNumber {
  pub number: RealNumber,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ComplexNumber {
  pub real: Option<RealNumber>,
  pub imaginary: ImaginaryNumber
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Comment {
  pub text: Token,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RangeOp {
  Inclusive,
  Exclusive,      
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AddSubOp {
  Add,
  Sub
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MulDivOp {
  Mul,
  Div
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum VecOp {
  MatMul,
  Solve,
  Dot,
  Cross,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExponentOp {
  Exp
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ComparisonOp {
  LessThan,
  GreaterThan,
  LessThanEqual,
  GreaterThanEqual,
  Equal,
  NotEqual,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LogicOp {
  And,
  Or,
  Not,
  Xor,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FormulaOperator {
  Logic(LogicOp),
  Comparison(ComparisonOp),
  AddSub(AddSubOp),
  MulDiv(MulDivOp),
  Exponent(ExponentOp),
  Vec(VecOp),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RangeExpression {
  pub start: Factor,
  pub increment: Option<(RangeOp,Factor)>,
  pub operator: RangeOp,
  pub terminal: Factor,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Term {
  pub lhs: Factor,
  pub rhs: Vec<(FormulaOperator,Factor)>
}

impl Term {
  pub fn tokens(&self) -> Vec<Token> {
    let mut lhs_tkns = self.lhs.tokens();
    let mut rhs_tkns = vec![];
    for (op, r) in &self.rhs {
      let mut tkns = r.tokens();
      rhs_tkns.append(&mut tkns);
    }
    lhs_tkns.append(&mut rhs_tkns);
    lhs_tkns
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Factor {
  Term(Box<Term>),
  Expression(Box<Expression>),
  Negate(Box<Factor>),
  Not(Box<Factor>),
  Transpose(Box<Factor>),
}

impl Factor {
  pub fn tokens(&self) -> Vec<Token> {
    match self {
      Factor::Term(x) => x.tokens(),
      Factor::Expression(x) => x.tokens(),
      Factor::Negate(x) => x.tokens(),
      Factor::Not(x) => x.tokens(),
      Factor::Transpose(x) => x.tokens(),
    }
  }
}