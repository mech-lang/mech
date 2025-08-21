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
pub type EnumTable = HashMap<u64, MechEnum>;

#[derive(Clone, Debug)]
pub struct ProgramState {
  pub symbol_table: SymbolTable,
  pub functions: FunctionsRef,
  pub plan: Plan,
  pub kinds: KindTable,
  pub enums: EnumTable,
  pub dictionary: Ref<Dictionary>,
}