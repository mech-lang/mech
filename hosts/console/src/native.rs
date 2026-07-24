use std::io::Write;

use mech_core::MResult;

use crate::{console_error, ConsoleBackend, ConsoleHostFactory};

#[derive(Clone, Copy, Debug, Default)]
pub struct NativeConsoleBackend;

impl ConsoleBackend for NativeConsoleBackend {
  fn write_line(&mut self, text: &str) -> MResult<()> {
    let mut out = std::io::stdout().lock();
    out.write_all(text.as_bytes()).map_err(|err| console_error("console://output", err.to_string()))?;
    out.write_all(b"\n").map_err(|err| console_error("console://output", err.to_string()))?;
    out.flush().map_err(|err| console_error("console://output", err.to_string()))
  }
}

pub type NativeConsoleHostFactory = ConsoleHostFactory<NativeConsoleBackend>;

impl NativeConsoleHostFactory {
  pub fn new() -> MResult<Self> { Self::with_backend(NativeConsoleBackend) }
}
