//! Owner-prepared ASCII mapping representation, shared by both native mappers.
//! The fixed 144-byte allocation stays within the existing 256-byte allowance.
//! A projected table is admitted only after exact finite-domain equivalence;
//! incompatible immutable maps keep all 128 cells. No input content is sampled.
pub(super) const HEADER: usize = 16;
pub(super) const BYTES: usize = HEADER + 128;
pub(super) const MISSING: u8 = 128;
pub(super) const KIND: usize = 0;
pub(super) const DOMAIN_MASK: usize = 1;
pub(super) const DOMAIN_VALUE: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MappingKind {
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
pub(super) fn prepare(table: &mut [u8]) -> MappingKind {
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
pub(super) fn test_table(cells: &[u16; 128]) -> [u8; BYTES] {
    let mut table = [MISSING; BYTES];
    for (output, &value) in table[HEADER..].iter_mut().zip(cells) {
        *output = if value < 128 { value as u8 } else { MISSING };
    }
    prepare(&mut table);
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded(cells: &[u8; 128]) -> (MappingKind, [u8; BYTES]) {
        let mut table = [MISSING; BYTES];
        table[HEADER..].copy_from_slice(cells);
        (prepare(&mut table), table)
    }
    fn assert_equivalent(cells: &[u8; 128], kind: MappingKind, table: &[u8; BYTES]) {
        for key in 0..128 {
            let actual = match kind {
                MappingKind::Full => table[HEADER + key],
                MappingKind::Projected if key as u8 & table[DOMAIN_MASK] == table[DOMAIN_VALUE] => {
                    table[HEADER + (key & 31)]
                }
                MappingKind::Projected => MISSING,
            };
            assert_eq!(actual, cells[key].min(MISSING), "key={key}");
        }
    }

    #[test]
    fn projected_domains_preserve_every_ascii_value_missing_cell_and_zero() {
        for mask in [0, 32, 64, 96] {
            for domain in [0, 32, 64, 96] {
                for seed in 0..128 {
                    let cells = core::array::from_fn(|key| {
                        if key as u8 & mask == domain & mask && (key & 31) % 7 != 0 {
                            ((key & 31) as u8).wrapping_mul(37).wrapping_add(seed) & 127
                        } else {
                            MISSING
                        }
                    });
                    let (kind, table) = encoded(&cells);
                    assert_eq!(kind, MappingKind::Projected);
                    assert_equivalent(&cells, kind, &table);
                }
            }
        }
        for marker in 128..=255 {
            let cells = [marker; 128];
            let (kind, table) = encoded(&cells);
            assert_equivalent(&cells, kind, &table);
        }
    }

    #[test]
    fn conflicting_projections_keep_the_complete_table_without_changing_values() {
        let cells = core::array::from_fn(|key| ((key * 37 + 11) % 128) as u8);
        let (kind, table) = encoded(&cells);
        assert_eq!(kind, MappingKind::Full);
        assert_equivalent(&cells, kind, &table);
        let mut cells = [MISSING; 128];
        cells[1] = 0;
        cells[33] = 1;
        let (kind, table) = encoded(&cells);
        assert_eq!(kind, MappingKind::Full);
        assert_equivalent(&cells, kind, &table);
    }
}
