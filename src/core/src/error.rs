use crate::*;

// Errors
// ----------------------------------------------------------------------------

// Defines a struct for errors and an enum which enumerates the error types

type Rows = usize;
type Cols = usize;
pub type ParserErrorReport = Vec<ParserErrorContext>;


#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SourceRange {
  pub file_id: u64,
  pub start: u32,   
  pub end: u32,
}

impl SourceRange {
  pub fn new(file_id: u64, start: u32, end: u32) -> Self {
    Self { file_id, start, end }
  }

  pub fn empty(file_id: u64, pos: u32) -> Self {
    Self { file_id, start: pos, end: pos }
  }

  pub fn length(&self) -> u32 {
    self.end - self.start
  }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompilerSourceRange {
  pub file: &'static str,
  pub line: u32,
}

impl CompilerSourceRange {
  pub fn here() -> Self {
    Self {
      file: file!(),
      line: line!(),
    }
  }
}

#[macro_export]
macro_rules! compiler_loc {
  () => {
    $crate::CompilerSourceRange {
      file: file!(),
      line: line!(),
    }
  };
}

pub trait MechErrorKind2: std::fmt::Debug {
  fn name(&self) -> &str;
  fn message(&self) -> String;
}

#[derive(Debug)]
pub struct MechError2 {
  pub kind: Box<dyn MechErrorKind2>,
  pub program_range: Option<SourceRange>,
  pub annotations: Vec<SourceRange>,
  pub compiler_location: Option<CompilerSourceRange>,
  pub source: Option<Box<MechError2>>, // for propagation
}

impl MechError2 {
  pub fn new<K: MechErrorKind2 + 'static>(
    kind: K,
    program_range: Option<SourceRange>
  ) -> Self {
    Self {
      kind: Box::new(kind),
      program_range,
      annotations: Vec::new(),
      compiler_location: None,
      source: None,
    }
  }

  pub fn with_compiler_loc(mut self) -> Self {
    self.compiler_location = Some(CompilerSourceRange::here());
    self
  }

  pub fn with_specific_compiler_loc(mut self, loc: CompilerSourceRange) -> Self {
    self.compiler_location = Some(loc);
    self
  }

  pub fn with_annotation(mut self, range: SourceRange) -> Self {
    self.annotations.push(range);
    self
  }

  pub fn with_annotations<I>(mut self, iter: I) -> Self
  where
    I: IntoIterator<Item = SourceRange>,
  {
    self.annotations.extend(iter);
    self
  }

  pub fn with_source(mut self, src: MechError2) -> Self {
    self.source = Some(Box::new(src));
    self
  }

  pub fn primary_range(&self) -> Option<SourceRange> {
    self.program_range
  }

  pub fn simple_message(&self) -> String {
    format!("{}: {}", self.kind.name(), self.kind.message())
  }

  pub fn full_chain_message(&self) -> String {
    let mut out = self.simple_message();
    let mut current = &self.source;

    while let Some(err) = current {
      out.push_str("\nCaused by: ");
      out.push_str(&err.simple_message());
      current = &err.source;
    }

    out
  }

  pub fn boxed(self) -> Box<Self> {
    Box::new(self)
  }
}

#[derive(Debug)]
pub struct UndefinedKindError {
  pub kind_id: u64,
}
impl MechErrorKind2 for UndefinedKindError {
  fn name(&self) -> &str {
    "UndefinedKind"
  }

  fn message(&self) -> String {
    format!("Kind `{}` is not defined.", self.kind_id)
  }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct MechError{ 
  pub id: u32,
  pub file: String,
  pub tokens: Vec<Token>,
  pub kind: MechErrorKind,
  pub msg: String,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ParserErrorContext {
  pub cause_rng: SourceRange,
  pub err_message: String,
  pub annotation_rngs: Vec<SourceRange>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum MechErrorKind {
  UndefinedField(u64),                               // Accessed a field of a record that's not defined
  UndefinedVariable(u64),                            // Accessed a variable that's not defined
  UndefinedKind(u64),                                // Used a kind that's not defined
  MissingTable(u64),                               // TableId of missing table
  WrongTableColumnKind,
  MissingBlock(u64),                             // BlockId of missing block
  PendingExpression,                              // id of pending variable
  PendingTable(u64),                             // TableId of pending table                          
  DimensionMismatch(Vec<(Rows,Cols)>),               // Argument dimensions are mismatched ((row,col),(row,col))
  KindMismatch(ValueKind,ValueKind),
  UnhandledIndexKind,
  //MissingColumn((TableId,TableIndex)),             // The identified table is missing a needed column
  //ColumnKindMismatch(Vec<ValueKind>),              // Excepted kind versus given kind
  //SubscriptOutOfBounds(((Rows,Cols),(Rows,Cols))), // (target) vs (actual) index
  LinearSubscriptOutOfBounds((Rows,Rows)),           // (target) vs (actual) index
  IndexOutOfBounds,                 
  //DomainMismatch(u64, u64),                        // domain IDs (target vs actual)
  MissingFunction(u64),                              // ID of missing function
  //TransformationPending(Transformation),           // Block is unsatisfied so the transformation is not added
  //IncorrectFunctionArgumentType,
  ZeroIndex,                                         // Zero cannot ever be used as an index.
  VariableRedefined(u64),
  NotMutable(u64), 
  BlockDisabled,
  IoError,
  UnhandledFormulaOperator(FormulaOperator),
  FeatureNotEnabled(String), // Feature is not enabled in the current build
  GenericError(String),
  FileNotFound(String),
  Unhandled,
  OutputUndefinedInFunctionBody(u64),
  UnknownFunctionArgument(u64),
  UnknownColumnKind(u64),
  UnknownEnumVairant(u64,u64),
  UnableToConvertValueKind,
  UnhandledFunctionArgumentKind,
  CouldNotAssignKindToValue,
  ExpectedNumericForSize,                            // When something non-numeric is passed as a size
  MatrixMustHaveHomogenousKind,                      // When multiple element kinds are specified for a matrix
  IncorrectNumberOfArguments,
  //UnhandledTableShape(TableShape),
  TooManyInputArguments(usize,usize),                // (given,expected)
  ParserError(ParserErrorReport, String),
  //MissingCapability(Capability),
  InvalidCapabilityToken,
  UnknownReplCommand(String),
  NoCode,
  None,
}

#[cfg(not(feature = "no_std"))]
impl From<std::io::Error> for MechError{ 
  fn from(n: std::io::Error) -> MechError{ 
    MechError{ 
      id: line!(),
      file: file!().to_string(),
      tokens: vec![],
      kind: MechErrorKind::IoError,
      msg: n.to_string(),
    }
  } 
}

#[cfg(feature = "no_std")]
impl From<()> for MechError {
  fn from(_: ()) -> Self {
    MechError {
      id: line!(),
      file: file!().to_string(),
      tokens: Vec::new(),
      kind: MechErrorKind::IoError,
      msg: "embedded-io error".into(),
    }
  }
}

impl From<std::io::Error> for MechError2 {
  fn from(err: std::io::Error) -> Self {
    MechError2::new(
      IoErrorWrapper { msg: err.to_string() },
      None
    )
    .with_compiler_loc()
  }
}

#[derive(Debug)]
pub struct IoErrorWrapper {
  pub msg: String,
}
impl MechErrorKind2 for IoErrorWrapper {
  fn name(&self) -> &str { "IoError" }

  fn message(&self) -> String {
    format!("IO error: {}", self.msg)
  }
}

/*
impl fmt::Debug for MechErrorKind {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      _ => write!(f,"No Format")?;
    }
    Ok(())
  }
}*/