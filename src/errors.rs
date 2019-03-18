// # Errors

// Defines a struct for errors and an enum which enumerates the error types

#[derive(Clone, Debug)]
pub struct Error { 
  pub block: u64,
  pub line: usize,
  pub column: usize,
  pub error_id: ErrorType,
}


#[derive(Clone, Debug)]
pub enum ErrorType {
  MissingAttribute,
  DuplicateAlias,
}