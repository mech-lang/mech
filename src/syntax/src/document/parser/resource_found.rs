//! Resource envelopes are structural; potentially long diagnostic classification
//! is a separately retained child, drained under the document work allowance.
use super::*;
use canonical::found::{FilteredContinuation, SourceContinuation, SourceProgress};
enum Classifier {
    Source(SourceContinuation),
    Filtered(FilteredContinuation),
}
pub(super) struct Continuation {
    classifier: Classifier,
    diagnostic: usize,
    end: TextSize,
}
impl Continuation {
    pub fn new(mode: LexicalMode, diagnostic: usize, start: TextSize, end: TextSize) -> Self {
        Self {
            classifier: match mode {
                LexicalMode::CanonicalSourceFragment => {
                    Classifier::Source(SourceContinuation::new(start))
                }
                LexicalMode::CanonicalGrammar => {
                    Classifier::Filtered(FilteredContinuation::new(start))
                }
                LexicalMode::PrototypeDocument => {
                    unreachable!("prototype found syntax is constant")
                }
            },
            diagnostic,
            end,
        }
    }
}
impl Parser<'_> {
    pub(crate) fn advance_resource_found(&mut self, allowance: &mut u64) -> bool {
        let Some(mut child) = self.state.resource_found.take() else {
            return true;
        };
        let progress = match &mut child.classifier {
            Classifier::Source(scan) => scan.advance(self.source, child.end, true, allowance),
            Classifier::Filtered(scan) => scan.advance(self.source, child.end, true, allowance),
        };
        match progress {
            SourceProgress::Complete(found) => {
                self.state.diagnostics[child.diagnostic].diagnostic.found = Some(found);
                true
            }
            SourceProgress::NeedsProcessing => {
                self.state.resource_found = Some(child);
                false
            }
            SourceProgress::NeedInput => unreachable!("accepted resource remainder is final"),
        }
    }
}
