//! Bounded predictions derived from the same base and operator specifications
//! that drive the resumable parser. Summaries retain the original logical work,
//! fuel and peak requirements while eliding failed transactional execution.
//! Input outside the admitted ASCII window follows the ordinary continuation.
use super::super::Parser;
use super::super::rule::rules;
use super::operators::spec::{Spec, specification};
use super::operators::{EXPRESSION_OPERATOR_RULES, OperatorAtom, OperatorGuard};
use super::{base, statements};
use crate::document::RuleId;
use crate::document::TextSize;
use crate::document::parser::grapheme_scan::{GraphemeScan, ScanProgress};
const OPERATOR_COUNT: usize = EXPRESSION_OPERATOR_RULES.len();
const MIN_ANALYSIS_ALLOWANCE: u64 = 4096;
const MAX_WHITESPACE_PREFIX: u64 = 128;
const ASCII_HEADS: u128 = ((1_u128 << 127) - 1) ^ ((1_u128 << 32) - 1);
#[derive(Clone, Copy)]
pub(super) struct Plan {
    mask: u128,
    work: u64,
    rewinds: u64,
    marker_depth: usize,
    rule_depth: usize,
    whitespace_visits: u64,
    literal_scans: u64,
}
impl Plan {
    fn none() -> Self {
        Self {
            mask: 0,
            work: 0,
            rewinds: 0,
            marker_depth: 0,
            rule_depth: 0,
            whitespace_visits: 0,
            literal_scans: 0,
        }
    }
    fn sequence(
        parts: &[Self],
        own_work: u64,
        own_rewinds: u64,
        own_markers: usize,
        own_depth: usize,
    ) -> Self {
        Self {
            mask: parts
                .iter()
                .fold(ASCII_HEADS, |mask, part| mask & part.mask),
            work: own_work + parts.iter().map(|part| part.work).sum::<u64>(),
            rewinds: own_rewinds + parts.iter().map(|part| part.rewinds).sum::<u64>(),
            marker_depth: own_markers
                + parts
                    .iter()
                    .map(|part| part.marker_depth)
                    .max()
                    .unwrap_or(0),
            rule_depth: own_depth + parts.iter().map(|part| part.rule_depth).max().unwrap_or(0),
            whitespace_visits: parts.iter().map(|part| part.whitespace_visits).sum(),
            literal_scans: parts.iter().map(|part| part.literal_scans).sum(),
        }
    }
}

