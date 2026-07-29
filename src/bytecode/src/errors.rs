use mech_core::MechErrorKind;

#[derive(Debug, Clone)]
pub struct BufferPositionMismatchError {
  pub expected: u64,
  pub got: u64,
}

impl MechErrorKind for BufferPositionMismatchError {
  fn name(&self) -> &str { "BufferPositionMismatch" }

  fn message(&self) -> String {
    format!("Buffer position mismatch: expected {}, got {}", self.expected, self.got)
  }
}

#[derive(Debug, Clone)]
pub struct FinalBufferLengthMismatchError {
  pub expected: u64,
  pub got: u64,
}

impl MechErrorKind for FinalBufferLengthMismatchError {
  fn name(&self) -> &str { "FinalBufferLengthMismatch" }

  fn message(&self) -> String {
    format!("Final buffer length mismatch: expected {}, got {}", self.expected, self.got)
  }
}
