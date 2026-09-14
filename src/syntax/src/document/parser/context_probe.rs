//! Retained physical context probes shared by document boundaries and recovery.
use super::ContextView;
use super::mechdown::{FenceDelimiter, FenceStart};
use super::terminal::is_horizontal_space;

pub(crate) enum Progress<T> {
    Complete(T),
    NeedInput,
    NeedsProcessing,
}

pub(crate) struct FenceProbe {
    relative: u32,
    delimiter: bool,
    done: Option<Option<FenceStart>>,
    pub work: u64,
}
impl FenceProbe {
    pub fn indentation_bytes(&self) -> u32 {
        self.relative
    }
    pub fn new() -> Self {
        Self {
            relative: 0,
            delimiter: false,
            done: None,
            work: 0,
        }
    }
    pub fn advance(
        &mut self,
        view: ContextView<'_>,
        final_input: bool,
        allowance: &mut u64,
    ) -> Progress<Option<FenceStart>> {
        if let Some(done) = self.done {
            return Progress::Complete(done);
        }
        while *allowance > 0 {
            *allowance -= 1;
            self.work += 1;
            let at = view
                .at_relative(self.relative)
                .expect("retained fence probe offset");
            if !self.delimiter {
                if let Some(character) = at.peek_char().filter(|ch| is_horizontal_space(*ch)) {
                    self.relative += character.len_utf8() as u32;
                    continue;
                }
                if at.offset() == at.end() && !final_input {
                    return Progress::NeedInput;
                }
                self.delimiter = true;
            }
            let delimiter = match at.byte_at(0) {
                Some(b'`') => Some((b'`', FenceDelimiter::Grave)),
                Some(b'~') => Some((b'~', FenceDelimiter::Tilde)),
                _ => None,
            };
            let result = if let Some((byte, delimiter)) = delimiter {
                for i in 1..3 {
                    match at.byte_at(i) {
                        Some(found) if found == byte => {}
                        None if !final_input => return Progress::NeedInput,
                        _ => {
                            self.done = Some(None);
                            return Progress::Complete(None);
                        }
                    }
                }
                Some(FenceStart {
                    delimiter,
                    indentation_bytes: self.relative,
                })
            } else {
                None
            };
            self.done = Some(result);
            return Progress::Complete(result);
        }
        Progress::NeedsProcessing
    }
}