pub(super) struct OperatorCache {
    plans: [Option<Plan>; OPERATOR_COUNT * 2],
    tokens: Option<Option<[(u64, u64); 2]>>,
}
impl OperatorCache {
    fn new() -> Self {
        Self {
            plans: [None; OPERATOR_COUNT * 2],
            tokens: None,
        }
    }
}
fn token_costs() -> Option<[(u64, u64); 2]> {
    Some([whitespace_token(b' ')?, whitespace_token(b'\t')?])
}
pub(super) fn predict_operator(p: &mut Parser<'_>, rule: RuleId, allowance: u64) -> Option<u64> {
    #[cfg(test)]
    if p.state.disable_rejections_for_test {
        return None;
    }
    if allowance < MIN_ANALYSIS_ALLOWANCE || p.is_halted() {
        return None;
    }
    let at = p.offset();
    let first = p.cursor().byte()?;
    let has_prefix = matches!(first, b' ' | b'\t');
    if !has_prefix && !(b'!'..=b'~').contains(&first) {
        return None;
    }
    if at.0 > 0 && !p.source.byte_at(TextSize(at.0 - 1))?.is_ascii() {
        return None;
    }
    let index = EXPRESSION_OPERATOR_RULES
        .iter()
        .position(|candidate| *candidate == rule)?
        + OPERATOR_COUNT * usize::from(has_prefix);
    let cache = p
        .state
        .operator_storage
        .prediction
        .get_or_insert_with(|| alloc::boxed::Box::new(OperatorCache::new()));
    let plan = *cache.plans[index].get_or_insert_with(|| operator(rule, 0, has_prefix));
    let costs = if has_prefix {
        *cache.tokens.get_or_insert_with(token_costs)
    } else {
        Some([(0, 0); 2])
    };
    if plan.mask == 0 {
        return None;
    }
    let mut head = at;
    let mut spaces = 0_u64;
    let mut tabs = 0_u64;
    if has_prefix {
        let min_token_work = costs?.iter().map(|(work, _)| *work).min()?;
        let max = allowance
            .checked_div(min_token_work)?
            .min(MAX_WHITESPACE_PREFIX);
        loop {
            if head >= p.cursor().end() {
                return None;
            }
            match p.source.byte_at(head)? {
                b' ' => spaces += 1,
                b'\t' => tabs += 1,
                _ => break,
            }
            if spaces + tabs > max {
                return None;
            }
            head = TextSize(head.0.checked_add(1)?);
        }
    }
    let next = TextSize(head.0.checked_add(1)?);
    if next >= p.cursor().context_end() || next > p.cursor().end() {
        return None;
    }
    let byte = p.source.byte_at(head)?;
    if !(b'!'..=b'~').contains(&byte) || !p.source.byte_at(next)?.is_ascii() {
        return None;
    }
    if plan.mask & (1_u128 << byte) == 0 {
        return None;
    }
    let tokens = spaces + tabs;
    let [(space_work, space_rewinds), (tab_work, tab_rewinds)] = costs?;
    let work = plan.work + plan.whitespace_visits * (spaces * space_work + tabs * tab_work);
    let charges = plan.whitespace_visits * tokens;
    if work > allowance || charges > p.state.fuel {
        return None;
    }
    // The ordinary path may have emitted temporary whitespace tokens and
    // markers before its rejection. Preserve its fuel charges and ensure every
    // reserved marker finish and resource envelope would have fit.
    let reserve = p
        .state
        .events
        .len()
        .checked_add(p.state.open_markers.len())?
        .checked_add(tokens as usize)?
        .checked_add(plan.marker_depth.checked_mul(2)?)?
        .checked_add(3)?;
    if reserve > p.state.config.limits.max_events as usize {
        return None;
    }
    p.state
        .rules
        .note_peak(p.state.rules.len() + plan.rule_depth);
    p.state.peak_events = p
        .state
        .peak_events
        .max(p.state.events.len() + tokens as usize + plan.marker_depth);
    p.state.peak_open_markers = p
        .state
        .peak_open_markers
        .max(p.state.open_markers.len() + plan.marker_depth);
    p.state.rewinds +=
        plan.rewinds + plan.whitespace_visits * (spaces * space_rewinds + tabs * tab_rewinds);
    p.state.rewound_bytes += charges;
    p.state.fuel -= charges;
    p.state.stats.parser_steps = p.state.stats.parser_steps.saturating_add(charges);
    Some(work)
}
// Include failed earlier alternatives and the successful one-byte token. The
// repeat/choice and rule phases are the same for every admitted prefix byte.
// A changed spelling that is not one byte declines this prediction.
fn whitespace_token(byte: u8) -> Option<(u64, u64)> {
    let mut work = 4;
    let mut rewinds = 0;
    let base::Spec::Choice(choices) = base::specification(rules::SPACE_TAB)? else {
        return None;
    };
    for rule in choices {
        let spec = super::terminal_spec::fixed_terminal_spec(*rule)?;
        if spec.spacing != super::terminal_spec::TerminalSpacing::Exact {
            return None;
        }
        work += literal(spec.literal).work + 5;
        if spec.literal.as_bytes()[0] == byte {
            return (spec.literal.len() == 1).then_some((work, rewinds));
        }
        rewinds += 1;
    }
    None
}

// The admitted source window has a one-byte ASCII grapheme with an ASCII
// successor. Its fresh scan costs two calls. Expected segmentation remains in
// the pinned Unicode scanner; a differing first byte proves failure without a
// token, including literals whose first grapheme occupies several bytes.
fn literal(text: &'static str) -> Plan {
    let end = TextSize::checked_from_usize(text.len()).unwrap();
    let mut scan = GraphemeScan::new(TextSize::ZERO, Some(end));
    let mut allowance = u64::MAX;
    let range = match scan.advance(text, end, true, &mut allowance) {
        ScanProgress::Grapheme(range) => range,
        _ => return Plan::none(),
    };
    let first = text.as_bytes()[0];
    Plan {
        mask: if first.is_ascii() {
            ASCII_HEADS & !(1_u128 << first)
        } else {
            ASCII_HEADS
        },
        work: u64::MAX - allowance + 3 + u64::from(range.len().0 == 1),
        rewinds: 0,
        marker_depth: 0,
        rule_depth: 0,
        whitespace_visits: 0,
        literal_scans: 1,
    }
}
fn fixed(rule: RuleId) -> Plan {
    let Some(spec) = super::terminal_spec::fixed_terminal_spec(rule) else {
        return Plan::none();
    };
    let scan = literal(spec.literal);
    match spec.spacing {
        super::terminal_spec::TerminalSpacing::Exact => Plan::sequence(&[scan], 4, 1, 0, 1),
        super::terminal_spec::TerminalSpacing::Whitespace0Both => {
            Plan::sequence(&[whitespace(rules::WHITESPACE0), scan], 6, 1, 0, 1)
        }
    }
}

