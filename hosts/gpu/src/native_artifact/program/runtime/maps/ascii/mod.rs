//! Owner-prepared ASCII mapping representation, shared by both native mappers.
//! The fixed 144-byte allocation stays within the existing 256-byte allowance.
//! A projected table is admitted only after exact finite-domain equivalence;
//! incompatible immutable maps keep all 128 cells. No input content is sampled.
pub(in crate::native_artifact) const HEADER: usize = 16;
pub(in crate::native_artifact) const BYTES: usize = HEADER + 128;
pub(in crate::native_artifact) const MISSING: u8 = 128;
pub(in crate::native_artifact) const KIND: usize = 0;
pub(in crate::native_artifact) const DOMAIN_MASK: usize = 1;
pub(in crate::native_artifact) const DOMAIN_VALUE: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_artifact) enum MappingKind {
    Full,
    Projected,
}
impl MappingKind {
    #[cfg(feature = "canonical-native-program")]
    pub const fn cells(self) -> usize {
        match self {
            Self::Full => 128,
            Self::Projected => 32,
        }
    }
    #[cfg(feature = "canonical-native-program")]
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Projected => "projected",
        }
    }
}

/// Input cells begin at HEADER and use the existing high-bit missing contract.
/// No allocation or source traversal is performed here. Preparation is bounded
/// by three traversals of 128 cells inside the original fixed work envelope.
pub(in crate::native_artifact) fn prepare(table: &mut [u8]) -> MappingKind {
    assert_eq!(table.len(), BYTES);
    table[..HEADER].fill(0);
    let cells = &mut table[HEADER..];
    let mut first = None;
    let mut differing = 0;
    for (key, value) in cells.iter_mut().enumerate() {
        if *value >= MISSING {
            *value = MISSING;
        } else {
            let first = *first.get_or_insert(key as u8);
            differing |= first ^ key as u8;
        }
    }
    // Only high index bits participate in the domain predicate. The low five
    // bits select one of 32 cells. Every high-bit combination is derived from
    // this actual immutable table, without an alphabet or workload constant.
    let mask = !differing & 0x60;
    let expected = first.unwrap_or(0) & mask;
    let mut projected = [MISSING; 32];
    let mut seen = 0u32;
    for (key, &value) in cells.iter().enumerate() {
        if key as u8 & mask != expected {
            continue;
        }
        let index = key & 31;
        let bit = 1u32 << index;
        if seen & bit != 0 && projected[index] != value {
            return MappingKind::Full;
        }
        seen |= bit;
        projected[index] = value;
    }
    // Check the complete ASCII domain, including every absent key and valid
    // zero. A matching projection must preserve both value and missing status.
    for (key, &value) in cells.iter().enumerate() {
        let candidate = if key as u8 & mask == expected {
            projected[key & 31]
        } else {
            MISSING
        };
        if candidate != value {
            return MappingKind::Full;
        }
    }
    cells[..32].copy_from_slice(&projected);
    table[KIND] = 1;
    table[DOMAIN_MASK] = mask;
    table[DOMAIN_VALUE] = expected;
    MappingKind::Projected
}

#[cfg(all(test, feature = "canonical-native-program-host"))]
pub(in crate::native_artifact) fn test_table(cells: &[u16; 128]) -> [u8; BYTES] {
    let mut table = [MISSING; BYTES];
    for (output, &value) in table[HEADER..].iter_mut().zip(cells) {
        *output = if value < 128 { value as u8 } else { MISSING };
    }
    prepare(&mut table);
    table
}

#[cfg(test)]
mod tests;
