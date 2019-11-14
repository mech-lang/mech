// # Errors

// Defines a struct for errors and an enum which enumerates the error types

// ## Prelude

use table::{Index};
use runtime::Constraint;

// ## The Error Struct

#[derive(Clone, Debug, PartialEq)]
pub struct Error { 
  pub block: u64,
  pub constraint: Constraint,
  pub error_id: ErrorType,
}


#[derive(Clone, Debug, PartialEq)]
pub enum ErrorType {
  MissingAttribute(Index),
  IndexOutOfBounds(((u64, u64), (u64, u64))),
  DuplicateAlias(u64),
  DomainMismatch(u64, u64),
}