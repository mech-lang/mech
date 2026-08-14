//! Ephemeral source compilation for resident program artifacts.

mod compiler;

pub(crate) use compiler::ProgramCompilerView;
pub use compiler::{CompilerImportValueUnsupported, ProgramCompiler};
