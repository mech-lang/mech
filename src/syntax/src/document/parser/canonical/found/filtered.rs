//! The grammar's ignored scalars are removed before the shared classifier.
//! Both filtering and classification retain their progress across source appends.
use super::*;
use crate::document::TextSnapshot;
use crate::document::parser::grapheme_scan::ScanSource;

pub(crate) struct FilteredContinuation {
    at: TextSize,
    logical: String,
    child: SourceContinuation,
    feed: bool,
    pub work: u64,
}
impl FilteredContinuation {
    pub fn new(at: TextSize) -> Self {
        Self {
            at,
            logical: String::new(),
            child: SourceContinuation::new(TextSize::ZERO),
            feed: true,
            work: 0,
        }
    }
    pub fn advance(
        &mut self,
        source: &TextSnapshot,
        end: TextSize,
        final_input: bool,
        allowance: &mut u64,
    ) -> SourceProgress {
        loop {
            if *allowance == 0 {
                return SourceProgress::NeedsProcessing;
            }
            if self.feed {
                if self.at >= end {
                    if !final_input {
                        return SourceProgress::NeedInput;
                    }
                    self.feed = false;
                } else {
                    *allowance -= 1;
                    self.work += 1;
                    let scalar = source
                        .scalar_at(self.at.to_usize())
                        .expect("filtered source scalar");
                    self.at += TextSize(scalar.len() as u32);
                    if is_grammar_ignored(scalar.chars().next().expect("scalar")) {
                        continue;
                    }
                    self.logical.push_str(scalar);
                    self.feed = false;
                }
            }
            let before = *allowance;
            let progress = self.child.advance(
                self.logical.as_str(),
                TextSize(self.logical.len() as u32),
                final_input && self.at >= end,
                allowance,
            );
            self.work += before - *allowance;
            match progress {
                SourceProgress::NeedInput => self.feed = true,
                result => return result,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{DocumentId, Revision};
    use alloc::vec::Vec;

    fn run(text: &str, chunks: &[&str]) -> FoundSyntax {
        let mut source = TextSnapshot::new(DocumentId(826), Revision(0), "").unwrap();
        let mut child = FilteredContinuation::new(TextSize::ZERO);
        for chunk in chunks {
            source = source.append(*chunk).unwrap();
            loop {
                let mut allowance = 1;
                let before = child.work;
                let progress = child.advance(&source, source.byte_len(), false, &mut allowance);
                assert!(child.work - before <= 1);
                match progress {
                    SourceProgress::Complete(found) => return found,
                    SourceProgress::NeedInput => break,
                    SourceProgress::NeedsProcessing => {}
                }
            }
        }
        assert_eq!(source.byte_len().to_usize(), text.len());
        loop {
            let mut allowance = 1;
            match child.advance(&source, source.byte_len(), true, &mut allowance) {
                SourceProgress::Complete(found) => return found,
                SourceProgress::NeedsProcessing => {}
                SourceProgress::NeedInput => panic!("final filtered source"),
            }
        }
    }

    #[test]
    fn filtered_found_uses_shared_classification_across_every_scalar_cut() {
        for text in [
            "",
            " \t\n",
            " : = x",
            " a \u{301} b",
            "👩 \u{200d} 💻!",
            "- > x",
            "1 2",
            "🇺 🇸!",
        ] {
            let logical: String = text.chars().filter(|c| !is_grammar_ignored(*c)).collect();
            let mut source = SourceContinuation::new(TextSize::ZERO);
            let mut allowance = u64::MAX;
            let expected = match source.advance(
                logical.as_str(),
                TextSize(logical.len() as u32),
                true,
                &mut allowance,
            ) {
                SourceProgress::Complete(found) => found,
                _ => panic!("finite shared classifier"),
            };
            let boundaries: Vec<_> = text
                .char_indices()
                .map(|(at, _)| at)
                .chain(core::iter::once(text.len()))
                .collect();
            for at in boundaries.iter().copied() {
                assert_eq!(
                    run(text, &[&text[..at], &text[at..]]),
                    expected,
                    "{text:?} split {at}"
                );
            }
            let chunks: Vec<_> = boundaries
                .windows(2)
                .map(|pair| &text[pair[0]..pair[1]])
                .collect();
            assert_eq!(run(text, &chunks), expected);
        }
    }
}
