use crate::*;

#[cfg(feature = "program")]
pub mod bytecode;
#[cfg(feature = "semantic-compiler")]
pub mod compiler;
#[cfg(feature = "symbol_table")]
pub mod symbol_table;

#[cfg(feature = "program")]
pub use self::bytecode::*;
#[cfg(feature = "semantic-compiler")]
pub use self::compiler::*;
#[cfg(feature = "symbol_table")]
pub use self::symbol_table::*;

// Program State
// ----------------------------------------------------------------------------

pub type Dictionary = HashMap<u64, String>;
pub type NamedSchemaTable = HashMap<u64, SchemaBody>;
#[cfg(feature = "enum")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalEnumVariant {
    pub id: u64,
    pub name: String,
    pub payload: Option<SchemaBody>,
}
#[cfg(feature = "enum")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalEnumDefinition {
    pub id: u64,
    pub name: String,
    pub variants: Box<[CanonicalEnumVariant]>,
}
#[cfg(feature = "enum")]
pub type EnumTable = HashMap<u64, CanonicalEnumDefinition>;
