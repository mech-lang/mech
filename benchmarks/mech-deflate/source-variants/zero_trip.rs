//! Explicit source experiment. The pinned compressor remains unchanged.
//! Restore the zero-trip guards of the reference's match and skip loops.

fn replace_once(source: &str, original: &str, replacement: &str) -> String {
    assert_eq!(
        source.matches(original).count(),
        1,
        "source witness drifted"
    );
    source.replacen(original, replacement, 1)
}

pub fn path_source(pinned: &str) -> String {
    replace_once(
        pinned,
        "  choice := #OptimalPathMatches(nodes, cache, header-ptr, number, length-costs, offset-costs, slots, best, item)\n",
        "  choice := ((number != 0u64)?\n    | true => #OptimalPathMatches(nodes, cache, header-ptr, number, length-costs, offset-costs, slots, best, item)\n    | false => {best: best, item: item, cache-ptr: header-ptr})\n",
    )
}

pub fn collector_source(pinned: &str) -> String {
    replace_once(
        pinned,
        "  advanced := optimal-skip-run(mf, matches, nh3, nh4, input, at, current-base, slide-at, max-length, nice-length, skip, ptr)\n",
        "  advanced := ((skip != 0u64)?\n    | true => optimal-skip-run(mf, matches, nh3, nh4, input, at, current-base, slide-at, max-length, nice-length, skip, ptr)\n    | false => {tables: mf, cache: matches, next-hash3: nh3, next-hash4: nh4, next-position: at, cache-ptr: ptr, base: current-base, next-slide: slide-at, maximum: max-length, nice: nice-length, skip: skip})\n",
    )
}

pub fn source(pinned: &str) -> String {
    collector_source(&path_source(pinned))
}
