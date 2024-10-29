// # Errors

// Defines a struct for errors and an enum which enumerates the error types

// ## Prelude

use crate::*;
use crate::nodes::{SourceRange, Token};

type Rows = usize;
type Cols = usize;
pub type ParserErrorReport = Vec<ParserErrorContext>;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MechError {
  pub id: u32,
  pub tokens: Vec<Token>,
  pub kind: MechErrorKind,
  pub msg: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ParserErrorContext {
  pub cause_rng: SourceRange,
  pub err_message: String,
  pub annotation_rngs: Vec<SourceRange>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
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
  DuplicateAlias(u64),                               // Alias ID
  //DomainMismatch(u64, u64),                        // domain IDs (target vs actual)
  MissingFunction(u64),                              // ID of missing function
  //TransformationPending(Transformation),           // Block is unsatisfied so the transformation is not added
  //IncorrectFunctionArgumentType,
  ZeroIndex,                                         // Zero cannot ever be used as an index.
  BlockDisabled,
  IoError,
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
  None,
}

impl From<std::io::Error> for MechError {
  fn from(n: std::io::Error) -> MechError {
    MechError{tokens: vec![], msg: "".to_string(), id: 74892, kind: MechErrorKind::IoError}
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