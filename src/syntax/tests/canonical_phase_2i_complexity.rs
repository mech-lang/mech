use mech_syntax::document::parser::canonical::parse_canonical_phase_2i_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{DocumentId, ParseConfig, Revision, RuleId, TextSnapshot};

// Fixed allowance for rule setup, terminal lookahead, and the fragment wrapper.
const LINEAR_SLACK: u64 = 512;

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(0x2c4), Revision(0), text).unwrap()
}

fn measurements(
    rule: RuleId,
    inputs: impl IntoIterator<Item = (usize, String)>,
) -> Vec<(usize, u64, u64)> {
    inputs
        .into_iter()
        .map(|(size, input)| {
            let parsed = parse_canonical_phase_2i_rule_for_test(
                source(&input),
                rule,
                ParseConfig::default(),
            )
            .unwrap();
            assert!(
                parsed.is_strictly_clean(),
                "{rule:?}, size {size}: {input:?}"
            );
            assert_eq!(parsed.consumed.end.0 as usize, input.len());
            (size, parsed.stats.parser_steps, parsed.stats.events_emitted)
        })
        .collect()
}

fn assert_linear(values: &[(usize, u64, u64)]) {
    for pair in values.windows(2) {
        let (small_size, small_steps, small_events) = pair[0];
        let (large_size, large_steps, large_events) = pair[1];
        assert_eq!(large_size, small_size * 2);
        assert!(
            large_steps <= small_steps.saturating_mul(2).saturating_add(LINEAR_SLACK),
            "parser-step growth was not linear: {values:?}"
        );
        assert!(
            large_events <= small_events.saturating_mul(2).saturating_add(LINEAR_SLACK),
            "event growth was not linear: {values:?}"
        );
    }
}

fn wrap(depth: usize, open: &str, body: &str, close: &str) -> String {
    format!("{}{}{}", open.repeat(depth), body, close.repeat(depth))
}

#[test]
fn nested_shared_prefix_families_have_measured_linear_growth() {
    let depths = [8_usize, 16, 32, 64];
    for (rule, inputs) in [
        (
            rules::EXPRESSION,
            depths.map(|depth| (depth, wrap(depth, "(", "1", ")"))),
        ),
        (
            rules::EXPRESSION,
            depths.map(|depth| (depth, wrap(depth, "[", "1", "]"))),
        ),
        (
            rules::EXPRESSION,
            depths.map(|depth| (depth, wrap(depth, "{", "1", "}"))),
        ),
        (
            rules::EXPRESSION,
            depths.map(|depth| (depth, wrap(depth, "f(", "1", ")"))),
        ),
        (
            rules::KIND,
            depths.map(|depth| (depth, wrap(depth, "[", "u8", "]"))),
        ),
    ] {
        assert_linear(&measurements(rule, inputs));
    }

    let patterns = depths.map(|depth| {
        let mut pattern = String::from("x");
        for _ in 0..depth {
            pattern = format!("({pattern},x)");
        }
        (depth, pattern)
    });
    assert_linear(&measurements(rules::PATTERN, patterns));

    let record_looking_maps = [4_usize, 8, 16, 32].map(|depth| {
        let mut value = String::from("0");
        for _ in 0..depth {
            value = format!("{{a: {value}, 1: 2}}");
        }
        (depth, value)
    });
    assert_linear(&measurements(rules::EXPRESSION, record_looking_maps));
}

#[test]
fn long_operator_and_list_families_have_measured_linear_growth() {
    let sizes = [32_usize, 64, 128, 256];
    let joined = |size: usize, item: &str, separator: &str| {
        core::iter::repeat_n(item, size)
            .collect::<Vec<_>>()
            .join(separator)
    };
    for (rule, inputs) in [
        (
            rules::EXPRESSION,
            sizes.map(|size| (size, joined(size, "1", " + "))),
        ),
        (
            rules::EXPRESSION,
            sizes.map(|size| (size, joined(size, "true", " && "))),
        ),
        (
            rules::EXPRESSION,
            sizes.map(|size| (size, joined(size, "{1}", " ∪ "))),
        ),
        (
            rules::ARGUMENT_LIST,
            sizes.map(|size| (size, format!("({})", joined(size, "1", ",")))),
        ),
        (
            rules::MATRIX,
            sizes.map(|size| (size, format!("[{}]", joined(size, "1", " ")))),
        ),
        (
            rules::SET,
            sizes.map(|size| (size, format!("{{{}}}", joined(size, "1", ",")))),
        ),
        (
            rules::PATTERN_ARRAY,
            sizes.map(|size| (size, format!("[{}]", joined(size, "x", ",")))),
        ),
        (
            rules::FSM_PIPE,
            sizes.map(|size| (size, format!("#m{}", " -> :x".repeat(size)))),
        ),
    ] {
        assert_linear(&measurements(rule, inputs));
    }
}
