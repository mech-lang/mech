//! Literal recognition keeps both Unicode cursors and comparison progress.
//! Matching never treats a temporary source frontier as a final grapheme end.
use super::grapheme_scan::{GraphemeScan, ScanProgress, ScanSource};
use crate::document::{TextRange, TextSize};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LiteralProgress {
    Complete(Option<TextSize>),
    NeedInput,
    NeedsProcessing,
    InvalidSource,
}

pub(crate) struct LiteralScan<'l> {
    literal: &'l str,
    literal_end: TextSize,
    final_context: Option<TextSize>,
    expected: GraphemeScan,
    actual: GraphemeScan,
    expected_range: Option<TextRange>,
    actual_range: Option<TextRange>,
    compared: u32,
    prefix_checked: bool,
    pub comparison_bytes: u64,
    pub prefix_bytes: u64,
    matched_end: TextSize,
    result: Option<Option<TextSize>>,
}
impl<'l> LiteralScan<'l> {
    pub fn new(literal: &'l str, start: TextSize, final_context: Option<TextSize>) -> Option<Self> {
        if literal.is_empty() {
            return None;
        }
        let literal_end = TextSize::checked_from_usize(literal.len()).ok()?;
        Some(Self {
            literal,
            literal_end,
            final_context,
            expected: GraphemeScan::new(TextSize::ZERO, Some(literal_end)),
            actual: GraphemeScan::new(start, final_context),
            expected_range: None,
            actual_range: None,
            compared: 0,
            prefix_checked: false,
            comparison_bytes: 0,
            prefix_bytes: 0,
            matched_end: start,
            result: None,
        })
    }
    fn complete(&mut self, result: Option<TextSize>) -> LiteralProgress {
        self.result = Some(result);
        LiteralProgress::Complete(result)
    }
    pub fn advance<S: ScanSource + ?Sized>(
        &mut self,
        source: &S,
        consume_end: TextSize,
        context_end: TextSize,
        final_input: bool,
        allowance: &mut u64,
    ) -> LiteralProgress {
        if consume_end > context_end
            || context_end.to_usize() > source.len_bytes()
            || !source.boundary(consume_end.to_usize())
            || !source.boundary(context_end.to_usize())
            || !source.boundary(self.matched_end.to_usize())
            || self.final_context.is_some_and(|end| end != context_end)
        {
            return LiteralProgress::InvalidSource;
        }
        if final_input {
            self.final_context = Some(context_end);
        }
        if let Some(result) = self.result {
            return LiteralProgress::Complete(result);
        }
        loop {
            if self.expected_range.is_none() {
                match self
                    .expected
                    .advance(self.literal, self.literal_end, true, allowance)
                {
                    ScanProgress::Grapheme(range) => self.expected_range = Some(range),
                    ScanProgress::End => return self.complete(Some(self.matched_end)),
                    ScanProgress::NeedsProcessing => return LiteralProgress::NeedsProcessing,
                    ScanProgress::NeedInput | ScanProgress::InvalidSource => {
                        return LiteralProgress::InvalidSource;
                    }
                }
            }
            if !self.prefix_checked {
                if *allowance == 0 {
                    return LiteralProgress::NeedsProcessing;
                }
                *allowance -= 1;
                self.prefix_bytes += 1;
                let expected = self.expected_range.expect("expected grapheme");
                if let Some(byte) = source.scan_byte_at(self.matched_end.to_usize()) {
                    if !byte.is_ascii()
                        && self.matched_end < consume_end
                        && byte != self.literal.as_bytes()[expected.start.to_usize()]
                    {
                        return self.complete(None);
                    }
                }
                self.prefix_checked = true;
            }
            if self.actual_range.is_none() {
                match self
                    .actual
                    .advance(source, context_end, final_input, allowance)
                {
                    ScanProgress::Grapheme(range) if range.end <= consume_end => {
                        self.actual_range = Some(range)
                    }
                    ScanProgress::Grapheme(_) | ScanProgress::End => return self.complete(None),
                    ScanProgress::NeedInput => return LiteralProgress::NeedInput,
                    ScanProgress::NeedsProcessing => return LiteralProgress::NeedsProcessing,
                    ScanProgress::InvalidSource => return LiteralProgress::InvalidSource,
                }
            }
            let expected = self.expected_range.expect("expected grapheme");
            let actual = self.actual_range.expect("source grapheme");
            if expected.len() != actual.len() {
                return self.complete(None);
            }
            while self.compared < actual.len().0 {
                if *allowance == 0 {
                    return LiteralProgress::NeedsProcessing;
                }
                *allowance -= 1;
                self.comparison_bytes += 1;
                let byte = source.scan_byte_at((actual.start + TextSize(self.compared)).to_usize());
                let expected =
                    self.literal.as_bytes()[expected.start.to_usize() + self.compared as usize];
                if byte != Some(expected) {
                    return self.complete(None);
                }
                self.compared += 1;
            }
            self.matched_end = actual.end;
            self.actual_range = None;
            self.expected_range = None;
            self.compared = 0;
            self.prefix_checked = false;
        }
    }
}

