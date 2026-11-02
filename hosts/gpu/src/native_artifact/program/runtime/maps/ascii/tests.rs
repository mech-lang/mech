use crate::native_artifact::program::runtime::maps::ascii::*;

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
