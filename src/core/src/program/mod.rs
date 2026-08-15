use crate::*;
#[cfg(feature = "no_std")]
use alloc::collections::BTreeMap;
#[cfg(not(feature = "no_std"))]
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
pub type KindTable = HashMap<u64, ValueKind>;
#[cfg(feature = "enum")]
pub type EnumTable = HashMap<u64, MechEnum>;
#[cfg(feature = "invariant_define")]
#[derive(Clone, Debug)]
pub struct IntegrityConstraint {
    pub id: u64,
    pub name: String,
    pub expression: String,
    pub result: ValRef,
    pub lhs: Option<ValRef>,
    pub operator: Option<FormulaOperator>,
    pub rhs: Option<ValRef>,
    pub tokens: Vec<Token>,
}
#[cfg(feature = "invariant_define")]
pub type IntegrityConstraintTable = BTreeMap<u64, IntegrityConstraint>;
