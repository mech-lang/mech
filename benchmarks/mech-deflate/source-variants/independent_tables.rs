//! Explicit application experiment; the pinned assembly remains the baseline.
const LOOPS: [&str; 9] = [
    "optimal-default-literals",
    "optimal-default-lengths",
    "optimal-default-offsets",
    "optimal-code-literals",
    "optimal-code-lengths",
    "optimal-code-offsets",
    "optimal-adjust-literals",
    "optimal-adjust-lengths",
    "optimal-adjust-offsets",
];
const OVERRIDES: &str = include_str!("independent-cost-tables.mec");

/// Retain each original function as its sequential fallback. Only the exact
/// named benchmark functions are rewritten; this is not compiler recognition.
pub fn source(pinned: &str) -> String {
    let mut source = pinned.to_owned();
    for name in LOOPS {
        let call = format!("{name}(");
        let starts = source
            .match_indices(&call)
            .filter(|(at, _)| *at == 0 || source.as_bytes()[*at - 1] == b'\n')
            .map(|(at, _)| at)
            .collect::<Vec<_>>();
        assert_eq!(starts.len(), 1, "expected the pinned definition of {name}");
        let start = starts[0];
        let end = start + source[start..].find(".\n").expect("function terminator") + 2;
        let fallback = source[start..end].replace(&call, &format!("{name}-sequential("));
        source.replace_range(start..end, &fallback);
    }
    source.push('\n');
    source.push_str(OVERRIDES);
    source.push('\n');
    source
}
