//! Resumable Unicode scanning shared by canonical cursor reads and retained
//! input. This owns segmentation state, not grammar recognition or recovery.
use crate::document::{TextRange, TextSize, TextSnapshot};
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};

/// Scalar access only; Unicode recognition remains in the pinned cursor.
pub(crate) trait ScanSource {
    fn len_bytes(&self) -> usize;
    fn scan_byte_at(&self, offset: usize) -> Option<u8>;
    fn boundary(&self, offset: usize) -> bool;
    fn scalar_at(&self, offset: usize) -> Option<&str>;
    fn scalar_before(&self, offset: usize) -> Option<&str>;
    fn contiguous_without_lookup_work(&self) -> Option<&str> {
        None
    }
}
impl ScanSource for str {
    fn contiguous_without_lookup_work(&self) -> Option<&str> {
        Some(self)
    }
    fn scan_byte_at(&self, offset: usize) -> Option<u8> {
        self.as_bytes().get(offset).copied()
    }
    fn len_bytes(&self) -> usize {
        self.len()
    }
    fn boundary(&self, offset: usize) -> bool {
        self.is_char_boundary(offset)
    }
    fn scalar_at(&self, offset: usize) -> Option<&str> {
        let tail = self.get(offset..)?;
        tail.get(..tail.chars().next()?.len_utf8())
    }
    fn scalar_before(&self, offset: usize) -> Option<&str> {
        let prefix = self.get(..offset)?;
        prefix.get(prefix.len() - prefix.chars().next_back()?.len_utf8()..)
    }
}
impl ScanSource for TextSnapshot {
    fn contiguous_without_lookup_work(&self) -> Option<&str> {
        TextSnapshot::contiguous_without_lookup_work(self)
    }
    fn scan_byte_at(&self, offset: usize) -> Option<u8> {
        self.byte_at(TextSize::checked_from_usize(offset).ok()?)
    }
    fn len_bytes(&self) -> usize {
        self.byte_len().to_usize()
    }
    fn boundary(&self, offset: usize) -> bool {
        TextSize::checked_from_usize(offset)
            .ok()
            .is_some_and(|at| self.is_char_boundary(at))
    }
    fn scalar_at(&self, offset: usize) -> Option<&str> {
        let chunk = self.chunk_at(TextSize::checked_from_usize(offset).ok()?)?;
        let tail = chunk.text.get(offset - chunk.range.start.to_usize()..)?;
        tail.get(..tail.chars().next()?.len_utf8())
    }
    fn scalar_before(&self, offset: usize) -> Option<&str> {
        let chunk = self.chunk_before(TextSize::checked_from_usize(offset).ok()?)?;
        chunk
            .text
            .get(chunk.text.len() - chunk.text.chars().next_back()?.len_utf8()..)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ScanProgress {
    Grapheme(TextRange),
    NeedInput,
    NeedsProcessing,
    End,
    InvalidSource,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ScanWork {
    pub calls: u64,
    pub forward_bytes: u64,
    pub context_bytes: u64,
    pub lookbehind_bytes: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct GraphemeScan {
    cursor: GraphemeCursor,
    start: TextSize,
    context: Option<usize>,
    final_end: Option<TextSize>,
    pub work: ScanWork,
}

impl GraphemeScan {
    /// The owner must retain the exact source prefix while this continuation is
    /// live, and invalidate it on an edit. None means temporary, not final, EOF.
    pub fn new(start: TextSize, final_end: Option<TextSize>) -> Self {
        Self {
            cursor: GraphemeCursor::new(
                start.to_usize(),
                final_end.map_or(usize::MAX, TextSize::to_usize),
                true,
            ),
            start,
            context: None,
            final_end,
            work: ScanWork::default(),
        }
    }

    pub(crate) fn resumes_at(&self, start: TextSize, final_end: Option<TextSize>) -> bool {
        self.start == start && self.final_end.is_none_or(|end| Some(end) == final_end)
    }

    /// One allowance unit permits one Unicode-library call with at most one
    /// forward scalar and one preceding scalar (eight bytes). Context requests
    /// are suspended and charged too.
    pub fn advance<S: ScanSource + ?Sized>(
        &mut self,
        source: &S,
        end: TextSize,
        final_input: bool,
        allowance: &mut u64,
    ) -> ScanProgress {
        if let Some(text) = source.contiguous_without_lookup_work() {
            if final_input && let Some(progress) = self.advance_sealed_ascii(text, end, allowance) {
                return progress;
            }
            return self.advance_in(text, end, final_input, allowance);
        }
        self.advance_in(source, end, final_input, allowance)
    }

    fn advance_sealed_ascii(
        &mut self,
        source: &str,
        end: TextSize,
        allowance: &mut u64,
    ) -> Option<ScanProgress> {
        let start = self.start.to_usize();
        if self.final_end != Some(end)
            || self.context.is_some()
            || self.cursor.cur_cursor() != start
        {
            return None;
        }
        let text = source.get(..end.to_usize())?;
        let bytes = text.as_bytes();
        let first = *bytes.get(start)?;
        if !first.is_ascii() {
            return None;
        }
        // CRLF is the only pair of adjacent ASCII scalars in one grapheme.
        // An ASCII lookahead also gives the cursor a settled next origin.
        let width = if first == b'\r' && bytes.get(start + 1) == Some(&b'\n') {
            2
        } else {
            1
        };
        let boundary = start + width;
        let calls = match bytes.get(boundary) {
            None => width as u64,
            Some(byte) if byte.is_ascii() => (width + 1) as u64,
            Some(_) => return None,
        };
        if *allowance < calls {
            return None;
        }

        // The ordinary scanner charges a bounded Unicode call for each
        // forward scalar and for the lookahead at a non-final boundary.
        let preceding = text[..start].chars().next_back().map_or(0, char::len_utf8);
        *allowance -= calls;
        self.work.calls += calls;
        self.work.forward_bytes += calls;
        self.work.lookbehind_bytes += preceding as u64 + calls - 1;
        self.cursor.set_cursor(boundary);
        let next = TextSize(boundary as u32);
        let range = TextRange::new(self.start, next);
        self.start = next;
        Some(ScanProgress::Grapheme(range))
    }

    fn advance_in<S: ScanSource + ?Sized>(
        &mut self,
        source: &S,
        end: TextSize,
        final_input: bool,
        allowance: &mut u64,
    ) -> ScanProgress {
        if end.to_usize() > source.len_bytes()
            || !source.boundary(end.to_usize())
            || !source.boundary(self.cursor.cur_cursor())
            || self.start > end
            || self.cursor.cur_cursor() > end.to_usize()
            || self.final_end.is_some_and(|prior| prior != end)
        {
            return ScanProgress::InvalidSource;
        }
        if final_input {
            self.final_end = Some(end);
        }
        loop {
            if let Some(before) = self.context {
                if *allowance == 0 {
                    return ScanProgress::NeedsProcessing;
                }
                let Some(text) = source.scalar_before(before) else {
                    return ScanProgress::InvalidSource;
                };
                let width = text.len();
                self.cursor.provide_context(text, before - width);
                self.context = None;
                *allowance -= 1;
                self.work.calls += 1;
                self.work.context_bytes += width as u64;
                continue;
            }
            let at = self.cursor.cur_cursor();
            if at == end.to_usize() {
                if self.final_end.is_none() {
                    return ScanProgress::NeedInput;
                }
                if self.start == end {
                    return ScanProgress::End;
                }
                let range = TextRange::new(self.start, end);
                self.start = end;
                return ScanProgress::Grapheme(range);
            }
            if *allowance == 0 {
                return ScanProgress::NeedsProcessing;
            }
            let Some(forward) = source.scalar_at(at) else {
                return ScanProgress::InvalidSource;
            };
            let width = forward.len();
            if at + width > end.to_usize() {
                return ScanProgress::InvalidSource;
            }
            // Supply bounded overlap so the Unicode cursor can use its retained
            // regional-indicator/emoji state at a chunk edge. Starting every
            // library chunk exactly at its cursor would request fresh context
            // even when the forward scan already established that context.
            let mut bytes = [0_u8; 8];
            let mut prior_width = 0;
            if at > 0 {
                let Some(prior) = source.scalar_before(at) else {
                    return ScanProgress::InvalidSource;
                };
                prior_width = prior.len();
                bytes[..prior_width].copy_from_slice(prior.as_bytes());
            }
            bytes[prior_width..prior_width + width].copy_from_slice(forward.as_bytes());
            let text = core::str::from_utf8(&bytes[..prior_width + width])
                .expect("complete UTF-8 scalars");
            *allowance -= 1;
            self.work.calls += 1;
            self.work.forward_bytes += width as u64;
            self.work.lookbehind_bytes += prior_width as u64;
            match self.cursor.next_boundary(text, at - prior_width) {
                Ok(Some(boundary)) => {
                    let boundary = TextSize(boundary as u32);
                    let range = TextRange::new(self.start, boundary);
                    self.start = boundary;
                    return ScanProgress::Grapheme(range);
                }
                Ok(None) => return ScanProgress::End,
                Err(GraphemeIncomplete::NextChunk) => {}
                Err(GraphemeIncomplete::PreContext(before)) => self.context = Some(before),
                Err(GraphemeIncomplete::PrevChunk | GraphemeIncomplete::InvalidOffset) => {
                    return ScanProgress::InvalidSource;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{DocumentId, Revision};
    use unicode_segmentation::UnicodeSegmentation;

    #[test]
    fn sealed_ascii_preserves_poll_partitions_and_scanner_work() {
        for text in [
            "",
            "abc",
            "\r\n\r\t\0",
            "a\r\nb",
            "éa\r\nβ",
            "ae\u{301}b",
            "a👩\u{200d}💻b",
            "a🇦🇧🇨z",
        ] {
            for start in (0..=text.len()).filter(|&i| text.is_char_boundary(i)) {
                for end in (start..=text.len()).filter(|&i| text.is_char_boundary(i)) {
                    for budget in 1..=5 {
                        let start = TextSize(start as u32);
                        let end = TextSize(end as u32);
                        let mut reference = GraphemeScan::new(start, Some(end));
                        let mut shortcut = reference.clone();
                        for poll in 0.. {
                            let mut a = if poll == 0 { 0 } else { budget };
                            let mut b = a;
                            let expected = reference.advance_in(text, end, true, &mut a);
                            let observed = shortcut.advance(text, end, true, &mut b);
                            assert_eq!(
                                (observed, b, shortcut.work),
                                (expected, a, reference.work),
                                "{text:?} {start:?} {end:?} {budget}"
                            );
                            if observed == ScanProgress::End {
                                break;
                            }
                            assert!(!matches!(
                                observed,
                                ScanProgress::InvalidSource | ScanProgress::NeedInput
                            ));
                        }
                    }
                }
            }
        }
    }

    fn drain(
        scan: &mut GraphemeScan,
        source: &TextSnapshot,
        final_input: bool,
        ranges: &mut alloc::vec::Vec<TextRange>,
    ) -> ScanProgress {
        loop {
            let mut allowance = 1;
            let before = scan.work.calls;
            let result = scan.advance(source, source.byte_len(), final_input, &mut allowance);
            assert!(scan.work.calls - before <= 1);
            match result {
                ScanProgress::Grapheme(range) => ranges.push(range),
                ScanProgress::NeedsProcessing => assert_eq!(allowance, 0),
                _ => return result,
            }
        }
    }

    #[test]
    fn every_scalar_partition_matches_unicode_authority_without_committing_temporary_eof() {
        for text in [
            "abc",
            "a\r\nb\r",
            "e\u{301}\u{302}x",
            "👩\u{200d}💻!",
            "🇦🇧🇨🇩🇪",
            "क्\u{200d}क!",
        ] {
            let expected: alloc::vec::Vec<_> = text
                .grapheme_indices(true)
                .map(|(at, g)| TextRange::new(TextSize(at as u32), TextSize((at + g.len()) as u32)))
                .collect();
            for split in text
                .char_indices()
                .map(|(at, _)| at)
                .chain(core::iter::once(text.len()))
            {
                let mut source = TextSnapshot::new(DocumentId(826), Revision(0), "").unwrap();
                let mut scan = GraphemeScan::new(TextSize::ZERO, None);
                let mut ranges = alloc::vec::Vec::new();
                for chunk in [&text[..split], &text[split..]] {
                    source = source.append(chunk).unwrap();
                    assert_eq!(
                        drain(&mut scan, &source, false, &mut ranges),
                        ScanProgress::NeedInput
                    );
                }
                assert_eq!(
                    drain(&mut scan, &source, true, &mut ranges),
                    ScanProgress::End
                );
                assert_eq!(ranges, expected, "split {split}: {text:?}");
                let work = scan.work;
                assert_eq!(
                    drain(&mut scan, &source, true, &mut ranges),
                    ScanProgress::End
                );
                assert_eq!(scan.work, work);
            }
        }
    }

    #[test]
    fn zero_allowance_yields_without_losing_the_frontier() {
        let source = TextSnapshot::new(DocumentId(826), Revision(0), "e\u{301}x").unwrap();
        let mut scan = GraphemeScan::new(TextSize::ZERO, None);
        assert_eq!(
            scan.advance(&source, source.byte_len(), false, &mut 0),
            ScanProgress::NeedsProcessing
        );
        assert_eq!(scan.work, ScanWork::default());
        let mut ranges = alloc::vec::Vec::new();
        assert_eq!(
            drain(&mut scan, &source, true, &mut ranges),
            ScanProgress::End
        );
        assert_eq!(ranges.len(), 2);
        let extended = source.append("y").unwrap();
        assert_eq!(
            scan.advance(&extended, extended.byte_len(), false, &mut 1),
            ScanProgress::InvalidSource
        );
    }

    #[test]
    fn long_unfinished_grapheme_keeps_scanner_state_and_bounded_work() {
        let mut previous = None;
        for n in [128, 256, 512, 1024] {
            let mut source = TextSnapshot::new(DocumentId(826), Revision(0), "e").unwrap();
            let mut scan = GraphemeScan::new(TextSize::ZERO, None);
            let mut ranges = alloc::vec::Vec::new();
            assert_eq!(
                drain(&mut scan, &source, false, &mut ranges),
                ScanProgress::NeedInput
            );
            for _ in 0..n {
                source = source.append("\u{301}").unwrap();
                assert_eq!(
                    drain(&mut scan, &source, false, &mut ranges),
                    ScanProgress::NeedInput
                );
                assert!(ranges.is_empty());
            }
            assert_eq!(
                drain(&mut scan, &source, true, &mut ranges),
                ScanProgress::End
            );
            assert_eq!(ranges, [source.full_range()]);
            assert!(scan.work.calls <= 3 * (n + 1) as u64);
            if let Some(previous) = previous {
                assert!(scan.work.calls <= previous * 3);
            }
            previous = Some(scan.work.calls);
        }
    }
}
