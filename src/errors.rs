// # Errors

// Defines a struct for errors and an enum which enumerates the error types

// ## Prelude

use table::{Index};
use block::Transformation;

// ## The Error Struct

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Error { 
  pub block: u64,
  pub constraint: Vec<Transformation>,
  pub error_id: ErrorType,
}


#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum ErrorType {
  MissingAttribute(Index),
  IndexOutOfBounds(((u64, u64), (u64, u64))),
  DuplicateAlias(u64),
  DomainMismatch(u64, u64),
  UnsatisfiedConstraint(Vec<u64>),
}
