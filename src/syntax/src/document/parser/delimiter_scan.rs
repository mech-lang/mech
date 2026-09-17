//! Delimiter search retains probes and grapheme traversal across input frontiers.
use super::grapheme_scan::{GraphemeScan, ScanProgress};
use super::literal_scan::{LiteralProgress, LiteralScan};
use crate::document::{TextRange, TextSize, TextSnapshot};

pub(crate) enum DelimiterProgress {
    Grapheme(TextRange),
    Found(TextSize),
    End(TextSize),
    NeedInput,
    NeedsProcessing,
    InvalidSource,
}
enum Phase {
    Probe(LiteralScan<'static>),
    Grapheme(GraphemeScan),
    Found,
    End,
}
pub(crate) struct DelimiterScan {
    delimiter: &'static str,
    offset: TextSize,
    phase: Phase,
    final_end: Option<TextSize>,
    pub work: u64,
}
impl DelimiterScan {
    pub fn new(
        delimiter: &'static str,
        start: TextSize,
        final_end: Option<TextSize>,
    ) -> Option<Self> {
        Some(Self {
            delimiter,
            offset: start,
            phase: Phase::Probe(LiteralScan::new(delimiter, start, final_end)?),
            final_end,
            work: 0,
        })
    }
    pub fn advance(
        &mut self,
        source: &TextSnapshot,
        end: TextSize,
        final_input: bool,
        allowance: &mut u64,
    ) -> DelimiterProgress {
        if end > source.byte_len()
            || self.offset > end
            || !source.is_char_boundary(end)
            || self.final_end.is_some_and(|prior| prior != end)
        {
            return DelimiterProgress::InvalidSource;
        }
        if final_input {
            self.final_end = Some(end);
        }
        loop {
            match &mut self.phase {
                Phase::Found => return DelimiterProgress::Found(self.offset),
                Phase::End => return DelimiterProgress::End(self.offset),
                Phase::Probe(_) if self.offset == end => {
                    if self.final_end.is_none() {
                        return DelimiterProgress::NeedInput;
                    }
                    self.phase = Phase::End;
                }
                Phase::Probe(scan) => {
                    let before = *allowance;
                    let progress =
                        scan.advance(source, end, end, self.final_end.is_some(), allowance);
                    self.work += before - *allowance;
                    match progress {
                        LiteralProgress::Complete(Some(_)) => self.phase = Phase::Found,
                        LiteralProgress::Complete(None) => {
                            self.phase =
                                Phase::Grapheme(GraphemeScan::new(self.offset, self.final_end))
                        }
                        LiteralProgress::NeedInput => return DelimiterProgress::NeedInput,
                        LiteralProgress::NeedsProcessing => {
                            return DelimiterProgress::NeedsProcessing;
                        }
                        LiteralProgress::InvalidSource => return DelimiterProgress::InvalidSource,
                    }
                }
                Phase::Grapheme(scan) => {
                    let before = *allowance;
                    let progress = scan.advance(source, end, self.final_end.is_some(), allowance);
                    self.work += before - *allowance;
                    match progress {
                        ScanProgress::Grapheme(range) => {
                            self.offset = range.end;
                            self.phase = Phase::Probe(
                                LiteralScan::new(self.delimiter, self.offset, self.final_end)
                                    .expect("nonempty delimiter"),
                            );
                            return DelimiterProgress::Grapheme(range);
                        }
                        ScanProgress::End => self.phase = Phase::End,
                        ScanProgress::NeedInput => return DelimiterProgress::NeedInput,
                        ScanProgress::NeedsProcessing => return DelimiterProgress::NeedsProcessing,
                        ScanProgress::InvalidSource => return DelimiterProgress::InvalidSource,
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{DocumentId, Revision};
    use alloc::vec::Vec;
    use unicode_segmentation::UnicodeSegmentation;

    fn run(
        delimiter: &'static str,
        text: &str,
        chunks: &[&str],
    ) -> (Vec<TextRange>, Option<TextSize>, u64) {
        let mut source = TextSnapshot::new(DocumentId(826), Revision(0), "").unwrap();
        let mut scan = DelimiterScan::new(delimiter, TextSize::ZERO, None).unwrap();
        let mut ranges = Vec::new();
        for chunk in chunks {
            source = source.append(*chunk).unwrap();
            loop {
                let before = scan.work;
                let progress = scan.advance(&source, source.byte_len(), false, &mut 1);
                assert!(scan.work - before <= 1);
                match progress {
                    DelimiterProgress::Grapheme(range) => ranges.push(range),
                    DelimiterProgress::NeedsProcessing => {}
                    DelimiterProgress::NeedInput | DelimiterProgress::Found(_) => break,
                    _ => panic!("unexpected open delimiter progress"),
                }
            }
        }
        assert_eq!(chunks.concat(), text);
        let found = loop {
            let before = scan.work;
            let progress = scan.advance(&source, source.byte_len(), true, &mut 1);
            assert!(scan.work - before <= 1);
            match progress {
                DelimiterProgress::Grapheme(range) => ranges.push(range),
                DelimiterProgress::NeedsProcessing => {}
                DelimiterProgress::Found(at) => break Some(at),
                DelimiterProgress::End(at) => {
                    assert_eq!(at, source.byte_len());
                    break None;
                }
                _ => panic!("final delimiter input failed to drain"),
            }
        };
        let before = scan.work;
        assert!(matches!(
            scan.advance(&source, source.byte_len(), true, &mut 0),
            DelimiterProgress::Found(_) | DelimiterProgress::End(_)
        ));
        assert_eq!(scan.work, before);
        let grown = source.append("!").unwrap();
        assert!(matches!(
            scan.advance(&grown, grown.byte_len(), true, &mut 1),
            DelimiterProgress::InvalidSource
        ));
        (ranges, found, scan.work)
    }
    #[test]
    fn every_fence_delimiter_partition_matches_contiguous_unicode_search() {
        for delimiter in ["```", "~~~"] {
            for body in [
                "",
                "hello",
                "e\u{301}\r\n👩\u{200d}💻",
                "``",
                "``\u{301}`",
                "~~~\u{301}x",
                "🇦🇧🇨",
            ] {
                for closed in [false, true] {
                    let text = alloc::format!("{body}{}", if closed { delimiter } else { "" });
                    let graphemes: Vec<_> = text.grapheme_indices(true).collect();
                    let wanted: Vec<_> = delimiter.graphemes(true).collect();
                    let found = graphemes.windows(wanted.len()).position(|window| {
                        window
                            .iter()
                            .zip(&wanted)
                            .all(|((_, actual), wanted)| actual == wanted)
                    });
                    let expected: Vec<_> = graphemes
                        .iter()
                        .take(found.unwrap_or(graphemes.len()))
                        .map(|(at, text)| {
                            TextRange::new(TextSize(*at as u32), TextSize((at + text.len()) as u32))
                        })
                        .collect();
                    let end = found.map(|index| TextSize(graphemes[index].0 as u32));
                    for split in text
                        .char_indices()
                        .map(|(at, _)| at)
                        .chain(core::iter::once(text.len()))
                    {
                        let (ranges, observed, _) =
                            run(delimiter, &text, &[&text[..split], &text[split..]]);
                        assert_eq!(ranges, expected);
                        assert_eq!(observed, end);
                    }
                }
            }
        }
    }
    #[test]
    fn growing_unclosed_fences_and_graphemes_keep_linear_scan_work() {
        for combining in [false, true] {
            let mut previous = None;
            for n in [64, 128, 256, 512] {
                let text = alloc::format!(
                    "e{}",
                    if combining {
                        "\u{301}".repeat(n)
                    } else {
                        "a".repeat(n)
                    }
                );
                let offsets: Vec<_> = text
                    .char_indices()
                    .map(|(at, _)| at)
                    .chain(core::iter::once(text.len()))
                    .collect();
                let chunks: Vec<_> = offsets
                    .windows(2)
                    .map(|pair| &text[pair[0]..pair[1]])
                    .collect();
                let (ranges, found, work) = run("```", &text, &chunks);
                assert_eq!(found, None);
                assert_eq!(ranges.last().unwrap().end.to_usize(), text.len());
                if let Some(previous) = previous {
                    assert!(work <= previous * 3);
                }
                previous = Some(work);
            }
        }
    }
}
