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
