// # Errors

// Defines a struct for errors and an enum which enumerates the error types

// ## Prelude

use crate::*;
use crate::nodes::SourceRange;

type Rows = usize;
type Cols = usize;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MechError {
  pub id: u64,
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
  MissingTable(TableId),                             // TableId of missing table
  MissingBlock(BlockId),                             // BlockId of missing block
  PendingTable(TableId),                             // TableId of pending table                          
  DimensionMismatch(Vec<(Rows,Cols)>),      // Argument dimensions are mismatched ((row,col),(row,col))
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
  UnknownFunctionArgument(u64),
  UnknownColumnKind(u64),
  UnhandledFunctionArgumentKind(ValueKind),
  UnhandledTableShape(TableShape),
  TooManyInputArguments(usize,usize),                // (given,expected)
  ParserError(nodes::ParserNode, ParserErrorReport, String),
  MissingCapability(Capability),
  InvalidCapabilityToken,
  None,
}

impl From<std::io::Error> for MechError {
  fn from(n: std::io::Error) -> MechError {
    MechError{msg: "".to_string(), id: 74892, kind: MechErrorKind::IoError}
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