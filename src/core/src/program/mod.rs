use crate::*;
#[cfg(feature = "no_std")]
use alloc::collections::BTreeMap;
#[cfg(feature = "no_std")]
use hashbrown::HashSet;
#[cfg(not(feature = "no_std"))]
use std::collections::{BTreeMap, HashSet};

#[cfg(any(feature = "compiler", feature = "program"))]
pub mod compiler;
#[cfg(feature = "program")]
pub mod program;
#[cfg(feature = "symbol_table")]
pub mod symbol_table;

#[cfg(any(feature = "compiler", feature = "program"))]
pub use self::compiler::*;
#[cfg(feature = "program")]
pub use self::program::*;
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

pub fn parse_version_to_u16(s: &str) -> Option<u16> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 {
        return None;
    }

    let major = parts[0].parse::<u16>().ok()?;
    let minor = parts[1].parse::<u16>().ok()?;
    let patch = parts[2].parse::<u16>().ok()?; // parse to u16 to check bounds easily

    if major > 0b111 {
        return None;
    } // 3 bits => 0..7
    if minor > 0b1_1111 {
        return None;
    } // 5 bits => 0..31
    if patch > 0xFF {
        return None;
    } // 8 bits => 0..255

    // Pack: major in bits 15..13, minor in bits 12..8, patch in bits 7..0
    let encoded = (major << 13) | (minor << 8) | patch;
    Some(encoded as u16)
}

#[derive(Debug, Clone)]
pub struct InvalidMagicNumberError;

impl MechErrorKind for InvalidMagicNumberError {
    fn name(&self) -> &str {
        "InvalidMagicNumber"
    }
    fn message(&self) -> String {
        "Invalid magic number".to_string()
    }
}
