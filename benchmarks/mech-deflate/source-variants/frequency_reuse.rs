//! Separate source experiment: keep the current path and reuse deterministic
//! codes/pricing only when its current symbol frequencies equal the prior pass.

fn replace_once(source: &str, old: &str, new: &str) -> String {
    assert_eq!(source.matches(old).count(), 1, "source witness drifted");
    source.replacen(old, new, 1)
}

pub fn tally_source(pinned: &str) -> String {
    let header = "optimal-tally-and-build-codes(nodes<[u64]>, block-length<u64>, lengths<[u8]>, slots<[u8]>, reversed<[u8]>) => (out<optimal-tally>) :=\n";
    let split = "  litlen-lens0 := [0u8 | i <- 0u64..=288u64, i < 288u64]\n";
    assert_eq!(pinned.matches(header).count(), 1, "source witness drifted");
    let start = pinned.find(header).unwrap() + header.len();
    let middle = start + pinned[start..].find(split).unwrap();
    let end = middle + pinned[middle..].find(".\n").unwrap() + 2;
    // Keep the original combined entry intact for every other caller. Derive
    // both halves from it, retaining the real count workspace and code arrays.
    let helpers = format!(
        concat!(
            "<optimal-frequency-counts> := <{{freqs-litlen<[u32]> freqs-offset<[u32]>}}>\n",
            "optimal-frequency-count-path(nodes<[u64]>, block-length<u64>, lengths<[u8]>, slots<[u8]>) => (out<optimal-frequency-counts>) :=\n",
            "{}",
            "  out := {{freqs-litlen: freqs-litlen, freqs-offset: offset-counts}}.\n",
            "optimal-frequency-build-codes(freqs-litlen<[u32]>, offset-counts<[u32]>, reversed<[u8]>) => (out<optimal-tally>) :=\n",
            "{}",
        ),
        &pinned[start..middle],
        &pinned[middle..end],
    );
    format!("{}{}{REUSE}\n{}", &pinned[..end], helpers, &pinned[end..])
}

const REUSE: &str = r#"
optimal-frequency-equal(left<[u32]>, right<[u32]>, i<u64>, count<u64>) => (out<bool>) :=
  out := ((i < count)?
    | true => ((huff-get32(left, i) == huff-get32(right, i))?
      | true => optimal-frequency-equal(left, right, i + 1u64, count)
      | false => false)
    | false => true).
optimal-frequency-matches(current-litlen<[u32]>, current-offset<[u32]>, previous-litlen<[u32]>, previous-offset<[u32]>, valid<bool>) => (out<bool>) :=
  out := (valid?
    | true => (optimal-frequency-equal(current-litlen, previous-litlen, 0u64, 288u64)?
      | true => optimal-frequency-equal(current-offset, previous-offset, 0u64, 32u64)
      | false => false)
    | false => false).
optimal-frequency-select-codes(current-litlen<[u32]>, current-offset<[u32]>, previous-litlen-lens<[u8]>, previous-litlen-words<[u32]>, previous-offset-lens<[u8]>, previous-offset-words<[u32]>, same<bool>, reversed<[u8]>) => (out<optimal-tally>) :=
  out := (same?
    | true => {freqs-litlen: current-litlen, freqs-offset: current-offset, litlen-lens: previous-litlen-lens, litlen-codewords: previous-litlen-words, offset-lens: previous-offset-lens, offset-codewords: previous-offset-words}
    | false => optimal-frequency-build-codes(current-litlen, current-offset, reversed)).
"#;

pub fn source(pinned: &str) -> String {
    let source = tally_source(pinned);
    let signature = "optimal-optimize-loop(block-length<u64>, cache<[u32]>, cache-end<u64>, nodes0<[u64]>, literal0<[u32]>, length0<[u32]>, offset0<[u32]>, saved-literal<[u32]>, saved-length<[u32]>, saved-offset<[u32]>, best<u64>, remaining<u64>, ";
    let source = replace_once(
        &source,
        signature,
        &format!(
            "{signature}previous-litlen<[u32]>, previous-offset<[u32]>, previous-litlen-lens<[u8]>, previous-litlen-words<[u32]>, previous-offset-lens<[u8]>, previous-offset-words<[u32]>, previous-valid<bool>, ",
        ),
    );
    let old = concat!(
        "  tallied := optimal-tally-and-build-codes(path.nodes, block-length, length-slots, offset-slots, reversed)\n",
        "  actual-cost := optimal-compute-true-cost(tallied.litlen-lens, tallied.offset-lens, tallied.freqs-litlen, tallied.freqs-offset, permutation, extra-precode, extra-lengths, extra-offsets, reversed)\n",
    );
    let new = concat!(
        "  counted := optimal-frequency-count-path(path.nodes, block-length, length-slots, offset-slots)\n",
        "  same := optimal-frequency-matches(counted.freqs-litlen, counted.freqs-offset, previous-litlen, previous-offset, previous-valid)\n",
        "  tallied := optimal-frequency-select-codes(counted.freqs-litlen, counted.freqs-offset, previous-litlen-lens, previous-litlen-words, previous-offset-lens, previous-offset-words, same, reversed)\n",
        "  actual-cost := (same? | true => best | false => optimal-compute-true-cost(tallied.litlen-lens, tallied.offset-lens, tallied.freqs-litlen, tallied.freqs-offset, permutation, extra-precode, extra-lengths, extra-offsets, reversed))\n",
    );
    let source = replace_once(&source, old, new);
    let source = replace_once(
        &source,
        "next-saved-offset, next-best, passes, length-slots,",
        concat!(
            "next-saved-offset, next-best, passes, tallied.freqs-litlen, tallied.freqs-offset, ",
            "tallied.litlen-lens, tallied.litlen-codewords, tallied.offset-lens, tallied.offset-codewords, true, length-slots,",
        ),
    );
    replace_once(
        &source,
        "saved-offset, 18446744073709551615u64, 10u64, length-slots,",
        concat!(
            "saved-offset, 18446744073709551615u64, 10u64, initial-freqs-litlen, initial-freqs-offset, ",
            "initial-litlen-lens, initial-litlen-codewords, initial-offset-lens, initial-offset-codewords, false, length-slots,",
        ),
    )
}
