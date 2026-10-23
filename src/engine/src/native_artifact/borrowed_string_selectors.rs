//! Elide only private constant selectors whose complete semantic read set is
//! already handled by the SAME native String region. Never fold a public range.
use super::super::fusion_semantics::Read;
use super::{Action, NativeBorrowedStringAction as View, Plan, Primitive, Scope};

pub(super) fn append(
    plan: &Plan,
    iteration: u32,
    reads: &[Vec<Read>],
    kept: &mut Vec<(u32, View)>,
) {
    let mut selectors = Vec::new();
    for (site, block) in plan.blocks().iter().enumerate() {
        let Action::LibraryCall {
            template,
            operands,
            output,
        } = block.action()
        else {
            continue;
        };
        if block.iteration() != Some(iteration)
            || template.primitive() != Primitive::RangeInclusive
            || operands.len() != 2
            || plan.value_scopes()[output.get() as usize]
                != (Scope::Immutable {
                    iteration: Some(iteration),
                })
        {
            continue;
        }
        let Some((first, last)) = super::fixed_range(plan, *output) else {
            continue;
        };
        let uses = &reads[output.get() as usize];
        if uses.is_empty()
            || !uses.iter().all(|usage| {
                let Read::Action(reader) = usage else {
                    return false;
                };
                let Some((
                    _,
                    View::FixedRange {
                        first: a, last: b, ..
                    },
                )) = kept.iter().find(|(s, _)| s == reader)
                else {
                    return false;
                };
                matches!(plan.blocks()[*reader as usize].action(),
                Action::LibraryCall { template, operands, .. }
                if template.primitive() == Primitive::StringRangeAccess
                    && operands.get(1) == Some(output))
                    && (*a, *b) == (first, last)
            })
        {
            continue;
        }
        // fixed_range proves nonzero ascending U64 bounds and at most32 cells;
        // no arithmetic/range error is suppressed. Bounds of the selected
        // String are still checked at each original String selection site.
        selectors.push((
            site as u32,
            View::ConstantRange {
                first,
                last,
                output: *output,
            },
        ));
    }
    kept.extend(selectors);
    kept.sort_by_key(|(site, _)| *site);
}
