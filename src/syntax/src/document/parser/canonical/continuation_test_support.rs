//! Shared append/allowance/resource qualification for canonical continuations.
use super::combinator::Attempt;
use crate::document::parser::{LexicalMode, Parser};
use crate::document::{
    DiagnosticAnchor, DocumentId, IdGenerator, ParseConfig, ParseStats, Revision, RuleId,
    SyntaxKind, TextSize, TextSnapshot,
};
use alloc::{string::String, vec::Vec};

pub(super) enum TestProgress {
    Complete(Attempt),
    NeedInput,
    NeedsProcessing,
    Limited,
}
pub(super) trait TestContinuation {
    fn new(rule: RuleId) -> Self;
    fn work(&self) -> u64;
    fn advance(
        &mut self,
        parser: &mut Parser<'_>,
        final_input: bool,
        allowance: &mut u64,
    ) -> TestProgress;
}
macro_rules! qualify_continuation {
    ($module:ident) => {
        impl TestContinuation for super::$module::Continuation {
            fn new(rule: RuleId) -> Self {
                Self::new(rule)
            }
            fn work(&self) -> u64 {
                self.work
            }
            fn advance(
                &mut self,
                parser: &mut Parser<'_>,
                final_input: bool,
                allowance: &mut u64,
            ) -> TestProgress {
                match self.advance(parser, final_input, allowance) {
                    super::$module::Progress::Complete(result) => TestProgress::Complete(result),
                    super::$module::Progress::NeedsProcessing => TestProgress::NeedsProcessing,
                    super::$module::Progress::NeedInput => TestProgress::NeedInput,
                    super::$module::Progress::Limited => TestProgress::Limited,
                }
            }
        }
    };
}
qualify_continuation!(strings);
qualify_continuation!(statements);
qualify_continuation!(prose);
qualify_continuation!(operators);
qualify_continuation!(paths);
qualify_continuation!(kinds);
qualify_continuation!(mechdown);
qualify_continuation!(literals);
qualify_continuation!(primitives);
qualify_continuation!(structure_shell);
qualify_continuation!(declarations);
qualify_continuation!(source_imports);
qualify_continuation!(imports);
qualify_continuation!(recursive_core);

#[derive(Debug, PartialEq)]
pub(super) struct Output {
    pub result: Attempt,
    pub end: TextSize,
    pub events: String,
    pub diagnostics: String,
    pub stats: ParseStats,
}
pub(super) fn run<T: TestContinuation>(
    rule: RuleId,
    text: &str,
    chunks: &[&str],
    fuel: u64,
    step: bool,
) -> (Output, u64) {
    let mut config = ParseConfig::default();
    config.limits.fuel = fuel;
    run_with_config::<T>(rule, text, chunks, config, step)
}
fn run_with_config<T: TestContinuation>(
    rule: RuleId,
    text: &str,
    chunks: &[&str],
    config: ParseConfig,
    step: bool,
) -> (Output, u64) {
    assert_eq!(chunks.concat(), text);
    let mut source = TextSnapshot::new(DocumentId(826), Revision(0), "").unwrap();
    let mut ids = IdGenerator::new();
    let mut parser = Parser::new(
        &source,
        LexicalMode::CanonicalSourceFragment,
        config,
        &mut ids,
    );
    let wrapper = parser.start();
    let mut state = parser.suspend();
    let mut continuation = T::new(rule);
    'input: for chunk in chunks {
        source = source.append(*chunk).unwrap();
        let mut parser = Parser::resume(&source, state, &mut ids);
        loop {
            let mut allowance = if step { 1 } else { u64::MAX };
            let before = continuation.work();
            let progress = continuation.advance(&mut parser, false, &mut allowance);
            if step {
                assert!(continuation.work() - before <= 1);
            }
            assert!(continuation.work() < 10_000_000);
            if matches!(progress, TestProgress::Limited) {
                state = parser.suspend();
                break 'input;
            }
            if !matches!(progress, TestProgress::NeedsProcessing) {
                break;
            }
            let suspended = parser.suspend();
            parser = Parser::resume(&source, suspended, &mut ids);
        }
        state = parser.suspend();
    }
    let mut parser = Parser::resume(&source, state, &mut ids);
    let result = loop {
        let mut allowance = if step { 1 } else { u64::MAX };
        let before = continuation.work();
        let progress = continuation.advance(&mut parser, true, &mut allowance);
        if step {
            assert!(continuation.work() - before <= 1);
        }
        assert!(continuation.work() < 10_000_000);
        match progress {
            TestProgress::Complete(result) => break result,
            TestProgress::NeedsProcessing => {
                let suspended = parser.suspend();
                parser = Parser::resume(&source, suspended, &mut ids);
            }
            _ => panic!("final continuation input must drain"),
        }
    };
    let end = parser.offset();
    assert_eq!(parser.state.rules.len(), 0);
    wrapper.complete(&mut parser, SyntaxKind::Document);
    let mut output = parser.finish();
    // Revision numbers count append operations; physical diagnostic content
    // and speculative identities must otherwise remain exactly identical.
    for pending in &mut output.diagnostics {
        let diagnostic = &mut pending.diagnostic;
        if let DiagnosticAnchor::Absolute { revision, .. } = &mut diagnostic.primary {
            *revision = Revision(0);
        }
        for label in &mut diagnostic.labels {
            if let DiagnosticAnchor::Absolute { revision, .. } = &mut label.anchor {
                *revision = Revision(0);
            }
        }
    }
    (
        Output {
            result,
            end,
            events: alloc::format!("{:?}", output.events),
            diagnostics: alloc::format!(
                "{:?}",
                output
                    .diagnostics
                    .iter()
                    .map(|pending| &pending.diagnostic)
                    .collect::<Vec<_>>()
            ),
            stats: output.stats,
        },
        continuation.work(),
    )
}

