use super::super::operators;
use crate::document::parser::canonical::combinator::Attempt;

use super::*;
use crate::document::parser::LexicalMode;
use crate::document::{DocumentId, IdGenerator, ParseConfig, ParseStats, Revision, TextSnapshot};
use alloc::{string::String, vec::Vec};
#[derive(Debug, PartialEq)]
struct Observation {
    result: Attempt,
    work: u64,
    checkpoint: crate::document::parser::checkpoint::ParserCheckpoint,
    events: String,
    diagnostics: String,
    ids: String,
    stats: ParseStats,
    fuel: u64,
    rewinds: u64,
    rewound_bytes: u64,
    rule_peak: usize,
    event_peak: usize,
    marker_peak: usize,
}
fn observe(
    rule: RuleId,
    text: &str,
    offset: u32,
    max_events: u32,
    fuel: u64,
    reference: bool,
) -> Observation {
    let source = TextSnapshot::new(DocumentId(0x52454a), Revision(1), text).unwrap();
    let mut ids = IdGenerator::new();
    let mut config = ParseConfig::default();
    config.limits.max_events = max_events;
    config.limits.fuel = fuel;
    let mut parser = Parser::new(
        &source,
        LexicalMode::CanonicalSourceFragment,
        config,
        &mut ids,
    );
    parser.cursor.bump_bytes(offset).unwrap();
    parser.state.disable_rejections_for_test = reference;
    let _wrapper = parser.start();
    let mut child = operators::Continuation::new(rule);
    let mut first = true;
    let result = loop {
        let mut allowance = if first && reference { 1 } else { u64::MAX };
        first = false;
        match child.advance(&mut parser, true, &mut allowance) {
            operators::Progress::Complete(result) => break result,
            operators::Progress::NeedsProcessing => {}
            _ => panic!("sealed operator control"),
        }
    };
    Observation {
        result,
        work: child.work,
        checkpoint: parser.checkpoint(),
        events: alloc::format!("{:?}", parser.state.events),
        diagnostics: alloc::format!("{:?}", parser.state.diagnostics),
        ids: alloc::format!("{:?}", parser.ids),
        stats: parser.state.stats,
        fuel: parser.state.fuel,
        rewinds: parser.state.rewinds,
        rewound_bytes: parser.state.rewound_bytes,
        rule_peak: parser.state.rules.peak_depth(),
        event_peak: parser.state.peak_events,
        marker_peak: parser.state.peak_open_markers,
    }
}
#[test]
fn predicted_failures_match_original_operator_traces_at_every_ascii_head() {
    for rule in EXPRESSION_OPERATOR_RULES {
        for byte in b'!'..=b'~' {
            for prefix in ["", " ", "\t", " \t "] {
                for prior in ["", "a"] {
                    let text = alloc::format!("{prior}{prefix}{}x", byte as char);
                    assert_eq!(
                        observe(*rule, &text, prior.len() as u32, 100000, 100000, false),
                        observe(*rule, &text, prior.len() as u32, 100000, 100000, true),
                        "rule {rule:?} text {text:?}"
                    );
                }
            }
        }
    }
}
#[test]
fn predicted_failures_keep_resource_limits_unicode_and_input_bounds() {
    let mut sources = Vec::new();
    for prefix in ["", " ", "\t", " \t ", "\r\n", "\u{a0}"] {
        for text in [
            "xq",
            ")x",
            "=x",
            "+x",
            "/x",
            "--",
            "//",
            "->",
            "<x",
            "Δx",
            "≠x",
            "\u{301}x",
            "x\u{301}x",
            "👩\u{200d}💻x",
            "🇦🇧x",
            "\r\nx",
            "x",
            "",
        ] {
            sources.push(alloc::format!("{prefix}{text}"));
        }
    }
    for rule in EXPRESSION_OPERATOR_RULES {
        for text in &sources {
            for events in [0, 1, 3, 4, 7, 8, 12, 16, 32, 1000] {
                for fuel in [0, 1, 2, 4, 16, 1000] {
                    assert_eq!(
                        observe(*rule, text, 0, events, fuel, false),
                        observe(*rule, text, 0, events, fuel, true),
                        "rule {rule:?} text {text:?} events {events} fuel {fuel}"
                    );
                }
            }
        }
    }
}
fn observe_base(
    rule: RuleId,
    text: &str,
    offset: u32,
    max_events: u32,
    fuel: u64,
    reference: bool,
) -> Observation {
    let source = TextSnapshot::new(DocumentId(0x42415345), Revision(1), text).unwrap();
    let mut ids = IdGenerator::new();
    let mut config = ParseConfig::default();
    config.limits.max_events = max_events;
    config.limits.fuel = fuel;
    let mut parser = Parser::new(
        &source,
        LexicalMode::CanonicalSourceFragment,
        config,
        &mut ids,
    );
    parser.state.disable_rejections_for_test = reference;
    parser.cursor.bump_bytes(offset).unwrap();
    let _wrapper = parser.start();
    let mut child = base::continuation::Continuation::new(rule);
    let mut first = true;
    let result = loop {
        let mut allowance = if first && reference { 1 } else { u64::MAX };
        first = false;
        match child.advance(&mut parser, true, &mut allowance) {
            base::continuation::Progress::Complete(result) => {
                break if result {
                    Attempt::Matched
                } else {
                    Attempt::NoMatch
                };
            }
            base::continuation::Progress::NeedsProcessing => {}
            _ => panic!("sealed base control"),
        }
    };
    Observation {
        result,
        work: child.work,
        checkpoint: parser.checkpoint(),
        events: alloc::format!("{:?}", parser.state.events),
        diagnostics: alloc::format!("{:?}", parser.state.diagnostics),
        ids: alloc::format!("{:?}", parser.ids),
        stats: parser.state.stats,
        fuel: parser.state.fuel,
        rewinds: parser.state.rewinds,
        rewound_bytes: parser.state.rewound_bytes,
        rule_peak: parser.state.rules.peak_depth(),
        event_peak: parser.state.peak_events,
        marker_peak: parser.state.peak_open_markers,
    }
}
#[test]
fn predicted_base_recognition_keeps_original_traces_at_every_ascii_head() {
    for rule in base::NON_FIXED_RULES {
        for byte in 0..=127_u8 {
            for prior in ["", "a", "é"] {
                for tail in ["x", "\u{301}x", ""] {
                    let text = alloc::format!("{prior}{}{tail}", byte as char);
                    assert_eq!(
                        observe_base(*rule, &text, prior.len() as u32, 100000, 100000, false),
                        observe_base(*rule, &text, prior.len() as u32, 100000, 100000, true),
                        "base rule {rule:?} text {text:?}"
                    );
                }
            }
        }
    }
}
#[test]
fn predicted_base_recognition_keeps_tight_limits_and_unicode_decisions() {
    for rule in base::NON_FIXED_RULES {
        for text in [
            "xq",
            " ",
            " xy",
            "\txy",
            "34",
            "_x",
            "👩\u{200d}💻x",
            "x\u{301}x",
            "éx",
            "🇦🇧x",
            "\u{a0}x",
            "\r\nx",
            "",
        ] {
            for events in [0, 1, 3, 4, 5, 6, 8, 12, 16, 32, 1000] {
                for fuel in [0, 1, 2, 4, 16, 1000] {
                    assert_eq!(
                        observe_base(*rule, text, 0, events, fuel, false),
                        observe_base(*rule, text, 0, events, fuel, true),
                        "base rule {rule:?} text {text:?} events {events} fuel {fuel}"
                    );
                }
            }
        }
    }
}

#[test]
fn long_whitespace_prefixes_decline_without_changing_parsing() {
    for count in [127, 128, 129, 1024] {
        for rule in EXPRESSION_OPERATOR_RULES {
            let text = alloc::format!("{}xq", " ".repeat(count));
            assert_eq!(
                observe(*rule, &text, 0, 100000, 100000, false),
                observe(*rule, &text, 0, 100000, 100000, true)
            );
        }
    }
}
