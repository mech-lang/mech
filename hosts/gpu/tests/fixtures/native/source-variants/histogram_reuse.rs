//! Explicit source experiment; the pinned compressor and compiler are unchanged.
//! Reuse the all-literal candidate's histogram when choosing initial costs.

fn replace_once(source: &str, old: &str, new: &str) -> String {
    assert_eq!(source.matches(old).count(), 1, "source witness drifted");
    source.replacen(old, new, 1)
}

pub fn cost_source(pinned: &str) -> String {
    let prefix = concat!(
        "optimal-choose-default-costs(input<[u8]>, begin<u64>, length<u64>, match-freqs<[u32]>, depth<u64>, table0<[u8]>, table1<[u8]>, table2<[u8]>, symbol-costs<[u8]>) => (out<optimal-default-costs>) :=\n",
        "  zero := [0u32 | i <- 0u64..=256u64, i < 256u64]\n",
        "  cutoff := bits/shift-right-wrap(length, 11u64)\n",
        "  counts := optimal-cost-count-literals(zero, input, begin, length, 0u64)\n",
    );
    let end = "  out := {lit-cost: lit-cost, len-sym-cost: sym-cost}.\n";
    assert_eq!(pinned.matches(prefix).count(), 1, "source witness drifted");
    let start = pinned.find(prefix).unwrap() + prefix.len();
    let finish = start + pinned[start..].find(end).unwrap() + end.len();
    // Derive the unchanged pricing body from the pinned function itself. Keep
    // its original public entry and operation/error order byte-for-byte intact.
    let helper = format!(
        "optimal-choose-default-costs-from-counts(counts<[u32]>, length<u64>, match-freqs<[u32]>, depth<u64>, table0<[u8]>, table1<[u8]>, table2<[u8]>, symbol-costs<[u8]>) => (out<optimal-default-costs>) :=\n  cutoff := bits/shift-right-wrap(length, 11u64)\n{}",
        &pinned[start..finish],
    );
    format!("{}{}{}", &pinned[..finish], helper, &pinned[finish..])
}

pub fn block_source(pinned: &str) -> String {
    replace_once(
        pinned,
        "  defaults := optimal-choose-default-costs(input, begin, length, frequencies, 300u64, table0, table1, table2, symbol-costs)\n",
        "  defaults := optimal-choose-default-costs-from-counts(literals.freqs-litlen, length, frequencies, 300u64, table0, table1, table2, symbol-costs)\n",
    )
}

pub fn source(pinned: &str) -> String {
    block_source(&cost_source(pinned))
}