/// Check all two-chunk cuts plus scalar chunks, with the same accepted prefix
/// when a hard limit stops ingestion. Every work yield suspends the parser too.
pub(super) fn assert_partitions<T: TestContinuation>(cases: &[(RuleId, &str)]) {
    for &(rule, text) in cases {
        let boundaries: Vec<_> = text
            .char_indices()
            .map(|(at, _)| at)
            .chain(core::iter::once(text.len()))
            .collect();
        for fuel in [0, 1, 2, 4, 8, 16, 64, u64::MAX] {
            for split in &boundaries {
                let (observed, _) =
                    run::<T>(rule, text, &[&text[..*split], &text[*split..]], fuel, true);
                let accepted = &text[..observed.stats.source_bytes as usize];
                let (baseline, _) = run::<T>(rule, accepted, &[accepted], fuel, false);
                assert_eq!(
                    observed, baseline,
                    "{rule:?}, split {split}, fuel {fuel}, source {text:?}"
                );
            }
            let chunks: Vec<_> = boundaries
                .windows(2)
                .map(|pair| &text[pair[0]..pair[1]])
                .collect();
            let (observed, _) = run::<T>(rule, text, &chunks, fuel, true);
            let accepted = &text[..observed.stats.source_bytes as usize];
            let (baseline, _) = run::<T>(rule, accepted, &[accepted], fuel, false);
            assert_eq!(
                observed, baseline,
                "{rule:?}, scalar chunks, fuel {fuel}, source {text:?}"
            );
        }
    }
}

