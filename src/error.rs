// # Errors

// Defines a struct for errors and an enum which enumerates the error types

// ## Prelude

use crate::*;

type Rows = usize;
type Cols = usize;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MechError {
  pub id: u64,
  pub kind: MechErrorKind,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum MechErrorKind {
  MissingTable(TableId),                             // TableId of missing table
  DimensionMismatch(((Rows,Cols),(Rows,Cols))),     // Argument dimensions are mismatched ((row,col),(row,col))
  //MissingColumn((TableId,TableIndex)),             // The identified table is missing a needed column
  //ColumnKindMismatch(Vec<ValueKind>),              // Excepted kind versus given kind
  //SubscriptOutOfBounds(((Rows,Cols),(Rows,Cols))), // (target) vs (actual) index
  LinearSubscriptOutOfBounds((Rows,Rows)),           // (target) vs (actual) index
  //DuplicateAlias(u64),                             // Alias ID
  //DomainMismatch(u64, u64),                        // domain IDs (target vs actual)
  MissingFunction(u64),                              // ID of missing function
  //TransformationPending(Transformation),           // Block is unsatisfied so the transformation is not added
  //IncorrectFunctionArgumentType,
  ZeroIndex,                                         // Zero cannot ever be used as an index.
  BlockDisabled,
  GenericError(String),
  Unhandled,
  UnknownFunctionArgument(u64),
  UnknownColumnKind(u64),
  TooManyInputArguments(usize,usize),                // (given,expected)
  None,
}
