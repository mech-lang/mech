use std::cmp::Ordering;
use crate::*; 
use std::fmt;
use std::io::{Write, Cursor, Read};
use std::hash::Hash;
use std::hash::Hasher;

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
  AbstractSigil, Alpha, Ampersand, Any, Apostrophe, AssignOperator, Asterisk, AsyncTransitionOperator, At,
  Backslash, Bar, BoxDrawing,
  Caret, CarriageReturn, CarriageReturnNewLine, Colon, CodeBlock, Comma,
  Dash, DefineOperator, Digit, Dollar,
  Emoji, EmphasisSigil, Empty, Equal, EquationSigil, Exclamation, 
  False, FloatLeft, FloatRight, FootnotePrefix,
  Grave,
  HashTag, HighlightSigil, HttpPrefix,
  Identifier, ImgPrefix, InfoSigil, InlineCode, 
  LeftAngle, LeftBrace, LeftBracket, LeftParenthesis,
  Newline, Not, Number,
  OutputOperator,
  Percent, Period, Plus,
  QuerySigil, Question, Quote, QuoteSigil,
  RightAngle, RightBrace, RightBracket, RightParenthesis,
  Semicolon, Space, Slash, String, StrikeSigil, StrongSigil,
  Tab, Text, Tilde, Title, TransitionOperator, True,
  UnderlineSigil, Underscore,
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

  pub fn new(kind: TokenKind, src_range: SourceRange, chars: Vec<char>) -> Token {
    Token{kind, chars, src_range}
  }

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

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct TableOfContents {
    pub title: Option<Title>,
    pub sections: Vec<Section>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
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

  pub fn table_of_contents(&self) -> TableOfContents {
    let mut sections = vec![];
    for s in &self.body.sections {
      sections.push(s.table_of_contents());
    }
    TableOfContents {
      title: self.title.clone(),
      sections,
    }
  }

  pub fn pretty_print(&self) -> String {
    let json_string = serde_json::to_string_pretty(self).unwrap();
  
    let depth = |line: &str|->usize{line.chars().take_while(|&c| c == ' ').count()};
    let mut result = String::new();
    let lines: Vec<&str> = json_string.lines().collect();
    result.push_str("Program\n");
    for (index, line) in lines.iter().enumerate() {
      let trm = line.trim();
      if trm == "}" || 
         trm == "},"|| 
         trm == "{" || 
         trm == "[" || 
         trm == "],"|| 
         trm == "]" {
        continue;
      }
  
      // Count leading spaces to determine depth
      let d = depth(line);
      // Construct the tree-like prefix
      let mut prefix = String::new();
      for _ in 0..d {
        prefix.push_str(" ");
      }
      if index == lines.len() {
        prefix.push_str("└ ");
      } else {
        if depth(lines[index + 1]) != d {
          prefix.push_str("└ ");
        } else {
          prefix.push_str("├ ");
        }
      }
      let trm = line.trim();
      let trm = trm.trim_end_matches('{')
                    .trim_start_matches('"')
                    .trim_end_matches(':')
                    .trim_end_matches('"')
                    .trim_end_matches('[');
      prefix.push_str(trm);
  
      // Append formatted line to result
      result.push_str(&prefix);
      result.push('\n');
      result = result.replace("\":", "");
    }
    let mut indexed_str = IndexedString::new(&result);
    'rows: for i in 0..indexed_str.rows {
      let rowz = &indexed_str.index_map[i];
      for j in 0..rowz.len() {
        let c = match indexed_str.get(i,j) {
          Some(c) => c,
          None => continue,
        };
        if c == '└' {
          for k in i+1..indexed_str.rows {
            match indexed_str.get(k,j) {
              Some(c2) => {
                if c2 == '└' {
                  indexed_str.set(i,j,'├');
                  for l in i+1..k {
                    match indexed_str.get(l,j) {
                      Some(' ') => {indexed_str.set(l,j,'│');},
                      Some('└') => {indexed_str.set(l,j,'├');},
                      _ => (),
                    }
                  }
                } else if c2 == ' ' {
                  continue;
                } else {
                  continue 'rows;
                }
              },
              None => continue,
            }
  
          }
        } else if c == ' ' || c == '│' {
          continue;
        } else {
          continue 'rows;
        }
      }
    }
    indexed_str.to_string()
  }

}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct Title {
  pub text: Token,
}

impl Title {

