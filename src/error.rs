// # Errors

// Defines a struct for errors and an enum which enumerates the error types

// ## Prelude

use crate::table::{TableIndex, TableId};

// ## The Error Struct

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct MechError { 
  pub block_id: u64,
  pub step_text: String,
  pub error_type: MechErrorKind,
}

type Rows = usize;
type Cols = usize;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum MechErrorKind {
  MissingTable(TableId),                            // TableId of missing table
  DimensionMismatch(((Rows,Cols),(Rows,Cols))), // Argument dimensions are mismatched ((row,col),(row,col))
  MissingColumn((TableId,TableIndex)),              // The identified table is missing a needed column
  //MissingAttribute(TableIndex),
  //IndexOutOfBounds(((u64, u64), (u64, u64))), // (target) vs (actual) index
  //DuplicateAlias(u64),                        // Alias ID
  //DomainMismatch(u64, u64),                   // domain IDs (target vs actual)
  //MissingFunction(u64),                       // ID of missing function
  //IncorrectFunctionArgumentType,
}
