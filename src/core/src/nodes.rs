use std::cmp::Ordering;
use crate::hash_chars; 
use std::fmt;
use std::io::{Write, Cursor, Read};


pub fn compress_and_encode<T: serde::Serialize>(tree: &T) -> Result<String, Box<dyn std::error::Error>> {
  let serialized_code = bincode::serde::encode_to_vec(tree,bincode::config::standard())?;
  let mut compressed = Vec::new();
  brotli::CompressorWriter::new(&mut compressed, 9, 4096, 22)
      .write(&serialized_code)?;
  Ok(base64::encode(compressed))
}

pub fn decode_and_decompress<T: serde::de::DeserializeOwned>(encoded: &str) -> Result<T, Box<dyn std::error::Error>> {
  let decoded = base64::decode(encoded)?;
  
  let mut decompressed = Vec::new();
  brotli::Decompressor::new(Cursor::new(decoded), 4096)
      .read_to_end(&mut decompressed)?;
  
  let (decoded,red) = bincode::serde::decode_from_slice(&decompressed,bincode::config::standard())?;

  Ok(decoded)
}

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

impl Token {

  pub fn to_string(&self) -> String {
    self.chars.iter().collect()
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
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Title {
  pub text: Token,
}

impl Title {

  pub fn to_string(&self) -> String {
    self.text.to_string()
  }

}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Subtitle {
  pub text: Token,
  pub level: u8,
}

impl Subtitle {
  pub fn to_string(&self) -> String {
    self.text.to_string()
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SectionElement {
  Section(Box<Section>),
  Comment(Comment),
  Paragraph(Paragraph),
  MechCode(Vec<MechCode>),
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
      SectionElement::MechCode(codes) => {
        let mut tokens = vec![];
        for code in codes {
          tokens.append(&mut code.tokens());
        }
        tokens
      },
      _ => todo!(),
    }
  }
}

pub type ListItem = Paragraph;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnorderedList {
  pub items: Vec<ListItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MechCode {
  Expression(Expression),
  Statement(Statement),
  FsmSpecification(FsmSpecification),
  FsmImplementation(FsmImplementation),
  FunctionDefine(FunctionDefine),
  Comment(Comment),
}

impl MechCode {
  pub fn tokens(&self) -> Vec<Token> {
    match self {
      MechCode::Expression(x) => x.tokens(),
      _ => todo!(),
      //Statement(x) => x.tokens(),
      //FunctionDefine(x) => x.tokens(),
      //FsmSpecification(x) => x.tokens(),
      //FsmImplementation(x) => x.tokens(),
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionDefine {
  pub name: Identifier,
  pub input: Vec<FunctionArgument>,
  pub output: Vec<FunctionArgument>,
  pub statements: Vec<Statement>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionArgument {
  pub name: Identifier,
  pub kind: KindAnnotation,
}

impl FunctionArgument {
  pub fn tokens(&self) -> Vec<Token> {
    let mut tokens = self.name.tokens();
    tokens.append(&mut self.kind.tokens());
    tokens
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FsmImplementation {
  pub name: Identifier,
  pub input: Vec<Identifier>,
  pub start: Pattern,
  pub arms: Vec<FsmArm>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FsmArm {
  Guard(Pattern,Vec<Guard>),
  Transition(Pattern,Vec<Transition>),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Guard { 
  pub condition: Pattern,
  pub transitions: Vec<Transition>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Transition {
  Next(Pattern),
  Output(Pattern),
  Async(Pattern),
  CodeBlock(Vec<MechCode>),
  Statement(Statement),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Pattern {
  Wildcard,
  Formula(Factor),
  Expression(Expression),
  TupleStruct(PatternTupleStruct),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PatternTupleStruct {
  pub name: Identifier,
  pub patterns: Vec<Pattern>,
}

pub type PatternTuple = Vec<Pattern>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FsmSpecification {
  pub name: Identifier,
  pub input: Vec<Var>,
  pub output: Option<KindAnnotation>,
  pub states: Vec<StateDefinition>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateDefinition {
  pub name: Identifier,
  pub state_variables: Option<Vec<Var>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Statement {
  VariableDefine(VariableDefine),
  VariableAssign(VariableAssign),
  KindDefine(KindDefine),
  EnumDefine(EnumDefine),
  FsmDeclare(FsmDeclare),    
  OpAssign(OpAssign), 
  SplitTable,     // todo
  FlattenTable,   // todo
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FsmPipe {
  pub start: FsmInstance,
  pub transitions: Vec<Transition>
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PipeElement {
  Expression(Expression),
  FsmInstance(FsmInstance),
  Timer // todo
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FsmDeclare {
  pub fsm: Fsm,
  pub pipe: FsmPipe,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Fsm {
  pub name: Identifier,
  pub args: Option<ArgumentList>,
  pub kind: Option<KindAnnotation>
}

pub type FsmArgs = Vec<(Option<Identifier>,Expression)>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FsmInstance {
  pub name: Identifier,
  pub args: Option<FsmArgs>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnumDefine {
  pub name: Identifier,
  pub variants: Vec<EnumVariant>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnumVariant {
  pub name: Identifier,
  pub value: Option<KindAnnotation>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct KindDefine {
  pub name: Identifier,
  pub kind: KindAnnotation,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Record {
  pub bindings: Vec<Binding>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Map {
  pub elements: Vec<Mapping>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Mapping {
  pub key: Expression,
  pub value: Expression,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Set {
  pub elements: Vec<Expression>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Atom {
  pub name: Identifier,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TupleStruct {
  pub name: Identifier,
  pub value: Box<Expression>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Table {
  pub header: TableHeader,
  pub rows: Vec<TableRow>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Field {
  pub name: Identifier,
  pub kind: Option<KindAnnotation>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TableColumn {
  pub element: Expression,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MatrixColumn {
  pub element: Expression,
}

impl MatrixColumn {
  pub fn tokens(&self) -> Vec<Token> {
    self.element.tokens()
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TableRow {
  pub columns: Vec<TableColumn>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VariableDefine {
  pub mutable: bool,
  pub var: Var,
  pub expression: Expression,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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
    self.name.to_string()
  }

}


impl Identifier {
  pub fn hash(&self) -> u64 {
    hash_chars(&self.name.chars)
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Emoji {
  pub tokens: Vec<Token>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Word {
  pub tokens: Vec<Token>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Slice {
  pub name: Identifier,
  pub subscript: Vec<Subscript>
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SliceRef {
  pub name: Identifier,
  pub subscript: Option<Vec<Subscript>>
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionCall {
  pub name: Identifier,
  pub args: ArgumentList,
}

impl FunctionCall {
  pub fn tokens(&self) -> Vec<Token> {
    self.name.tokens()
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tuple {
  pub elements: Vec<Expression>
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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
      Kind::Tuple(x) => x.iter().flat_map(|k| k.tokens()).collect(),
      Kind::Bracket((kinds, literals)) => {
        kinds.iter().flat_map(|k| k.tokens())
            .chain(literals.iter().flat_map(|l| l.tokens()))
            .collect()
      },
      Kind::Brace((kinds, literals)) => {
        kinds.iter().flat_map(|k| k.tokens())
            .chain(literals.iter().flat_map(|l| l.tokens()))
            .collect()
      }
      Kind::Map(x, y) => x.tokens().into_iter().chain(y.tokens()).collect(),
      Kind::Scalar(x) => x.tokens(),
      Kind::Atom(x) => x.tokens(),
      Kind::Function(args, rets) => {
        args.iter().flat_map(|k| k.tokens())
            .chain(rets.iter().flat_map(|k| k.tokens()))
            .collect()
      }
      Kind::Fsm(args, rets) => {
        args.iter().flat_map(|k| k.tokens())
            .chain(rets.iter().flat_map(|k| k.tokens()))
            .collect()
      }
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
      Literal::Atom(atm) => atm.name.tokens(),
      _ => todo!(),
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MechString {
  pub text: Token,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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

impl ParagraphElement {

  pub fn to_string(&self) -> String {
    match self {
      ParagraphElement::Start(t) => t.to_string(),
      ParagraphElement::Text(t) => t.to_string(),
      _ => "".to_string(),
    }
  }

}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Paragraph {
  pub elements: Vec<ParagraphElement>,
}

impl Paragraph {
  pub fn to_string(&self) -> String {
    let mut out = "".to_string();
    for e in &self.elements {
      out.push_str(&e.to_string());
    }
    out
  }
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Comment {
  pub text: Token,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpAssign {
  pub target: SliceRef,
  pub op: OpAssignOp,
  pub expression: Expression,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OpAssignOp {
  Add,
  Sub,   
  Mul,
  Div,
  Exp,   
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RangeOp {
  Inclusive,
  Exclusive,      
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AddSubOp {
  Add,
  Sub
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MulDivOp {
  Mul,
  Div
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum VecOp {
  MatMul,
  Solve,
  Dot,
  Cross,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExponentOp {
  Exp
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComparisonOp {
  LessThan,
  GreaterThan,
  LessThanEqual,
  GreaterThanEqual,
  Equal,
  NotEqual,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum LogicOp {
  And,
  Or,
  Not,
  Xor,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FormulaOperator {
  Logic(LogicOp),
  Comparison(ComparisonOp),
  AddSub(AddSubOp),
  MulDiv(MulDivOp),
  Exponent(ExponentOp),
  Vec(VecOp),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RangeExpression {
  pub start: Factor,
  pub increment: Option<(RangeOp,Factor)>,
  pub operator: RangeOp,
  pub terminal: Factor,
}

impl RangeExpression {
  pub fn tokens(&self) -> Vec<Token> {
    let mut tokens = self.start.tokens();
    tokens.append(&mut self.terminal.tokens());
    tokens
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Factor {
  Term(Box<Term>),
  Parenthetical(Box<Factor>),
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
      Factor::Parenthetical(x) => x.tokens(),
    }
  }
}