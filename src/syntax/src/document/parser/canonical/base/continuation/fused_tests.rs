//! Compare each yield boundary with the ordinary phase stack.
use super::*;
use crate::document::parser::LexicalMode;
use crate::document::{DocumentId, IdGenerator, ParseConfig, Revision, TextSnapshot};
use alloc::{format, string::String, vec::Vec};

fn snapshot(
    parser: &Parser<'_>,
    continuation: &Continuation,
    phase: &str,
    progress: &Progress,
    remaining: u64,
) -> String {
    let result = match progress {
        Progress::Complete(value) => format!("complete:{value}"),
        Progress::NeedInput => String::from("input"),
        Progress::NeedsProcessing => String::from("work"),
        Progress::Limited => String::from("limited"),
    };
    let events: Vec<_> = parser
        .state
        .events
        .iter_range(0..parser.state.events.len())
        .collect();
    format!(
        "{phase}:{result}:{remaining}:{:?}:{:?}:{:?}:{:?}",
        (
            parser.offset(),
            continuation.result,
            continuation.raw_range,
            continuation.work,
            continuation.storage.frames.len(),
            parser.state.fuel,
            parser.state.stats,
        ),
        (
            parser.state.rules.len(),
            parser.state.rules.peak_depth(),
            parser.state.open_markers.len(),
            parser.state.peak_open_markers,
            parser.state.peak_events,
            parser.state.rewinds,
            parser.state.rewound_bytes,
            parser.state.covered_end,
        ),
        (
            parser.ids.next_node(),
            parser.ids.next_token(),
            parser.ids.next_diagnostic(),
            &parser.state.grapheme_tail,
        ),
        events,
    )
}

fn trace(
    rule: RuleId,
    text: &str,
    grant: u64,
    fuel: u64,
    max_events: u32,
    initially_open: bool,
    disabled: bool,
) -> Vec<String> {
    let source = TextSnapshot::new(DocumentId(391), Revision(0), text).unwrap();
    let mut ids = IdGenerator::new();
    let mut config = ParseConfig::default();
    config.limits.fuel = fuel;
    config.limits.max_events = max_events;
    let mut parser = Parser::new(
        &source,
        LexicalMode::CanonicalSourceFragment,
        config,
        &mut ids,
    );
    let wrapper = parser.start();
    let mut continuation = Continuation::new(rule);
    continuation.disable_sealed_fusion = disabled;
    let mut sealed = !initially_open;
    let mut rows = Vec::new();
    loop {
        let mut zero = 0;
        let progress = continuation.advance(&mut parser, sealed, &mut zero);
        rows.push(snapshot(&parser, &continuation, "zero", &progress, zero));
        if matches!(progress, Progress::Limited | Progress::NeedInput) {
            assert!(!sealed);
            sealed = true;
        }
        let before = continuation.work;
        let mut remaining = grant;
        let progress = continuation.advance(&mut parser, sealed, &mut remaining);
        assert!(continuation.work - before <= grant);
        rows.push(snapshot(
            &parser,
            &continuation,
            "paid",
            &progress,
            remaining,
        ));
        if matches!(progress, Progress::Complete(_)) {
            break;
        }
        if matches!(progress, Progress::Limited | Progress::NeedInput) {
            assert!(!sealed);
            sealed = true;
        }
        let suspended = parser.suspend();
        parser = Parser::resume(&source, suspended, &mut ids);
        assert!(rows.len() < 20_000);
    }
    assert_eq!(parser.state.rules.len(), 0);
    wrapper.complete(&mut parser, SyntaxKind::Document);
    let output = parser.finish();
    rows.push(format!("output:{:?}:{:?}", output.events, output.stats));
    rows
}

#[test]
fn sealed_classification_fusion_preserves_every_yield_and_resource_boundary() {
    let cases = [
        (rules::ALPHA, "alpha"),
        (rules::ALPHA_TOKEN, "1"),
        (rules::DIGIT_TOKEN, "7"),
        (rules::ALPHA_TOKEN, "βe\u{301}"),
        (rules::EMOJI_GRAPHEME, "👩\u{200d}💻!"),
        (rules::IDENTIFIER, "ab12_c+tail!"),
        (rules::IDENTIFIER, "abcdefghijklmnop"),
        (rules::IDENTIFIER_PATH_SEGMENT, "hello-world/x"),
        (rules::DIGIT_SEQUENCE, "12_34x"),
        (rules::TEXT, "e\u{301}x"),
        (rules::ANY, "\r\nx"),
        (rules::ESCAPED_CHAR, "\\u{1\u{301}}"),
    ];
    for (rule, text) in cases {
        for grant in (1..=12).chain([4096, u64::MAX]) {
            for fuel in [0, 1, 2, 8, u64::MAX] {
                for max_events in [12, 128] {
                    for open in [false, true] {
                        assert_eq!(
                            trace(rule, text, grant, fuel, max_events, open, false),
                            trace(rule, text, grant, fuel, max_events, open, true),
                            "rule {rule:?}, {text:?}, grant {grant}, fuel {fuel}, events {max_events}, open {open}",
                        );
                    }
                }
            }
        }
    }
}
