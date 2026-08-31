use crate::*;
#[cfg(all(feature = "invariant_define", feature = "no_std", not(feature = "std")))]
use alloc::collections::BTreeMap;
#[cfg(all(
    feature = "invariant_define",
    any(not(feature = "no_std"), feature = "std")
))]
use std::collections::BTreeMap;

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
#[cfg(feature = "invariant_define")]
#[derive(Clone, Debug)]
pub struct IntegrityConstraint {
    pub id: u64,
    pub name: String,
    pub expression: String,
    pub result: ValueCell,
    pub lhs: Option<ValueCell>,
    pub operator: Option<FormulaOperator>,
    pub rhs: Option<ValueCell>,
    pub tokens: Vec<Token>,
}
#[cfg(feature = "invariant_define")]
pub type IntegrityConstraintTable = BTreeMap<u64, IntegrityConstraint>;