// This cache belongs to one ParserState. Source appends preserve its prefix;
// bounds participate in every key. An edit discards the owning state, just as
// it discards retained Unicode cursors. Only sealed, complete scans are cached.
const CACHE_CAPACITY: usize = 4096;

#[derive(Clone, Copy)]
struct CacheEntry {
    literal: &'static str,
    start: TextSize,
    consume_end: TextSize,
    context_end: TextSize,
    result: Option<TextSize>,
    work: u64,
}
#[derive(Default)]
pub(crate) struct LiteralCache {
    slots: alloc::vec::Vec<Option<CacheEntry>>,
}
impl LiteralCache {
    // Reserve Fixed, LiteralResult, and Exit in addition to the exact scanner
    // work. A smaller allowance follows the original resumable phases instead.
    pub fn replay(
        &mut self,
        literal: &'static str,
        start: TextSize,
        consume_end: TextSize,
        context_end: TextSize,
        allowance: u64,
    ) -> Option<(Option<TextSize>, u64)> {
        let index = Self::slot(literal, start);
        let entry = self.slots.get(index)?.as_ref()?;
        if entry.literal != literal
            || entry.start != start
            || entry.consume_end != consume_end
            || entry.context_end != context_end
            || entry
                .work
                .checked_add(3)
                .is_none_or(|work| work > allowance)
        {
            return None;
        }
        let result = entry.result;
        let work = entry.work;
        Some((result, work))
    }
    fn slot(literal: &str, start: TextSize) -> usize {
        let mut hash = 0x811c9dc5_u32;
        for byte in literal.bytes() {
            hash = (hash ^ u32::from(byte)).wrapping_mul(0x01000193);
        }
        let hash = hash ^ (start.0.wrapping_mul(0x9e3779b9));
        ((hash ^ (hash >> 16)) as usize) & (CACHE_CAPACITY - 1)
    }
}
impl LiteralScan<'static> {
    pub fn advance_cached<S: ScanSource + ?Sized>(
        &mut self,
        source: &S,
        consume_end: TextSize,
        context_end: TextSize,
        final_input: bool,
        allowance: &mut u64,
        cache: &mut LiteralCache,
    ) -> LiteralProgress {
        // Partially advanced scanners keep their retained Unicode state. Replay
        // is allowed only when the original complete scan fits in this poll.
        let initial = self.result.is_none()
            && self.expected.work.calls == 0
            && self.actual.work.calls == 0
            && self.prefix_bytes == 0
            && self.comparison_bytes == 0;
        if !initial || !final_input || self.final_context != Some(context_end) {
            return self.advance(source, consume_end, context_end, final_input, allowance);
        }
        let start = self.matched_end;
        let index = LiteralCache::slot(self.literal, start);
        if let Some(Some(entry)) = cache.slots.get(index)
            && entry.literal == self.literal
            && entry.start == start
            && entry.consume_end == consume_end
            && entry.context_end == context_end
            && entry.work <= *allowance
        {
            let work = entry.work;
            let result = entry.result;
            *allowance -= work;
            return self.complete(result);
        }
        let before = *allowance;
        let result = self.advance(source, consume_end, context_end, final_input, allowance);
        if let LiteralProgress::Complete(result) = result {
            if cache.slots.is_empty() {
                cache.slots.resize(CACHE_CAPACITY, None);
            }
            cache.slots[index] = Some(CacheEntry {
                literal: self.literal,
                start,
                consume_end,
                context_end,
                result,
                work: before - *allowance,
            });
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{DocumentId, Revision, TextSnapshot};
    use alloc::{string::String, vec::Vec};
    use unicode_segmentation::UnicodeSegmentation;

    fn work(scan: &LiteralScan<'_>) -> u64 {
        scan.expected.work.calls
            + scan.actual.work.calls
            + scan.comparison_bytes
            + scan.prefix_bytes
    }

    fn drain(
        scan: &mut LiteralScan<'_>,
        source: &TextSnapshot,
        final_input: bool,
    ) -> LiteralProgress {
        loop {
            let before = work(scan);
            let mut allowance = 1;
            let progress = scan.advance(
                source,
                source.byte_len(),
                source.byte_len(),
                final_input,
                &mut allowance,
            );
            assert!(
                work(scan) - before <= 1,
                "a single poll exceeded its allowance"
            );
            if progress != LiteralProgress::NeedsProcessing {
                return progress;
            }
            assert_eq!(allowance, 0);
        }
    }

    // Independent, contiguous Unicode iterator reference. It does not use the
    // retained scanner or canonical cursor under test.
    fn expected(literal: &str, source: &str) -> Option<TextSize> {
        let mut actual = source.grapheme_indices(true);
        let mut end = 0;
        for wanted in literal.graphemes(true) {
            let (at, found) = actual.next()?;
            if wanted != found {
                return None;
            }
            end = at + found.len();
        }
        Some(TextSize(end as u32))
    }

    #[test]
    fn cached_scans_preserve_unicode_results_and_every_allowance_boundary() {
        for (literal, text) in [
            (":", ":="),
            (":=", ":"),
            ("true", "true!"),
            ("false", "true!"),
            ("x", ""),
            ("e", "e\u{301}x"),
            ("e\u{301}", "e\u{301}x"),
            ("\r", "\r\nx"),
            ("\r\n", "\r\nx"),
            ("👩\u{200d}💻", "👩\u{200d}💻!"),
            ("🇦🇧", "🇦🇧🇨"),
        ] {
            let source = TextSnapshot::new(DocumentId(826), Revision(0), text).unwrap();
            let end = source.byte_len();
            let mut cache = LiteralCache::default();
            let mut warm = LiteralScan::new(literal, TextSize::ZERO, Some(end)).unwrap();
            let mut allowance = u64::MAX;
            assert_eq!(
                warm.advance_cached(&source, end, end, true, &mut allowance, &mut cache),
                LiteralProgress::Complete(expected(literal, text))
            );
            let scan_work = u64::MAX - allowance;
            assert_eq!(
                cache.replay(literal, TextSize::ZERO, end, end, scan_work + 3),
                Some((expected(literal, text), scan_work))
            );
            assert!(
                cache
                    .replay(literal, TextSize::ZERO, end, end, scan_work + 2)
                    .is_none()
            );
            for budget in 1..=scan_work + 2 {
                let mut reference = LiteralScan::new(literal, TextSize::ZERO, Some(end)).unwrap();
                let mut cached = LiteralScan::new(literal, TextSize::ZERO, Some(end)).unwrap();
                loop {
                    let mut reference_left = budget;
                    let mut cached_left = budget;
                    let expected = reference.advance(&source, end, end, true, &mut reference_left);
                    let actual = cached.advance_cached(
                        &source,
                        end,
                        end,
                        true,
                        &mut cached_left,
                        &mut cache,
                    );
                    assert_eq!((actual, cached_left), (expected, reference_left));
                    if actual != LiteralProgress::NeedsProcessing {
                        break;
                    }
                }
            }
        }
    }

    #[test]
    fn cache_bounds_and_unsealed_frontiers_cannot_reuse_a_different_scan() {
        let source = TextSnapshot::new(DocumentId(826), Revision(0), "\r\nx").unwrap();
        let end = source.byte_len();
        let mut cache = LiteralCache::default();
        for (consume, context, expected) in [(1, 1, Some(TextSize(1))), (1, 3, None), (2, 3, None)]
        {
            let mut scan = LiteralScan::new("\r", TextSize::ZERO, Some(TextSize(context))).unwrap();
            let mut allowance = u64::MAX;
            assert_eq!(
                scan.advance_cached(
                    &source,
                    TextSize(consume),
                    TextSize(context),
                    true,
                    &mut allowance,
                    &mut cache,
                ),
                LiteralProgress::Complete(expected)
            );
        }
        let mut unsealed = LiteralScan::new("\r", TextSize::ZERO, None).unwrap();
        let mut allowance = u64::MAX;
        assert_eq!(
            unsealed.advance_cached(
                &source,
                TextSize(1),
                TextSize(1),
                false,
                &mut allowance,
                &mut cache,
            ),
            LiteralProgress::NeedInput
        );
        assert_eq!(
            unsealed.advance_cached(&source, end, end, true, &mut allowance, &mut cache),
            LiteralProgress::Complete(None)
        );
    }

    #[test]
    fn literal_cache_storage_is_bounded_as_source_offsets_grow() {
        let source = "x".repeat(CACHE_CAPACITY * 4);
        let end = TextSize(source.len() as u32);
        let mut cache = LiteralCache::default();
        for offset in 0..end.0 {
            let mut scan = LiteralScan::new("x", TextSize(offset), Some(end)).unwrap();
            let mut allowance = u64::MAX;
            assert_eq!(
                scan.advance_cached(source.as_str(), end, end, true, &mut allowance, &mut cache,),
                LiteralProgress::Complete(Some(TextSize(offset + 1)))
            );
            assert_eq!(cache.slots.len(), CACHE_CAPACITY);
        }
    }

    #[test]
    fn every_literal_partition_matches_independent_unicode_reference() {
        let cases = [
            (":", ":="),
            (":=", ":="),
            (":=", ":"),
            (".", "..="),
            ("..", "..="),
            ("..=", "..="),
            ("-", "->"),
            ("->", "->"),
            ("~", "~>"),
            ("~>", "~>"),
            ("abc", "abcd"),
            ("abc", "abX"),
            ("x", ""),
            ("e", "e\u{301}"),
            ("e\u{301}", "e\u{301}x"),
            ("\r\n", "\r\nx"),
            ("\r", "\r\n"),
            ("👩\u{200d}💻", "👩\u{200d}💻!"),
            ("🇦🇧", "🇦🇧🇨"),
            ("🇦", "🇦🇧"),
            ("क्\u{200d}क", "क्\u{200d}क!"),
        ];
        for (literal, text) in cases {
            let boundaries: Vec<_> = text
                .char_indices()
                .map(|(at, _)| at)
                .chain(core::iter::once(text.len()))
                .collect();
            // Every two-part cut, then an append per scalar.
            let partitions = boundaries
                .iter()
                .map(|&split| alloc::vec![&text[..split], &text[split..]])
                .chain(core::iter::once(
                    boundaries
                        .windows(2)
                        .map(|pair| &text[pair[0]..pair[1]])
                        .collect(),
                ));
            for chunks in partitions {
                let mut source = TextSnapshot::new(DocumentId(826), Revision(0), "").unwrap();
                let mut scan = LiteralScan::new(literal, TextSize::ZERO, None).unwrap();
                for chunk in chunks {
                    source = source.append(chunk).unwrap();
                    match drain(&mut scan, &source, false) {
                        LiteralProgress::NeedInput => {}
                        LiteralProgress::Complete(result) => assert_eq!(
                            result,
                            expected(literal, text),
                            "early decision for {literal:?} in {text:?}"
                        ),
                        result => panic!("unexpected progress {result:?}"),
                    }
                }
                assert_eq!(
                    drain(&mut scan, &source, true),
                    LiteralProgress::Complete(expected(literal, text)),
                    "{literal:?} in {text:?}"
                );
                let before = work(&scan);
                assert_eq!(
                    drain(&mut scan, &source, true),
                    LiteralProgress::Complete(expected(literal, text))
                );
                assert_eq!(work(&scan), before);
                let grown = source.append("!").unwrap();
                assert_eq!(
                    drain(&mut scan, &grown, true),
                    LiteralProgress::InvalidSource
                );
            }
        }
    }

    #[test]
    fn temporary_eof_does_not_finish_literal_or_grapheme() {
        let mut source = TextSnapshot::new(DocumentId(826), Revision(0), "e").unwrap();
        let mut scan = LiteralScan::new("e\u{301}", TextSize::ZERO, None).unwrap();
        assert_eq!(drain(&mut scan, &source, false), LiteralProgress::NeedInput);
        source = source.append("\u{301}").unwrap();
        assert_eq!(drain(&mut scan, &source, false), LiteralProgress::NeedInput);
        source = source.append("!").unwrap();
        assert_eq!(
            drain(&mut scan, &source, false),
            LiteralProgress::Complete(Some(TextSize(3)))
        );
    }

    #[test]
    fn zero_allowance_and_invalid_bounds_preserve_work() {
        let source = TextSnapshot::new(DocumentId(826), Revision(0), "é!").unwrap();
        let mut scan = LiteralScan::new("é", TextSize::ZERO, None).unwrap();
        assert_eq!(
            scan.advance(&source, source.byte_len(), source.byte_len(), false, &mut 0),
            LiteralProgress::NeedsProcessing
        );
        assert_eq!(work(&scan), 0);
        assert_eq!(
            scan.advance(&source, TextSize(1), source.byte_len(), false, &mut 1),
            LiteralProgress::InvalidSource
        );
        assert_eq!(work(&scan), 0);
        assert_eq!(
            drain(&mut scan, &source, true),
            LiteralProgress::Complete(Some(TextSize(2)))
        );
        let mut invalid = LiteralScan::new("é", TextSize(1), None).unwrap();
        assert_eq!(
            drain(&mut invalid, &source, true),
            LiteralProgress::InvalidSource
        );
        assert!(LiteralScan::new("", TextSize::ZERO, None).is_none());
    }

    #[test]
    fn long_expected_and_actual_graphemes_resume_with_linear_scanner_work() {
        let mut previous = None;
        for n in [128, 256, 512, 1024] {
            let literal = String::from("e") + &"\u{301}".repeat(n);
            let mut source = TextSnapshot::new(DocumentId(826), Revision(0), "").unwrap();
            let mut scan = LiteralScan::new(&literal, TextSize::ZERO, None).unwrap();
            for scalar in literal.chars() {
                source = source.append(scalar.encode_utf8(&mut [0; 4])).unwrap();
                assert_eq!(drain(&mut scan, &source, false), LiteralProgress::NeedInput);
            }
            assert_eq!(
                scan.comparison_bytes, 0,
                "unfinished grapheme must remain pending"
            );
            assert_eq!(
                drain(&mut scan, &source, true),
                LiteralProgress::Complete(Some(source.byte_len()))
            );
            assert_eq!(scan.comparison_bytes, literal.len() as u64);
            let current = work(&scan);
            assert!(current <= 10 * literal.len() as u64);
            if let Some(old) = previous {
                assert!(current <= old * 3, "growing literal was restarted");
            }
            previous = Some(current);
        }
    }
}
