// # Errors

// Defines a struct for errors and an enum which enumerates the error types

pub struct Error { 
  block: u64,
  line: usize,
  column: usize,
  message: ErrorType,
}


pub enum ErrorType {
  MissingAttribute,
}