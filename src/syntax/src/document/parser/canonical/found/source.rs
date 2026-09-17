//! Resumable physical found-syntax classification under the shared terminal table.
use super::*;
use crate::document::parser::grapheme_scan::{GraphemeScan, ScanProgress, ScanSource};
use crate::document::parser::literal_scan::{LiteralProgress, LiteralScan};

pub(crate) enum SourceProgress {
    Complete(FoundSyntax),
    NeedInput,
    NeedsProcessing,
}
enum Phase {
    Start,
    Terminal(usize),
    Literal(usize, LiteralScan<'static>),
    Grapheme(GraphemeScan),
    Export {
        kind: SyntaxKind,
        at: TextSize,
        end: TextSize,
        text: String,
    },
    Done,
}
pub(crate) struct SourceContinuation {
    start: TextSize,
    longest: Option<&'static FixedTerminalSpec>,
    phase: Phase,
    pub work: u64,
}
impl SourceContinuation {
    pub fn new(start: TextSize) -> Self {
        Self {
            start,
            longest: None,
            phase: Phase::Start,
            work: 0,
        }
    }
    pub fn advance<S: ScanSource + ?Sized>(
        &mut self,
        source: &S,
        context_end: TextSize,
        final_input: bool,
        allowance: &mut u64,
    ) -> SourceProgress {
        loop {
            if *allowance == 0 {
                return SourceProgress::NeedsProcessing;
            }
            let phase = core::mem::replace(&mut self.phase, Phase::Done);
            if !matches!(phase, Phase::Literal(..) | Phase::Grapheme(_)) {
                *allowance -= 1;
                self.work += 1;
            }
            match phase {
                Phase::Start => {
                    if self.start >= context_end {
                        if final_input {
                            return SourceProgress::Complete(eof());
                        }
                        self.phase = Phase::Start;
                        return SourceProgress::NeedInput;
                    }
                    self.phase = Phase::Terminal(0);
                }
                Phase::Terminal(index) => {
                    if let Some(spec) = FIXED_TERMINALS.get(index) {
                        self.phase = Phase::Literal(
                            index,
                            LiteralScan::new(
                                spec.literal,
                                self.start,
                                final_input.then_some(context_end),
                            )
                            .expect("canonical fixed terminal"),
                        );
                    } else if let Some(spec) = self.longest {
                        return SourceProgress::Complete(FoundSyntax {
                            kind: Some(spec.kind),
                            text: Some(spec.literal.to_string()),
                        });
                    } else {
                        self.phase = Phase::Grapheme(GraphemeScan::new(
                            self.start,
                            final_input.then_some(context_end),
                        ));
                    }
                }
                Phase::Literal(index, mut scan) => {
                    let before = *allowance;
                    let progress =
                        scan.advance(source, context_end, context_end, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        LiteralProgress::Complete(end) => {
                            let spec = &FIXED_TERMINALS[index];
                            if end.is_some()
                                && self.longest.map_or(true, |previous| {
                                    spec.literal.len() > previous.literal.len()
                                })
                            {
                                self.longest = Some(spec);
                            }
                            self.phase = Phase::Terminal(index + 1);
                        }
                        LiteralProgress::NeedInput => {
                            self.phase = Phase::Literal(index, scan);
                            return SourceProgress::NeedInput;
                        }
                        LiteralProgress::NeedsProcessing => {
                            self.phase = Phase::Literal(index, scan);
                            return SourceProgress::NeedsProcessing;
                        }
                        LiteralProgress::InvalidSource => {
                            panic!("found-syntax source bounds changed")
                        }
                    }
                }
                Phase::Grapheme(mut scan) => {
                    let before = *allowance;
                    let progress = scan.advance(source, context_end, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        ScanProgress::Grapheme(range) => {
                            let first = source
                                .scalar_at(range.start.to_usize())
                                .and_then(|scalar| scalar.chars().next())
                                .expect("found grapheme scalar");
                            let kind = if first.is_alphabetic() {
                                SyntaxKind::Alpha
                            } else if first.is_numeric() {
                                SyntaxKind::Digit
                            } else if is_canonical_emoji(first) {
                                SyntaxKind::Emoji
                            } else {
                                SyntaxKind::Any
                            };
                            self.phase = Phase::Export {
                                kind,
                                at: range.start,
                                end: range.end,
                                text: String::new(),
                            };
                        }
                        ScanProgress::End => return SourceProgress::Complete(eof()),
                        ScanProgress::NeedInput => {
                            self.phase = Phase::Grapheme(scan);
                            return SourceProgress::NeedInput;
                        }
                        ScanProgress::NeedsProcessing => {
                            self.phase = Phase::Grapheme(scan);
                            return SourceProgress::NeedsProcessing;
                        }
                        ScanProgress::InvalidSource => {
                            panic!("found-syntax grapheme source bounds changed")
                        }
                    }
                }
                Phase::Export {
                    kind,
                    at,
                    end,
                    mut text,
                } => {
                    if at < end {
                        let scalar = source
                            .scalar_at(at.to_usize())
                            .expect("found source scalar");
                        let next = TextSize(at.0 + scalar.len() as u32);
                        assert!(next <= end);
                        text.push_str(scalar);
                        self.phase = Phase::Export {
                            kind,
                            at: next,
                            end,
                            text,
                        };
                    } else {
                        return SourceProgress::Complete(FoundSyntax {
                            kind: Some(kind),
                            text: Some(text),
                        });
                    }
                }
                Phase::Done => panic!("found-syntax result was already consumed"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{DocumentId, Revision, TextSnapshot};
    use alloc::vec::Vec;

    fn run(text: &str, chunks: &[&str], step: bool) -> (FoundSyntax, u64, bool) {
        let mut source = TextSnapshot::new(DocumentId(826), Revision(0), "").unwrap();
        let mut continuation = SourceContinuation::new(TextSize::ZERO);
        let mut found = None;
        for chunk in chunks {
            source = source.append(*chunk).unwrap();
            if found.is_some() {
                continue;
            }
            loop {
                let mut allowance = if step { 1 } else { u64::MAX };
                let before = continuation.work;
                let progress =
                    continuation.advance(&source, source.byte_len(), false, &mut allowance);
                if step {
                    assert!(continuation.work - before <= 1);
                }
                match progress {
                    SourceProgress::Complete(result) => {
                        found = Some(result);
                        break;
                    }
                    SourceProgress::NeedInput => break,
                    SourceProgress::NeedsProcessing => {}
                }
            }
        }
        assert_eq!(source.byte_len().to_usize(), text.len());
        let completed_open = found.is_some();
        while found.is_none() {
            let mut allowance = if step { 1 } else { u64::MAX };
            let before = continuation.work;
            match continuation.advance(&source, source.byte_len(), true, &mut allowance) {
                SourceProgress::Complete(result) => found = Some(result),
                SourceProgress::NeedsProcessing => {}
                SourceProgress::NeedInput => panic!("sealed found syntax"),
            }
            if step {
                assert!(continuation.work - before <= 1);
            }
        }
        (found.unwrap(), continuation.work, completed_open)
    }
    #[test]
    fn physical_found_classification_preserves_all_terminal_and_unicode_cuts() {
        let mut cases: Vec<&str> = FIXED_TERMINALS.iter().map(|spec| spec.literal).collect();
        cases.extend([
            "",
            ":=",
            ": =",
            "! ",
            "é\u{301}x",
            "👩\u{200d}💻x",
            "🇺🇸🇨🇦",
            "\r\nx",
            "\u{600}=x",
            ":\u{301}",
            "1\u{301}",
            "\u{2009}x",
        ]);
        for text in cases {
            let (expected, _, _) = run(text, &[text], false);
            let boundaries: Vec<_> = text
                .char_indices()
                .map(|(at, _)| at)
                .chain(core::iter::once(text.len()))
                .collect();
            for split in &boundaries {
                let (found, _, _) = run(text, &[&text[..*split], &text[*split..]], true);
                assert_eq!(found, expected, "{text:?}, split {split}");
            }
            let chunks: Vec<_> = boundaries
                .windows(2)
                .map(|pair| &text[pair[0]..pair[1]])
                .collect();
            let (found, _, _) = run(text, &chunks, true);
            assert_eq!(found, expected, "{text:?}, scalar chunks");
        }
        for (text, expected_text, kind) in [
            ("é\u{301}x", "é\u{301}", SyntaxKind::Alpha),
            ("👩\u{200d}💻x", "👩\u{200d}💻", SyntaxKind::Emoji),
            (":\u{301}x", ":\u{301}", SyntaxKind::Any),
        ] {
            let (found, _, completed_open) = run(text, &[text], true);
            assert!(
                completed_open,
                "complete physical context should not require EOF"
            );
            assert_eq!(found.kind, Some(kind));
            assert_eq!(found.text.as_deref(), Some(expected_text));
        }
    }
    #[test]
    fn long_grapheme_classification_and_export_retain_linear_work() {
        for (prefix, unit) in [("a", "\u{301}"), (":", "\u{301}"), ("👩", "\u{200d}💻")] {
            let mut previous = None;
            for n in [64, 128, 256, 512] {
                let text = String::from(prefix) + &unit.repeat(n);
                let boundaries: Vec<_> = text
                    .char_indices()
                    .map(|(at, _)| at)
                    .chain(core::iter::once(text.len()))
                    .collect();
                let chunks: Vec<_> = boundaries
                    .windows(2)
                    .map(|pair| &text[pair[0]..pair[1]])
                    .collect();
                let (found, work, _) = run(&text, &chunks, true);
                let (expected, one_shot, _) = run(&text, &[&text], false);
                assert_eq!(found, expected);
                assert_eq!(found.text.as_deref(), Some(text.as_str()));
                if let Some((prior_work, prior_one_shot)) = previous {
                    assert!(work <= prior_work * 3);
                    assert!(one_shot <= prior_one_shot * 3);
                }
                previous = Some((work, one_shot));
            }
        }
    }
}