mod recovery_tests {
    use super::*;
    use crate::document::parser::recovery::{RecoveryClass, SkipContinuation, SkipProgress};
    use crate::document::parser::rules;
    struct Skip(SkipContinuation<'static>);
    impl TestContinuation for Skip {
        fn new(rule: RuleId) -> Self {
            let class = match rule {
                rules::MECH_CODE => RecoveryClass::MechItem,
                rules::PARAGRAPH => RecoveryClass::Paragraph,
                rules::CODE_BLOCK => RecoveryClass::Fence,
                _ => unreachable!("recovery test class"),
            };
            Self(SkipContinuation::new(
                class,
                "syntax/skip-test",
                "test recovery",
            ))
        }
        fn work(&self) -> u64 {
            self.0.work
        }
        fn advance(
            &mut self,
            parser: &mut Parser<'_>,
            final_input: bool,
            allowance: &mut u64,
        ) -> TestProgress {
            match self.0.advance(parser, final_input, allowance) {
                SkipProgress::Complete(result) => TestProgress::Complete(if result.is_some() {
                    Attempt::Matched
                } else {
                    Attempt::NoMatch
                }),
                SkipProgress::NeedInput => TestProgress::NeedInput,
                SkipProgress::NeedsProcessing => TestProgress::NeedsProcessing,
                SkipProgress::Limited => TestProgress::Limited,
            }
        }
    }
    #[test]
    fn recovery_boundaries_found_text_and_limits_survive_input_cuts() {
        assert_partitions::<Skip>(&[
            (rules::MECH_CODE, "broken;next"),
            (rules::MECH_CODE, "broken\r\nnext"),
            (rules::MECH_CODE, "\n1. title\r\n---"),
            (rules::MECH_CODE, "\n1. title\r\n--x"),
            (rules::MECH_CODE, "\n ```next"),
            (rules::MECH_CODE, "\r\nnext"),
            (rules::PARAGRAPH, "é💡   ```next"),
            (rules::PARAGRAPH, "é💡\u{a0}\u{2009}~~~next"),
            (rules::PARAGRAPH, "a   ~~x"),
            (rules::PARAGRAPH, "a   ``"),
            (rules::PARAGRAPH, "a\r\nnext"),
            (rules::CODE_BLOCK, "é💡\u{301}\r\n```"),
            (rules::CODE_BLOCK, ""),
        ]);
        for (rule, text, end) in [
            (rules::MECH_CODE, "broken;next", 6),
            (rules::MECH_CODE, "\n1. title\r\n---", 1),
            (rules::MECH_CODE, "\n1. title\r\n--x", 9),
            (rules::PARAGRAPH, "é💡   ```next", 6),
            (rules::CODE_BLOCK, "é💡\u{301}\r\n```", 13),
        ] {
            let (output, _) = run::<Skip>(rule, text, &[text], u64::MAX, false);
            assert_eq!(output.end.to_usize(), end, "{text:?}");
            assert_eq!(output.stats.recovery_bytes as usize, end);
            assert_eq!(output.stats.diagnostics_emitted, 1);
        }
    }
    #[test]
    fn recovery_byte_and_event_limits_are_cumulative_across_appends() {
        for text in ["é💡abc", "a   ```", "\n1. title\n---", "abc;next"] {
            let boundaries: Vec<_> = text
                .char_indices()
                .map(|(at, _)| at)
                .chain(core::iter::once(text.len()))
                .collect();
            let chunks: Vec<_> = boundaries
                .windows(2)
                .map(|pair| &text[pair[0]..pair[1]])
                .collect();
            for recovery_limit in 0..=12 {
                for events in [8, 16, 64] {
                    let mut config = ParseConfig::default();
                    config.limits.max_recovery_bytes = recovery_limit;
                    config.limits.max_events = events;
                    let (observed, _) =
                        run_with_config::<Skip>(rules::PARAGRAPH, text, &chunks, config, true);
                    let accepted = &text[..observed.stats.source_bytes as usize];
                    let (expected, _) = run_with_config::<Skip>(
                        rules::PARAGRAPH,
                        accepted,
                        &[accepted],
                        config,
                        false,
                    );
                    assert_eq!(
                        observed, expected,
                        "{text:?}, recovery {recovery_limit}, events {events}"
                    );
                    assert!(observed.stats.recovery_bytes <= u64::from(recovery_limit));
                }
            }
        }
    }
    #[test]
    fn recovery_indent_probes_and_found_export_retain_linear_work() {
        for (rule, prefix, unit, tail) in [
            (rules::PARAGRAPH, "a", " ", "x"),
            (rules::PARAGRAPH, "a", " ", "```"),
            (rules::MECH_CODE, "\n1. ", "a", "\n---"),
            (rules::MECH_CODE, "\n1. ", "a", "\n--x"),
            (rules::CODE_BLOCK, "", "💡", ""),
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
                let (observed, work) = run::<Skip>(rule, &text, &chunks, u64::MAX, true);
                let (expected, one_shot_work) = run::<Skip>(rule, &text, &[&text], u64::MAX, false);
                assert_eq!(observed, expected, "{rule:?}, tail {tail:?}");
                assert_eq!(observed.stats.source_bytes as usize, text.len());
                assert_eq!(observed.stats.diagnostics_emitted, 1);
                if let Some((prior_streamed, prior_one_shot)) = previous {
                    assert!(work <= prior_streamed * 3, "repeated recovery lookahead");
                    assert!(one_shot_work <= prior_one_shot * 3);
                }
                previous = Some((work, one_shot_work));
            }
        }
    }
}