#[derive(Clone, Copy)]
enum SubtitlePhase {
    Start,
    Name(bool),
    BeforeTitle,
    Title {
        any: bool,
        qualified: bool,
        previous_was_horizontal: bool,
    },
    TitleNewline,
    Dashes(bool),
    AfterDashes,
}
pub(crate) struct SubtitleProbe {
    relative: u32,
    phase: SubtitlePhase,
    done: Option<bool>,
    pub work: u64,
}
impl SubtitleProbe {
    pub fn new() -> Self {
        Self {
            relative: 0,
            phase: SubtitlePhase::Start,
            done: None,
            work: 0,
        }
    }
    pub fn advance(
        &mut self,
        view: ContextView<'_>,
        final_input: bool,
        allowance: &mut u64,
    ) -> Progress<bool> {
        if let Some(done) = self.done {
            return Progress::Complete(done);
        }
        while *allowance > 0 {
            *allowance -= 1;
            self.work += 1;
            let at = view
                .at_relative(self.relative)
                .expect("retained subtitle probe offset");
            let character = at.peek_char();
            // Every unfinished phase can change when more physical input arrives.
            if character.is_none() && !final_input {
                return Progress::NeedInput;
            }
            let result = match self.phase {
                SubtitlePhase::Start => {
                    if view.is_line_start() {
                        self.phase = SubtitlePhase::Name(false);
                        None
                    } else {
                        Some(false)
                    }
                }
                SubtitlePhase::Name(any) => {
                    if let Some(character) = character.filter(|ch| ch.is_alphanumeric()) {
                        self.relative += character.len_utf8() as u32;
                        self.phase = SubtitlePhase::Name(true);
                        None
                    } else if any && character == Some('.') {
                        self.relative += 1;
                        self.phase = SubtitlePhase::BeforeTitle;
                        None
                    } else {
                        self.phase = SubtitlePhase::Title {
                            any,
                            qualified: false,
                            previous_was_horizontal: false,
                        };
                        None
                    }
                }
                SubtitlePhase::BeforeTitle => {
                    if let Some(character) = character.filter(|ch| is_horizontal_space(*ch)) {
                        self.relative += character.len_utf8() as u32;
                    } else {
                        self.phase = SubtitlePhase::Title {
                            any: false,
                            qualified: true,
                            previous_was_horizontal: false,
                        };
                    }
                    None
                }
                SubtitlePhase::Title {
                    any,
                    qualified,
                    previous_was_horizontal,
                } => {
                    if let Some(character) = character.filter(|ch| !matches!(ch, '\r' | '\n')) {
                        let annotation = character == '@'
                            && previous_was_horizontal
                            && view
                                .at_relative(self.relative + character.len_utf8() as u32)
                                .and_then(|next| next.peek_char())
                                .is_some_and(|next| next.is_alphabetic() || next == '_');
                        self.relative += character.len_utf8() as u32;
                        self.phase = SubtitlePhase::Title {
                            any: true,
                            qualified: qualified || annotation,
                            previous_was_horizontal: is_horizontal_space(character),
                        };
                        None
                    } else if any && qualified {
                        self.phase = SubtitlePhase::TitleNewline;
                        None
                    } else {
                        Some(false)
                    }
                }
                SubtitlePhase::TitleNewline => match character {
                    Some('\r') => {
                        if at.byte_at(1).is_none() && !final_input {
                            return Progress::NeedInput;
                        }
                        self.relative += if at.byte_at(1) == Some(b'\n') { 2 } else { 1 };
                        self.phase = SubtitlePhase::Dashes(false);
                        None
                    }
                    Some('\n') => {
                        self.relative += 1;
                        self.phase = SubtitlePhase::Dashes(false);
                        None
                    }
                    _ => Some(false),
                },
                SubtitlePhase::Dashes(any) => {
                    if character == Some('-') {
                        self.relative += 1;
                        self.phase = SubtitlePhase::Dashes(true);
                        None
                    } else if any {
                        self.phase = SubtitlePhase::AfterDashes;
                        None
                    } else {
                        Some(false)
                    }
                }
                SubtitlePhase::AfterDashes => {
                    if let Some(character) = character.filter(|ch| is_horizontal_space(*ch)) {
                        self.relative += character.len_utf8() as u32;
                        None
                    } else {
                        Some(matches!(character, None | Some('\r' | '\n')))
                    }
                }
            };
            if let Some(result) = result {
                self.done = Some(result);
                return Progress::Complete(result);
            }
        }
        Progress::NeedsProcessing
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::parser::Cursor;
    use crate::document::{DocumentId, Revision, TextSize, TextSnapshot};
    use alloc::{string::String, vec::Vec};

    fn probes(
        text: &str,
        chunks: &[&str],
        step: bool,
    ) -> (bool, Option<(FenceDelimiter, u32)>, u64) {
        let mut source = TextSnapshot::new(DocumentId(826), Revision(0), "").unwrap();
        let mut subtitle = SubtitleProbe::new();
        let mut fence = FenceProbe::new();
        for chunk in chunks {
            source = source.append(*chunk).unwrap();
            let cursor = Cursor::new(&source);
            loop {
                let mut allowance = if step { 1 } else { u64::MAX };
                if !matches!(
                    subtitle.advance(cursor.context_view(), false, &mut allowance),
                    Progress::NeedsProcessing
                ) {
                    break;
                }
            }
            loop {
                let mut allowance = if step { 1 } else { u64::MAX };
                if !matches!(
                    fence.advance(cursor.context_view(), false, &mut allowance),
                    Progress::NeedsProcessing
                ) {
                    break;
                }
            }
        }
        assert_eq!(source.byte_len(), TextSize(text.len() as u32));
        let cursor = Cursor::new(&source);
        let subtitle_result = loop {
            let mut allowance = if step { 1 } else { u64::MAX };
            match subtitle.advance(cursor.context_view(), true, &mut allowance) {
                Progress::Complete(result) => break result,
                Progress::NeedsProcessing => {}
                Progress::NeedInput => panic!("sealed subtitle probe"),
            }
        };
        let fence_result = loop {
            let mut allowance = if step { 1 } else { u64::MAX };
            match fence.advance(cursor.context_view(), true, &mut allowance) {
                Progress::Complete(result) => {
                    break result.map(|found| (found.delimiter, found.indentation_bytes));
                }
                Progress::NeedsProcessing => {}
                Progress::NeedInput => panic!("sealed fence probe"),
            }
        };
        (subtitle_result, fence_result, subtitle.work + fence.work)
    }

    #[test]
    fn context_probes_retain_crlf_indentation_and_unfinished_heading_decisions() {
        for (text, subtitle, fence) in [
            ("", false, None),
            ("```", false, Some((FenceDelimiter::Grave, 0))),
            (" \u{a0}\t~~~text", false, Some((FenceDelimiter::Tilde, 4))),
            (" ``", false, None),
            (" ~~x", false, None),
            ("A. title\r\n---\t\n", true, None),
            ("é1.\u{2009}💡\n-", true, None),
            ("1. title\r---", true, None),
            ("1. title\n--- \u{a0}", true, None),
            ("1. title\n--- x", false, None),
            ("1. \n---", false, None),
            ("1. title", false, None),
            ("1. title\n", false, None),
            ("1. title\n--\u{301}", false, None),
            ("1. title\r\n---\r\nnext", true, None),
            ("calculation @compute\n---\n", true, None),
            ("calculation@compute\n---\n", false, None),
            ("plain title\n---\n", false, None),
        ] {
            let expected = (subtitle, fence);
            let boundaries: Vec<_> = text
                .char_indices()
                .map(|(at, _)| at)
                .chain(core::iter::once(text.len()))
                .collect();
            for split in &boundaries {
                let (subtitle, fence, _) = probes(text, &[&text[..*split], &text[*split..]], true);
                assert_eq!((subtitle, fence), expected, "{text:?}, split {split}");
            }
            let chunks: Vec<_> = boundaries
                .windows(2)
                .map(|pair| &text[pair[0]..pair[1]])
                .collect();
            let (subtitle, fence, _) = probes(text, &chunks, true);
            assert_eq!((subtitle, fence), expected, "{text:?}, scalar chunks");
        }
    }

    #[test]
    fn context_probe_work_grows_with_input_not_repeated_prefix_scans() {
        for (prefix, unit, tail) in [
            ("", " ", "```"),
            ("", "a", ". title\n---"),
            ("1. ", "💡", "\n---"),
            ("1. title\n", "-", "x"),
        ] {
            let mut previous = None;
            for n in [64, 128, 256, 512] {
                let text = String::from(prefix) + &unit.repeat(n) + tail;
                let boundaries: Vec<_> = text
                    .char_indices()
                    .map(|(at, _)| at)
                    .chain(core::iter::once(text.len()))
                    .collect();
                let chunks: Vec<_> = boundaries
                    .windows(2)
                    .map(|pair| &text[pair[0]..pair[1]])
                    .collect();
                let (subtitle, fence, work) = probes(&text, &chunks, true);
                let (expected_subtitle, expected_fence, _) = probes(&text, &[&text], false);
                assert_eq!((subtitle, fence), (expected_subtitle, expected_fence));
                if let Some(previous) = previous {
                    assert!(work <= previous * 3);
                }
                previous = Some(work);
            }
        }
    }
}
