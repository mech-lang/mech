// # Errors

// Defines a struct for errors and an enum which enumerates the error types

#[derive(Clone, Debug, PartialEq)]
pub struct Error { 
  pub block: u64,
  pub constraint: usize,
  pub line: usize,
  pub column: usize,
  pub error_id: ErrorType,
}


#[derive(Clone, Debug, PartialEq)]
pub enum ErrorType {
  MissingAttribute(u64),
  DuplicateAlias(u64),
}