mod abandonment_tests {
    use super::*;
    use crate::document::parser::recovery::{
        AbandonContinuation, AbandonProgress, BoundaryProgress,
    };
    use crate::document::parser::rules;
    struct Abandon(AbandonContinuation<'static>);
    impl TestContinuation for Abandon {
        fn new(rule: RuleId) -> Self {
            Self(AbandonContinuation::new(
                rule,
                "syntax/test-abandon",
                "test abandonment",
            ))
        }
        fn work(&self) -> u64 {
            self.0.work
        }
        fn advance(
            &mut self,
            parser: &mut Parser<'_>,
            final_input: bool,
            allowance: &mut u64,
        ) -> TestProgress {
            match self
                .0
                .advance(parser, final_input, allowance, |_, ch, _, allowance| {
                    if *allowance == 0 {
                        return BoundaryProgress::NeedsProcessing;
                    }
                    *allowance -= 1;
                    BoundaryProgress::Complete(matches!(
                        ch,
                        ';' | ',' | ')' | ']' | '}' | '>' | '⟩' | '\n' | '\r'
                    ))
                }) {
                AbandonProgress::Complete(result) => TestProgress::Complete(if result.is_some() {
                    Attempt::Matched
                } else {
                    Attempt::NoMatch
                }),
                AbandonProgress::NeedInput => TestProgress::NeedInput,
                AbandonProgress::NeedsProcessing => TestProgress::NeedsProcessing,
                AbandonProgress::Limited => TestProgress::Limited,
            }
        }
    }
    #[test]
    fn balanced_abandonment_preserves_quotes_angles_and_ancestor_boundaries_at_all_cuts() {
        let cases = [
            "",
            ";next",
            "bad;next",
            "(a,b);next",
            "[a,(b,c)];next",
            "[a};next",
            "\"quoted;source\";next",
            "\"a\\\";b\";next",
            "\"\"\"raw;source\"\"\";next",
            "\"\"\"raw\"\"",
            "\"",
            "\"\"",
            "é💡\u{301};next",
            "x\r\nnext",
            "<u8>;next",
            "<foo,bar>;next",
            "<u8;next",
            "<-x;next",
            "<=x;next",
            "<+x;next",
            "⟨u8⟩;next",
            "\"<u8>;\";next",
            "\"\"\"<u8>;\"\"\";next",
        ];
        let cases: Vec<_> = cases
            .into_iter()
            .map(|text| (rules::EXPRESSION, text))
            .collect();
        assert_partitions::<Abandon>(&cases);
        for &(rule, text) in &cases {
            let boundaries: Vec<_> = text
                .char_indices()
                .map(|(at, _)| at)
                .chain(core::iter::once(text.len()))
                .collect();
            let chunks: Vec<_> = boundaries
                .windows(2)
                .map(|pair| &text[pair[0]..pair[1]])
                .collect();
            for limit in 0..=12 {
                for events in [8, 16, 64] {
                    let mut config = ParseConfig::default();
                    config.limits.max_recovery_bytes = limit;
                    config.limits.max_events = events;
                    let (observed, _) =
                        run_with_config::<Abandon>(rule, text, &chunks, config, true);
                    let accepted = &text[..observed.stats.source_bytes as usize];
                    let (expected, _) =
                        run_with_config::<Abandon>(rule, accepted, &[accepted], config, false);
                    assert_eq!(
                        observed, expected,
                        "{text:?}, recovery {limit}, events {events}"
                    );
                    assert!(observed.stats.recovery_bytes <= u64::from(limit));
                }
            }
        }
    }
    #[test]
    fn balanced_abandonment_scanning_and_export_are_linear() {
        for (prefix, unit, suffix) in [
            ("", "💡", ";next"),
            ("\"", "abc;", "\";next"),
            ("\"\"\"", "é;", "\"\"\";next"),
            ("(", "a,", ");next"),
        ] {
            let mut previous = None;
            for n in [64, 128, 256, 512] {
                let text = String::from(prefix) + &unit.repeat(n) + suffix;
                let boundaries: Vec<_> = text
                    .char_indices()
                    .map(|(at, _)| at)
                    .chain(core::iter::once(text.len()))
                    .collect();
                let chunks: Vec<_> = boundaries
                    .windows(2)
                    .map(|pair| &text[pair[0]..pair[1]])
                    .collect();
                let (observed, work) =
                    run::<Abandon>(rules::EXPRESSION, &text, &chunks, u64::MAX, true);
                let (expected, sealed_work) =
                    run::<Abandon>(rules::EXPRESSION, &text, &[&text], u64::MAX, false);
                assert_eq!(observed, expected);
                assert_eq!(observed.stats.source_bytes as usize, text.len());
                assert_eq!(observed.end.to_usize(), text.len() - 5);
                if let Some((prior, prior_sealed)) = previous {
                    assert!(work <= prior * 3);
                    assert!(sealed_work <= prior_sealed * 3);
                }
                previous = Some((work, sealed_work));
            }
        }
    }
}

mod nesting_tests {
    use super::*;
    use crate::document::parser::recovery::{NestingContinuation, NestingProgress};
    use crate::document::parser::rules;
    impl TestContinuation for NestingContinuation {
        fn new(_: RuleId) -> Self {
            Self::new()
        }
        fn work(&self) -> u64 {
            self.work
        }
        fn advance(
            &mut self,
            parser: &mut Parser<'_>,
            final_input: bool,
            allowance: &mut u64,
        ) -> TestProgress {
            match self.advance(parser, final_input, allowance) {
                NestingProgress::Complete => TestProgress::Complete(Attempt::Committed),
                NestingProgress::NeedInput => TestProgress::NeedInput,
                NestingProgress::NeedsProcessing => TestProgress::NeedsProcessing,
                NestingProgress::Limited => TestProgress::Limited,
            }
        }
    }
    #[test]
    fn nesting_recovery_preserves_depth_and_document_boundaries_across_all_cuts() {
        let cases = [
            "",
            ")next",
            ";next",
            "--comment",
            "//comment",
            "-",
            "/",
            "(a;b)c;next",
            "((a)b)c)next",
            "é💡\u{301}\r\nnext",
            "1. heading\n---",
            "1. heading\r\n--x",
            "   ```next",
            "   ~~x",
            "a```next",
            "a--comment",
            "a//comment",
        ];
        let cases: Vec<_> = cases
            .into_iter()
            .map(|text| (rules::FACTOR, text))
            .collect();
        assert_partitions::<NestingContinuation>(&cases);
        for &(rule, text) in &cases {
            let boundaries: Vec<_> = text
                .char_indices()
                .map(|(at, _)| at)
                .chain(core::iter::once(text.len()))
                .collect();
            let chunks: Vec<_> = boundaries
                .windows(2)
                .map(|pair| &text[pair[0]..pair[1]])
                .collect();
            for limit in 0..=12 {
                for events in [8, 16, 64] {
                    let mut config = ParseConfig::default();
                    config.limits.max_recovery_bytes = limit;
                    config.limits.max_events = events;
                    let (observed, _) =
                        run_with_config::<NestingContinuation>(rule, text, &chunks, config, true);
                    let accepted = &text[..observed.stats.source_bytes as usize];
                    let (expected, _) = run_with_config::<NestingContinuation>(
                        rule,
                        accepted,
                        &[accepted],
                        config,
                        false,
                    );
                    assert_eq!(
                        observed, expected,
                        "{text:?}, recovery {limit}, events {events}"
                    );
                    assert!(observed.stats.recovery_bytes <= u64::from(limit));
                }
            }
        }
    }
    #[test]
    fn nesting_recovery_retains_linear_heading_fence_and_export_work() {
        for (prefix, unit, tail) in [
            ("", " ", "x"),
            ("", " ", "```"),
            ("1. ", "a", "\n---"),
            ("1. ", "a", "\n--x"),
            ("(", "💡", ");next"),
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
                let (observed, work) =
                    run::<NestingContinuation>(rules::FACTOR, &text, &chunks, u64::MAX, true);
                let (expected, sealed_work) =
                    run::<NestingContinuation>(rules::FACTOR, &text, &[&text], u64::MAX, false);
                assert_eq!(observed, expected);
                assert_eq!(observed.stats.source_bytes as usize, text.len());
                if let Some((prior, prior_sealed)) = previous {
                    assert!(work <= prior * 3);
                    assert!(sealed_work <= prior_sealed * 3);
                }
                previous = Some((work, sealed_work));
            }
        }
    }
}