  pub fn to_string(&self) -> String {
    self.text.to_string()
  }

}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum ColumnAlignment {
  Left,
  Center,
  Right,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarkdownTable {
  pub header: Vec<Paragraph>,
  pub rows: Vec<Vec<Paragraph>>,
  pub alignment: Vec<ColumnAlignment>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct Subtitle {
  pub text: Paragraph,
  pub level: u8,
}

impl Subtitle {
  pub fn to_string(&self) -> String {
    self.text.to_string()
  }
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct Section {
  pub subtitle: Option<Subtitle>,
  pub elements: Vec<SectionElement>,
}

impl Section {

  pub fn table_of_contents(&self) -> Section {
    let elements: Vec<_> = self.elements.iter()
      .filter(|e| matches!(e, SectionElement::Subtitle(_)))
      .cloned()
      .collect();
    Section {
      subtitle: self.subtitle.clone(),
      elements,
    }
  }

  pub fn tokens(&self) -> Vec<Token> {
    let mut out = vec![];
    for s in &self.elements {
      let mut tkns = s.tokens();
      out.append(&mut tkns);
    }
    out
  }
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct Grammar {
  pub rules: Vec<Rule>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct GrammarIdentifier {
  pub name: Token,
}

impl GrammarIdentifier {
  pub fn tokens(&self) -> Vec<Token> {
    vec![self.name.clone()]
  }

  pub fn to_string(&self) -> String {
    self.name.to_string()
  }
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rule {
  pub name: GrammarIdentifier,
  pub expr: GrammarExpression,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum GrammarExpression {
  Choice(Vec<GrammarExpression>),
  Definition(GrammarIdentifier),
  Group(Box<GrammarExpression>),
  List(Box<GrammarExpression>, Box<GrammarExpression>),
  Not(Box<GrammarExpression>),
  Optional(Box<GrammarExpression>),
  Peek(Box<GrammarExpression>),
  Repeat0(Box<GrammarExpression>),
  Repeat1(Box<GrammarExpression>),
  Range(Token,Token),
  Sequence(Vec<GrammarExpression>),
  Terminal(Token),
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct Citation {
  pub id: Token,
  pub text: Paragraph,
}

impl Citation {
  pub fn to_string(&self) -> String {
    format!("[{}]: {}", self.id.to_string(), self.text.to_string())
  }
}

// Stores code block configuration
#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockConfig {
  pub namespace: u64,
  pub disabled: bool,
}

pub type Footnote = (Token, Paragraph);

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum FloatDirection {
  Left,
  Right,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum SectionElement {
  Abstract(Vec<Paragraph>),
  QuoteBlock(Vec<Paragraph>),
  InfoBlock(Vec<Paragraph>),
  QuestionBlock(Vec<Paragraph>),
  Citation(Citation),
  CodeBlock(Token),
  Comment(Comment),
  Diagram(Token),
  Equation(Token),
  FencedMechCode((Vec<(MechCode,Option<Comment>)>, BlockConfig)),
  Float((Box<SectionElement>, FloatDirection)),
  Footnote(Footnote),
  Grammar(Grammar),
  Image(Image),
  List(MDList),
  MechCode(Vec<(MechCode,Option<Comment>)>),
  Paragraph(Paragraph),
  Subtitle(Subtitle),
  Table(MarkdownTable),
  ThematicBreak,
}

impl SectionElement {
  pub fn tokens(&self) -> Vec<Token> {
    match self {
      SectionElement::FencedMechCode((code,config)) => {
        let mut tokens = vec![];
        for (c,_) in code {
          tokens.append(&mut c.tokens());
        }
        tokens
      }
      SectionElement::MechCode(codes) => {
        let mut tokens = vec![];
        for (code,_) in codes {
          tokens.append(&mut code.tokens());
        }
        tokens
      },
      _ => todo!(),
    }
  }
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct Image {
  pub src: Token,
  pub caption: Option<Paragraph>,
}

impl Image {
  pub fn to_string(&self) -> String {
    let caption = match &self.caption {
      Some(c) => c.to_string(),
      None => "".to_string(),
    };
    format!("![{}]({})", caption, self.src.to_string())
  }
}

pub type UnorderedList = Vec<((Option<Token>,Paragraph),Option<MDList>)>;
pub type CheckList = Vec<((bool,Paragraph),Option<MDList>)>;

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrderedList {
  pub start: Number,
  pub items: Vec<((Number,Paragraph),Option<MDList>)>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum MDList {
  Unordered(UnorderedList),
  Ordered(OrderedList),
  Check(CheckList)
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum MechCode {
  Comment(Comment),
  Expression(Expression),
  FsmImplementation(FsmImplementation),
  FsmSpecification(FsmSpecification),
  FunctionDefine(FunctionDefine),
  Statement(Statement),
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

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionDefine {
  pub name: Identifier,
  pub input: Vec<FunctionArgument>,
  pub output: Vec<FunctionArgument>,
  pub statements: Vec<Statement>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct FsmImplementation {
  pub name: Identifier,
  pub input: Vec<Identifier>,
  pub start: Pattern,
  pub arms: Vec<FsmArm>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum FsmArm {
  Guard(Pattern,Vec<Guard>),
  Transition(Pattern,Vec<Transition>),
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct Guard { 
  pub condition: Pattern,
  pub transitions: Vec<Transition>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum Transition {
  Async(Pattern),
  CodeBlock(Vec<(MechCode, Option<Comment>)>),
  Next(Pattern),
  Output(Pattern),
  Statement(Statement),
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum Pattern {
  Expression(Expression),
  Formula(Factor),
  TupleStruct(PatternTupleStruct),
  Wildcard,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct PatternTupleStruct {
  pub name: Identifier,
  pub patterns: Vec<Pattern>,
}

pub type PatternTuple = Vec<Pattern>;

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct FsmSpecification {
  pub name: Identifier,
  pub input: Vec<Var>,
  pub output: Option<KindAnnotation>,
  pub states: Vec<StateDefinition>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateDefinition {
  pub name: Identifier,
  pub state_variables: Option<Vec<Var>>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum Statement {
  EnumDefine(EnumDefine),
  FsmDeclare(FsmDeclare),
  KindDefine(KindDefine),
  OpAssign(OpAssign),
  VariableAssign(VariableAssign),
  VariableDefine(VariableDefine),
  TupleDestructure(TupleDestructure),
  SplitTable,     // todo
  FlattenTable,   // todo
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct TupleDestructure {
  pub vars: Vec<Identifier>,
  pub expression: Expression,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct FsmPipe {
  pub start: FsmInstance,
  pub transitions: Vec<Transition>
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum PipeElement {
  Expression(Expression),
  FsmInstance(FsmInstance),
  Timer // todo
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct FsmDeclare {
  pub fsm: Fsm,
  pub pipe: FsmPipe,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct Fsm {
  pub name: Identifier,
  pub args: Option<ArgumentList>,
  pub kind: Option<KindAnnotation>
}

pub type FsmArgs = Vec<(Option<Identifier>,Expression)>;

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct FsmInstance {
  pub name: Identifier,
  pub args: Option<FsmArgs>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnumDefine {
  pub name: Identifier,
  pub variants: Vec<EnumVariant>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnumVariant {
  pub name: Identifier,
  pub value: Option<KindAnnotation>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct KindDefine {
  pub name: Identifier,
  pub kind: KindAnnotation,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct Record {
  pub bindings: Vec<Binding>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum Structure {
  Empty,
  Map(Map),
  Matrix(Matrix),
  Record(Record),
  Set(Set),
  Table(Table),
  Tuple(Tuple),
  TupleStruct(TupleStruct),
}

impl Structure {
  pub fn tokens(&self) -> Vec<Token> {
    match self {
      Structure::Matrix(mat) => mat.tokens(),
      _ => todo!(),
    }
  }
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct Map {
  pub elements: Vec<Mapping>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct Mapping {
  pub key: Expression,
  pub value: Expression,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct Set {
  pub elements: Vec<Expression>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, Eq, PartialEq)]
pub struct Atom {
  pub name: Identifier,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct TupleStruct {
  pub name: Identifier,
  pub value: Box<Expression>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct Table {
  pub header: TableHeader,
  pub rows: Vec<TableRow>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct Field {
  pub name: Identifier,
  pub kind: Option<KindAnnotation>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct TableColumn {
  pub element: Expression,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct MatrixColumn {
  pub element: Expression,
}

impl MatrixColumn {
  pub fn tokens(&self) -> Vec<Token> {
    self.element.tokens()
  }
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct TableRow {
  pub columns: Vec<TableColumn>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct VariableDefine {
  pub mutable: bool,
  pub var: Var,
  pub expression: Expression,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
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


#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct VariableAssign {
  pub target: SliceRef,
  pub expression: Expression,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct Emoji {
  pub tokens: Vec<Token>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct Word {
  pub tokens: Vec<Token>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct Slice {
  pub name: Identifier,
  pub subscript: Vec<Subscript>
}

impl Slice {
  pub fn tokens(&self) -> Vec<Token> {
    let mut tkns = self.name.tokens();
    for sub in &self.subscript {
      let mut sub_tkns = sub.tokens();
      tkns.append(&mut sub_tkns);
    }
    tkns
  }
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct SliceRef {
  pub name: Identifier,
  pub subscript: Option<Vec<Subscript>>
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum Subscript {
  All,                      // a[:]
  Brace(Vec<Subscript>),    // a{"foo"}
  Bracket(Vec<Subscript>),  // a[1,2,3]
  Dot(Identifier),          // a.b
  DotInt(RealNumber),       // a.1
  Formula(Factor),          // a[1 + 1]
  Range(RangeExpression),   // a[1 + 1]
  Swizzle(Vec<Identifier>), // a.b,c
}

impl Subscript {

  pub fn tokens(&self) -> Vec<Token> {
    match self {
      Subscript::All => vec![
        Token::new(TokenKind::Colon, SourceRange::default(), vec![':']),
      ],
      Subscript::Brace(subs) => {
        let mut tkns = vec![Token::new(TokenKind::LeftBrace, SourceRange::default(), vec!['{'])];
        for sub in subs {
          let mut sub_tkns = sub.tokens();
          tkns.append(&mut sub_tkns);
        }
        tkns.push(Token::new(TokenKind::RightBrace, SourceRange::default(), vec!['}']));
        tkns
      },
      Subscript::Bracket(subs) => {
        let mut tkns = vec![Token::new(TokenKind::LeftBracket, SourceRange::default(), vec!['['])];
        for sub in subs {
          let mut sub_tkns = sub.tokens();
          tkns.append(&mut sub_tkns);
        }
        tkns.push(Token::new(TokenKind::RightBracket, SourceRange::default(), vec![']']));
        tkns
      },
      Subscript::Dot(id) => id.tokens(),
      Subscript::DotInt(num) => num.tokens(),
      Subscript::Formula(factor) => factor.tokens(),
      Subscript::Range(range) => range.tokens(),
      Subscript::Swizzle(ids) => ids.iter().flat_map(|id| id.tokens()).collect(),
    }
  }
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum Expression {
  Formula(Factor),
  FunctionCall(FunctionCall),
  FsmPipe(FsmPipe),
  Literal(Literal),
  Range(Box<RangeExpression>),
  Slice(Slice),
  Structure(Structure),
  Var(Var),
}

impl Expression {
  pub fn tokens(&self) -> Vec<Token> {
    match self {
      Expression::Var(v) => v.tokens(),
      Expression::Literal(ltrl) => ltrl.tokens(),
      Expression::Structure(strct) => strct.tokens(),
      Expression::Formula(fctr) => fctr.tokens(),
      Expression::Range(range) => range.tokens(),
      Expression::Slice(slice) => slice.tokens(),
      _ => todo!(),
    }
  }
}

pub type ArgumentList = Vec<(Option<Identifier>,Expression)>;

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionCall {
  pub name: Identifier,
  pub args: ArgumentList,
}

impl FunctionCall {
  pub fn tokens(&self) -> Vec<Token> {
    self.name.tokens()
  }
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tuple {
  pub elements: Vec<Expression>
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct Binding {
  pub name: Identifier,
  pub kind: Option<KindAnnotation>,
  pub value: Expression,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, Eq, PartialEq)]
pub struct KindAnnotation {
  pub kind: Kind,
}

impl KindAnnotation {

  pub fn hash(&self) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    self.kind.hash(&mut hasher);
    hasher.finish()
  }

  pub fn tokens(&self) -> Vec<Token> {
    self.kind.tokens()
  }
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize,Eq, PartialEq)]
pub enum Kind {
  Any,
  Atom(Identifier),
  Table((Vec<(Identifier,Kind)>,Box<Literal>)),
  Set(Box<Kind>,Option<Box<Literal>>),
  Record((Vec<(Identifier,Kind)>)),
  Empty,
  //Fsm(Vec<Kind>,Vec<Kind>),
  //Function(Vec<Kind>,Vec<Kind>),
  Map(Box<Kind>,Box<Kind>),
  Matrix((Box<Kind>,Vec<Literal>)),
  Option(Box<Kind>),
  Scalar(Identifier),
  Tuple(Vec<Kind>),
}

impl Kind {
  pub fn tokens(&self) -> Vec<Token> {
    match self {
      Kind::Option(x) => x.tokens(),
      Kind::Tuple(x) => x.iter().flat_map(|k| k.tokens()).collect(),
      Kind::Matrix((kind, literals)) => {
        let mut tokens = kind.tokens();
        for l in literals {
          tokens.append(&mut l.tokens());
        }
        tokens
      },
      Kind::Record(kinds) => {
        let mut tokens = vec![];
        for (id, kind) in kinds {
          tokens.append(&mut id.tokens());
          tokens.append(&mut kind.tokens());
        }
        tokens
      }
      Kind::Set(kind, size) => {
        let mut tokens = kind.tokens();
        if let Some(literal) = size {
          tokens.append(&mut literal.tokens());
        }
        tokens
      }
      Kind::Table((kinds, literal)) => {
        let mut tokens = vec![];
        for (id, kind) in kinds {
          tokens.append(&mut id.tokens());
          tokens.append(&mut kind.tokens());
        }
        tokens.append(&mut literal.tokens());
        tokens
      }
      Kind::Map(x, y) => x.tokens().into_iter().chain(y.tokens()).collect(),
      Kind::Scalar(x) => x.tokens(),
      Kind::Atom(x) => x.tokens(),
      Kind::Empty => vec![],
      Kind::Any => vec![],
    }
  }
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, Eq, PartialEq)]
pub enum Literal {
  Atom(Atom),
  Boolean(Token),
  Empty(Token),
  Number(Number),
  String(MechString),
  Kind(Kind),
  TypedLiteral((Box<Literal>,KindAnnotation))
}

impl Literal {
  pub fn tokens(&self) -> Vec<Token> {
    match self {
      Literal::Atom(atm) => atm.name.tokens(),
      Literal::Boolean(tkn) => vec![tkn.clone()],
      Literal::Number(x) => x.tokens(),
      Literal::String(strng) => vec![strng.text.clone()],
      Literal::Empty(tkn) => vec![tkn.clone()],
      Literal::Kind(knd) => knd.tokens(),
      Literal::TypedLiteral((lit, knd)) => {
        let mut tokens = lit.tokens();
        tokens.append(&mut knd.tokens());
        tokens
      }
    }
  }
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct MechString {
  pub text: Token,
}

pub type Hyperlink = (Token, Token);

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum ParagraphElement {
  Emphasis(Box<ParagraphElement>),
  FootnoteReference(Token),
  Highlight(Box<ParagraphElement>),
  Hyperlink(Hyperlink),
  InlineCode(Token),
  EvalInlineMechCode(Expression),
  InlineMechCode(MechCode),
  InlineEquation(Token),
  Reference(Token),
  Strikethrough(Box<ParagraphElement>),
  Strong(Box<ParagraphElement>),
  Text(Token),
  Underline(Box<ParagraphElement>),
}

impl ParagraphElement {

  pub fn to_string(&self) -> String {
    match self {
      ParagraphElement::Emphasis(t) => t.to_string(),
      ParagraphElement::FootnoteReference(t) => t.to_string(),
      ParagraphElement::Highlight(t) => t.to_string(),
      ParagraphElement::Hyperlink((t, u)) => {
        format!("[{}]({})", t.to_string(), u.to_string())
      }
      ParagraphElement::InlineCode(t) => t.to_string(),
      ParagraphElement::InlineEquation(t) => t.to_string(),
      ParagraphElement::InlineMechCode(t) => format!("{:?}", t),
      ParagraphElement::EvalInlineMechCode(t) => format!("{:?}", t),
      ParagraphElement::Reference(t) => t.to_string(),
      ParagraphElement::Strikethrough(t) => t.to_string(),
      ParagraphElement::Strong(t) => t.to_string(),
      ParagraphElement::Text(t) => t.to_string(),
      ParagraphElement::Underline(t) => t.to_string(),
    }
  }

}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Hash, Serialize, Deserialize, Eq, PartialEq)]
pub enum Number {
  Real(RealNumber),
  Complex(ComplexNumberNode),
}

impl Number {

  pub fn from_integer(x: i64) -> Number {
    Number::Real(RealNumber::Integer(Token::new(TokenKind::Digit, SourceRange::default(), x.to_string().chars().collect())))
  }

  pub fn to_string(&self) -> String {
    match self {
      Number::Real(x) => x.to_string(),
      Number::Complex(x) => x.to_string(),
    }
  }

  pub fn tokens(&self) -> Vec<Token> {
    match self {
      Number::Real(x) => x.tokens(),
      Number::Complex(x) => x.tokens(),
    }
  }
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, Eq, PartialEq)]
pub enum RealNumber {
  Binary(Token),
  Decimal(Token),
  Float((Whole,Part)),
  Hexadecimal(Token),
  Integer(Token),
  Negated(Box<RealNumber>),
  Octal(Token),
  Rational((Numerator,Denominator)),
  Scientific((Base,Exponent)),
}

impl RealNumber {
  pub fn tokens(&self) -> Vec<Token> {
    match self {
      RealNumber::Integer(tkn) => vec![tkn.clone()],
      _ => todo!(),
    }
  }
  pub fn to_string(&self) -> String {
    match self {
      RealNumber::Integer(tkn) => tkn.to_string(),
      RealNumber::Float((whole, part)) => format!("{}.{}", whole.to_string(), part.to_string()),
      RealNumber::Binary(tkn) => format!("0b{}", tkn.to_string()),
      RealNumber::Hexadecimal(tkn) => format!("0x{}", tkn.to_string()),
      RealNumber::Octal(tkn) => format!("0o{}", tkn.to_string()),
      RealNumber::Decimal(tkn) => format!("0d{}", tkn.to_string()),
      RealNumber::Rational((num, den)) => format!("{}/{}", num.to_string(), den.to_string()),
      RealNumber::Scientific(((whole,part), exponent)) => {
        let (sign, whole, part) = exponent;
        let sign_str = if *sign { "+" } else { "-" };
        let whole_str = whole.to_string();
        let part_str = part.to_string();
        format!("{}{}.{}/10^{}", whole.to_string(), part.to_string(), sign_str, whole_str)
      }
      RealNumber::Negated(x) => format!("-{}", x.to_string()),
    }
  }
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, Eq, PartialEq)]
pub struct ImaginaryNumber {
  pub number: RealNumber,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, Eq, PartialEq)]
pub struct ComplexNumberNode {
  pub real: Option<RealNumber>,
  pub imaginary: ImaginaryNumber
}

impl ComplexNumberNode {
  pub fn tokens(&self) -> Vec<Token> {
    let mut tkns = vec![];
    if let Some(r) = &self.real {
      tkns.append(&mut r.tokens());
    }
    tkns.append(&mut self.imaginary.number.tokens());
    tkns
  }

  pub fn to_string(&self) -> String {
    let mut out = "".to_string();
    if let Some(r) = &self.real {
      out.push_str(&r.to_string());
    }
    out.push_str("i");
    out
  }
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct Comment {
  pub paragraph: Paragraph,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpAssign {
  pub target: SliceRef,
  pub op: OpAssignOp,
  pub expression: Expression,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum OpAssignOp {
  Add,
  Div,
  Exp, 
  Mod,  
  Mul,
  Sub,   
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum RangeOp {
  Exclusive,      
  Inclusive,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum AddSubOp {
  Add,
  Sub,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum MulDivOp {
  Div,
  Mod,
  Mul,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum VecOp {
  Cross,
  Dot,
  MatMul,
  Solve,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExponentOp {
  Exp
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComparisonOp {
  Equal,
  GreaterThan,
  GreaterThanEqual,
  LessThan,
  LessThanEqual,
  NotEqual,
  StrictEqual,
  StrictNotEqual,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum LogicOp {
  And,
  Not,
  Or,
  Xor,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum FormulaOperator {
  AddSub(AddSubOp),
  Comparison(ComparisonOp),
  Exponent(ExponentOp),
  Logic(LogicOp),
  MulDiv(MulDivOp),
  Vec(VecOp),
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum Factor {
  Expression(Box<Expression>),
  Negate(Box<Factor>),
  Not(Box<Factor>),
  Parenthetical(Box<Factor>),
  Term(Box<Term>),
  Transpose(Box<Factor>),
}

impl Factor {
  pub fn tokens(&self) -> Vec<Token> {
    match self {
      Factor::Expression(x) => x.tokens(),
      Factor::Negate(x) => x.tokens(),
      Factor::Not(x) => x.tokens(),
      Factor::Parenthetical(x) => x.tokens(),
      Factor::Term(x) => x.tokens(),
      Factor::Transpose(x) => x.tokens(),
    }
  }
}