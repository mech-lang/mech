//! Shared text length semantics for maintained leaves and compiled generators.
//!
//! In ASCII the only multi-byte grapheme is CRLF. Use byte scans for that
//! representation; retain the canonical Unicode segmenter for every other
//! input. No alphabet, program identity, or cached owner assumption is involved.

/// Recognize the ASCII multi-byte grapheme without allocating an index.
/// A byte-absence scan can skip pair comparisons entirely. If CR is present,
/// retain the exact pair test (including isolated CR, NUL and scan boundaries).
/// Callers still pre-admit the complete scan, including a declined fast path.
pub fn has_crlf(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.contains(&b'\r') && bytes.windows(2).any(|pair| pair == b"\r\n")
}

/// Count extended graphemes without constructing an index or allocating.
/// Callers still admit the complete linear scan before invoking this function.
pub fn count(text: &str) -> usize {
    if text.is_ascii() {
        if !text.as_bytes().contains(&b'\r') {
            return text.len();
        }
        text.len()
            - text
                .as_bytes()
                .windows(2)
                .filter(|pair| *pair == b"\r\n")
                .count()
    } else {
        grapheme::Graphemes::from_usvs(text).iter().count()
    }
}
