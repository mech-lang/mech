//! Shared record/map lookahead retains delimiter, quote, and annotation state.
use super::*;
use crate::document::{NodeFlags, TokenFlags};
pub(super) struct Probe {
    checkpoint: ParserCheckpoint,
    node: Marker,
    delimiters: Vec<char>,
    quoted: bool,
    raw: bool,
    escaped: bool,
    key: bool,
}
pub(super) enum Phase {
    Enter,
    Separator(Probe),
    Scan(Probe),
    Triple(Probe, u8),
    Annotation(Probe),
    Consume(Probe),
}
impl Continuation {
    fn probe(&mut self, phase: Phase) {
        self.push(Frame::MappingProbe(Box::new(phase)));
    }
    fn probe_finish(&mut self, parser: &mut Parser<'_>, probe: Probe, matched: bool) {
        if parser.is_halted() {
            probe
                .node
                .complete_with_flags(parser, SyntaxKind::Error, NodeFlags::ERROR);
            self.result = Attempt::NoMatch;
        } else {
            parser.rewind(probe.checkpoint);
            self.result = if matched {
                Attempt::Matched
            } else {
                Attempt::NoMatch
            };
        }
    }
    fn probe_consume(&mut self, parser: &mut Parser<'_>) -> bool {
        if let Some((_, range)) = parser.bump_char_raw() {
            parser.token_with_flags(SyntaxKind::Unknown, range, TokenFlags::ERROR);
            !parser.is_halted()
        } else {
            false
        }
    }
    #[inline(never)]
    pub(super) fn mapping_probe_frame(
        &mut self,
        parser: &mut Parser<'_>,
        phase: Box<Phase>,
        final_input: bool,
    ) -> Option<Progress> {
        match *phase {
            Phase::Enter => {
                let probe = Probe {
                    checkpoint: parser.checkpoint(),
                    node: parser.start(),
                    delimiters: Vec::new(),
                    quoted: false,
                    raw: false,
                    escaped: false,
                    key: false,
                };
                self.probe(Phase::Separator(probe));
                self.base(rules::LIST_SEPARATOR);
            }
            Phase::Separator(probe) => {
                if self.result == Attempt::Matched {
                    self.probe(Phase::Separator(probe));
                    self.base(rules::LIST_SEPARATOR);
                } else {
                    self.probe(Phase::Scan(probe));
                }
            }
            Phase::Scan(mut probe) => {
                if parser.is_halted() {
                    self.probe_finish(parser, probe, false);
                    return None;
                }
                let Some(ch) = parser.cursor().peek_char() else {
                    if final_input {
                        self.probe_finish(parser, probe, false);
                    } else {
                        self.probe(Phase::Scan(probe));
                        return Some(Progress::NeedInput);
                    }
                    return None;
                };
                if !probe.quoted && ch == '"' {
                    let mut triple = true;
                    for i in 1..3 {
                        match parser.cursor().byte_at(i) {
                            Some(b'"') => {}
                            None if !final_input => {
                                self.probe(Phase::Scan(probe));
                                return Some(Progress::NeedInput);
                            }
                            _ => {
                                triple = false;
                                break;
                            }
                        }
                    }
                    if triple {
                        probe.raw = !probe.raw;
                        if probe.delimiters.is_empty() {
                            probe.key = true;
                        }
                        self.probe(Phase::Triple(probe, 3));
                        return None;
                    }
                }
                if probe.raw {
                    self.probe(Phase::Consume(probe));
                    return None;
                }
                if probe.quoted {
                    if probe.escaped {
                        probe.escaped = false;
                    } else if ch == '\\' {
                        probe.escaped = true;
                    } else if ch == '"' {
                        probe.quoted = false;
                    }
                    self.probe(Phase::Consume(probe));
                    return None;
                }
                match ch {
                    '"' => {
                        probe.quoted = true;
                        if probe.delimiters.is_empty() {
                            probe.key = true;
                        }
                    }
                    '<' | '⟨' => {
                        probe.key = true;
                        self.probe(Phase::Annotation(probe));
                        self.push(Frame::Annotation(false));
                        return None;
                    }
                    '(' | '[' | '{' => {
                        if probe.delimiters.is_empty() {
                            probe.key = true;
                        }
                        probe.delimiters.push(ch);
                    }
                    '>' | '⟩' if matches!(probe.delimiters.last(), Some('<' | '⟨')) => {
                        probe.delimiters.pop();
                    }
                    ')' | ']' | '}' => {
                        if probe.delimiters.is_empty() {
                            self.probe_finish(parser, probe, false);
                            return None;
                        }
                        probe.delimiters.pop();
                    }
                    ':' if probe.delimiters.is_empty() && probe.key => {
                        self.probe_finish(parser, probe, true);
                        return None;
                    }
                    ':' if probe.delimiters.is_empty() => probe.key = true,
                    ',' if probe.delimiters.is_empty() => {
                        self.probe_finish(parser, probe, false);
                        return None;
                    }
                    ch if probe.delimiters.is_empty() && !ch.is_whitespace() => probe.key = true,
                    _ => {}
                }
                self.probe(Phase::Consume(probe));
            }
            Phase::Triple(probe, remaining) => {
                if !self.probe_consume(parser) {
                    self.probe_finish(parser, probe, false);
                } else if remaining == 1 {
                    self.probe(Phase::Scan(probe));
                } else {
                    self.probe(Phase::Triple(probe, remaining - 1));
                }
            }
            Phase::Annotation(probe) => match self.result {
                Attempt::Matched => self.probe(Phase::Scan(probe)),
                Attempt::Committed => self.probe_finish(parser, probe, false),
                Attempt::NoMatch => self.probe(Phase::Consume(probe)),
            },
            Phase::Consume(probe) => {
                if self.probe_consume(parser) {
                    self.probe(Phase::Scan(probe));
                } else {
                    self.probe_finish(parser, probe, false);
                }
            }
        }
        None
    }
}
