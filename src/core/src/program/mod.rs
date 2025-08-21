use crate::*;

#[cfg(feature = "compiler")]
pub mod compiler;
#[cfg(feature = "symbol_table")]
pub mod symbol_table;

#[cfg(feature = "compiler")]
pub use self::compiler::*;
#[cfg(feature = "symbol_table")]
pub use self::symbol_table::*;

// Program State
// ----------------------------------------------------------------------------

pub type KindTable = HashMap<u64, ValueKind>;
#[cfg(feature = "enum")]
pub type EnumTable = HashMap<u64, MechEnum>;

#[derive(Clone)]
pub struct ProgramState {
  #[cfg(feature = "symbol_table")]
  pub symbol_table: SymbolTable,
  #[cfg(feature = "functions")]
  pub functions: FunctionsRef,
  #[cfg(feature = "functions")]
  pub plan: Plan,
  pub kinds: KindTable,
  #[cfg(feature = "enum")]
  pub enums: EnumTable,
  pub dictionary: Ref<Dictionary>,
}