fn whitespace(rule: RuleId) -> Plan {
    base_plan(rule)
}

fn operator(rule: RuleId, nesting: usize, has_prefix: bool) -> Plan {
    if nesting >= EXPRESSION_OPERATOR_RULES.len() {
        return Plan::none();
    }
    match specification(rule) {
        Spec::Transpose if !has_prefix => Plan::sequence(&[fixed(rules::APOSTROPHE)], 3, 1, 0, 1),
        Spec::SpacedSubtract if !has_prefix => {
            Plan::sequence(&[whitespace(rules::WS1E)], 4, 1, 1, 1)
        }
        Spec::Aggregate { alternatives, .. } => {
            let mut out = Plan {
                mask: ASCII_HEADS,
                work: 2,
                rewinds: 1,
                marker_depth: 1,
                rule_depth: 1,
                whitespace_visits: 0,
                literal_scans: 0,
            };
            for rule in alternatives {
                let child = operator(*rule, nesting + 1, has_prefix);
                out.mask &= child.mask;
                out.work += child.work + 1;
                out.rewinds += child.rewinds;
                out.literal_scans += child.literal_scans;
                out.whitespace_visits += child.whitespace_visits;
                out.marker_depth = out.marker_depth.max(child.marker_depth + 1);
                out.rule_depth = out.rule_depth.max(child.rule_depth + 1);
            }
            out
        }
        Spec::Leaf {
            alternatives,
            guard,
            spacing,
            ..
        } => {
            if has_prefix && spacing.is_none() {
                return Plan::none();
            }
            let mut out = Plan {
                mask: ASCII_HEADS,
                work: 3,
                rewinds: 1,
                marker_depth: 1,
                rule_depth: 1,
                whitespace_visits: 0,
                literal_scans: 0,
            };
            if let Some(rule) = spacing {
                let child = whitespace(if has_prefix { rules::WS0E } else { rule });
                out.whitespace_visits = u64::from(has_prefix);
                out.mask &= child.mask;
                out.work += child.work + 1;
                out.rewinds += child.rewinds;
                out.literal_scans += child.literal_scans;
                out.rule_depth = out.rule_depth.max(child.rule_depth + 1);
                if !has_prefix && matches!(rule, rules::WS1E | rules::SPACE_TAB1) {
                    return out;
                }
                if !matches!(
                    rule,
                    rules::WS0E | rules::SPACE_TAB0 | rules::WS1E | rules::SPACE_TAB1
                ) {
                    return Plan::none();
                }
            }
            let (child, extra) = match guard {
                OperatorGuard::None => (None, 2),
                OperatorGuard::NotCommentSigil => {
                    let literal_scans = statements::COMMENT_SIGIL_LITERALS.map(literal);
                    (Some(Plan::sequence(&literal_scans, 4, 1, 0, 1)), 3)
                }
                OperatorGuard::NotMatrixMultiply => (
                    Some(operator(rules::MATRIX_MULTIPLY, nesting + 1, false)),
                    4,
                ),
                OperatorGuard::NotXor => (Some(operator(rules::XOR, nesting + 1, false)), 4),
                OperatorGuard::NotGeneratorArrow => (Some(fixed(rules::GENERATOR_ARROW)), 3),
            };
            out.work += extra;
            if let Some(child) = child {
                out.mask &= child.mask;
                out.work += child.work;
                out.rewinds += child.rewinds + 1;
                out.literal_scans += child.literal_scans;
                out.marker_depth = out.marker_depth.max(child.marker_depth + 1);
                out.rule_depth = out.rule_depth.max(child.rule_depth + 1);
            }
            out.work += 1;
            for atom in alternatives {
                let child = match atom {
                    OperatorAtom::CanonicalRule(rule) => fixed(*rule),
                    OperatorAtom::Text(text) => Plan::sequence(&[literal(text)], 3, 1, 0, 1),
                };
                out.mask &= child.mask;
                out.work += child.work + 2;
                out.rewinds += child.rewinds;
                out.literal_scans += child.literal_scans;
                out.rule_depth = out.rule_depth.max(child.rule_depth + 1);
            }
            out
        }
        _ => Plan::none(),
    }
}
pub(super) fn predict_base(
    p: &mut Parser<'_>,
    rule: RuleId,
    index: usize,
    allowance: u64,
) -> Option<(bool, u64)> {
    #[cfg(test)]
    if p.state.disable_rejections_for_test {
        return None;
    }
    if allowance < MIN_ANALYSIS_ALLOWANCE || p.is_halted() {
        return None;
    }
    let byte = p.cursor().byte()?;
    if !(b' '..=b'~').contains(&byte) {
        return None;
    }
    let cache = p
        .state
        .base_storage
        .prediction
        .get_or_insert_with(|| alloc::boxed::Box::new([None; base::NON_FIXED_RULES.len()]));
    let plan = *cache[index].get_or_insert_with(|| base_plan(rule));
    if plan.mask & (1_u128 << byte) == 0 || plan.work > allowance {
        return None;
    }
    if plan.literal_scans > 0 {
        let at = p.offset();
        let next = TextSize(at.0.checked_add(1)?);
        if next >= p.cursor().context_end()
            || next > p.cursor().end()
            || !p.source.byte_at(next)?.is_ascii()
        {
            return None;
        }
        if at.0 > 0 && !p.source.byte_at(TextSize(at.0 - 1))?.is_ascii() {
            return None;
        }
    }
    let reserve = p
        .state
        .events
        .len()
        .checked_add(p.state.open_markers.len())?
        .checked_add(plan.marker_depth.checked_mul(2)?)?
        .checked_add(3)?;
    if reserve > p.state.config.limits.max_events as usize {
        return None;
    }
    p.state
        .rules
        .note_peak(p.state.rules.len() + plan.rule_depth);
    p.state.peak_events = p
        .state
        .peak_events
        .max(p.state.events.len() + plan.marker_depth);
    p.state.peak_open_markers = p
        .state
        .peak_open_markers
        .max(p.state.open_markers.len() + plan.marker_depth);
    p.state.rewinds += plan.rewinds;
    Some((
        matches!(
            base::specification(rule),
            Some(base::Spec::Repeat {
                required: false,
                ..
            })
        ),
        plan.work,
    ))
}
fn classified(classify: fn(char) -> bool) -> Plan {
    let mask = (b' '..=b'~')
        .filter(|byte| !classify(*byte as char))
        .fold(0, |mask, byte| mask | (1_u128 << byte));
    Plan {
        mask,
        work: 3,
        rewinds: 1,
        marker_depth: 0,
        rule_depth: 1,
        whitespace_visits: 0,
        literal_scans: 0,
    }
}
fn alternatives(
    rules: &[RuleId],
    work: u64,
    rewinds: u64,
    marker_depth: usize,
    rule_depth: usize,
) -> Plan {
    let mut out = Plan {
        mask: ASCII_HEADS,
        work,
        rewinds,
        marker_depth,
        rule_depth,
        whitespace_visits: 0,
        literal_scans: 0,
    };
    for rule in rules {
        let child = base_plan(*rule);
        out.mask &= child.mask;
        out.work += child.work + 1;
        out.rewinds += child.rewinds;
        out.marker_depth = out.marker_depth.max(marker_depth + child.marker_depth);
        out.rule_depth = out.rule_depth.max(rule_depth + child.rule_depth);
        out.literal_scans += child.literal_scans;
    }
    out
}
fn base_plan(rule: RuleId) -> Plan {
    if super::terminal_spec::fixed_terminal_spec(rule).is_some() {
        return fixed(rule);
    }
    let Some(spec) = base::specification(rule) else {
        return Plan::none();
    };
    match spec {
        base::Spec::Alias(child) => Plan::sequence(&[base_plan(child)], 2, 1, 0, 1),
        base::Spec::Classified(_, classify) => classified(classify),
        base::Spec::Choice(choices) => alternatives(choices, 2, 1, 0, 1),
        base::Spec::Repeat { required, choices } => Plan::sequence(
            &[alternatives(choices, 0, 0, 0, 0)],
            3,
            u64::from(required),
            0,
            1,
        ),
        base::Spec::RepeatedNode {
            first,
            single_first,
            ..
        } => {
            if single_first {
                Plan::sequence(&[base_plan(first[0])], 4, 1, 1, 1)
            } else {
                alternatives(first, 4, 1, 1, 1)
            }
        }
        base::Spec::Unless { excluded, rule } => Plan::sequence(
            &[alternatives(excluded, 0, 0, 0, 0), base_plan(rule)],
            3,
            2,
            0,
            1,
        ),
        base::Spec::Never => Plan {
            mask: ASCII_HEADS,
            work: 2,
            rewinds: 1,
            marker_depth: 0,
            rule_depth: 1,
            whitespace_visits: 0,
            literal_scans: 0,
        },
        _ => Plan::none(),
    }
}

#[cfg(test)]
#[path = "prediction_tests.rs"]
mod tests;
