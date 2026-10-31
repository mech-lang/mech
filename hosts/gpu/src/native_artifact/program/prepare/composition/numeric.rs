//! Existing numeric/frame operations and finite prices for a parent piece.
use super::{Segment, frames};
use crate::native_artifact::iteration_abi::NativeIterationVisit;
use crate::native_artifact::program::prepare::{
    frame_retirement::FrameRetirements,
    readonly_numeric::{Operation, width},
    tail::{FIXED_OVERHEAD, OVERHEAD},
};
use mech_engine::native_artifact::{
    NativeBufferFrames, NativeIterationAction as Action, NativePreparedLibraryCall,
    NativeProgramArtifactPlan as Plan, NativeProgramFacts, NativeScalarKind,
};

#[derive(Debug, Eq, PartialEq, serde::Serialize)]
pub(in crate::native_artifact) struct View {
    pub source: usize,
    pub width: u8,
    pub witness: usize,
}
#[derive(Debug, Eq, PartialEq, serde::Serialize)]
pub(in crate::native_artifact) struct Layout {
    pub views: Vec<View>,
    pub actions: Vec<(usize, Operation)>,
    pub words: usize,
    pub compute: u64,
    /// Reserved separately; only reached lexical retirement actions spend it.
    pub retirement_work: u64,
}
impl Layout {
    pub(super) fn derive(
        plan: &Plan,
        facts: &NativeProgramFacts<'_>,
        frame_layout: &NativeBufferFrames,
        calls: &[Option<NativePreparedLibraryCall>],
        visits: &[NativeIterationVisit],
        retirements: &FrameRetirements,
        segment: &Segment,
        frames: &mut frames::Layout,
    ) -> Result<Self, &'static str> {
        let mut views = Vec::<View>::new();
        let mut actions = Vec::new();
        let mut compute = FIXED_OVERHEAD;
        for &site in &segment.sites {
            let at = site as usize;
            compute = compute
                .checked_add(
                    visits
                        .get(at)
                        .ok_or("parent price table lacks a site")?
                        .work,
                )
                .and_then(|n| n.checked_add(OVERHEAD))
                .ok_or("parent native work overflows")?;
            let action = if frame_layout.action(at).is_some() {
                // Frame proof above already checked every pack/copy/project.
                Operation::Frame
            } else if let Action::LibraryCall {
                operands,
                output,
                template,
            } = plan.blocks()[at].action()
            {
                let call = calls
                    .get(at)
                    .and_then(Option::as_ref)
                    .ok_or("parent numeric site lacks its selected factory")?;
                if call.template() != template {
                    return Err("parent numeric factory differs from source");
                }
                let length = call.supports_unsigned_matrix_length();
                let read = call.supports_unsigned_matrix_element();
                if !(length || read) || operands.len() != if length { 1 } else { 2 } {
                    return Err("parent piece has an unsupported selected effect");
                }
                let width = width(template.inputs()[0].body())
                    .ok_or("parent numeric input has an unsupported shape or width")?;
                if facts
                    .definition_sites(operands[0].get())?
                    .iter()
                    .filter_map(|d| d.site())
                    .any(|s| segment.sites.contains(&s))
                {
                    return Err("parent native view changes within its piece");
                }
                let source = frames::Source::Value(operands[0].get());
                let source = if let Some(at) = frames.sources.iter().position(|s| *s == source) {
                    at
                } else {
                    frames.sources.push(source);
                    frames.sources.len() - 1
                };
                let array = if let Some(array) = views.iter().position(|v| v.source == source) {
                    if views[array].width != width {
                        return Err("parent numeric input width changes");
                    }
                    array
                } else {
                    views.push(View {
                        source,
                        width,
                        witness: at,
                    });
                    views.len() - 1
                };
                if length {
                    if plan.values()[output.get() as usize].scalar_kind()
                        != Some(NativeScalarKind::U64)
                    {
                        return Err("parent length result has a different scalar kind");
                    }
                    Operation::Length {
                        array,
                        output: output.get(),
                    }
                } else {
                    if plan.values()[operands[1].get() as usize].scalar_kind()
                        != Some(NativeScalarKind::U64)
                    {
                        return Err("parent read selector is not U64");
                    }
                    Operation::Read {
                        array,
                        index: operands[1].get(),
                        output: output.get(),
                    }
                }
            } else {
                continue;
            };
            actions.push((at, action));
        }
        let words = views
            .len()
            .checked_mul(4)
            .and_then(|n| n.checked_add(frames.scalar_words))
            .and_then(|n| n.checked_add(frames.retirements.len()))
            .ok_or("parent native word count overflows")?;
        // Native frame copies, spills and exit synchronization use the existing
        // word adapter. Fund their actual bounded word inventory as well.
        let fields = frames.frames.iter().try_fold(0usize, |n, f| {
            n.checked_add(f.fields.len())
                .ok_or("parent field count overflows")
        })?;
        compute = u64::try_from(fields)
            .ok()
            .and_then(|n| n.checked_mul(128))
            .and_then(|n| n.checked_add(compute))
            .and_then(|n| n.checked_add(u64::try_from(words).ok()?.checked_mul(16)?))
            .ok_or("parent native synchronization price overflows")?;
        // Each source error and bounds edge has a finite spill/reconciliation
        // template. Price even all static copies, a conservative upper bound
        // on every one-entry acyclic execution, without imposing a size cap.
        let boundaries = segment
            .sites
            .len()
            .checked_add(segment.exits.len())
            .and_then(|n| {
                n.checked_add(
                    actions
                        .iter()
                        .filter(|(_, a)| matches!(a, Operation::Read { .. }))
                        .count(),
                )
            })
            .and_then(|n| n.checked_add(1))
            .ok_or("parent boundary count overflows")?;
        let spill = u64::try_from(words)
            .ok()
            .and_then(|n| n.checked_mul(16))
            .and_then(|n| n.checked_add(FIXED_OVERHEAD))
            .and_then(|n| n.checked_mul(u64::try_from(boundaries).ok()?))
            .ok_or("parent native boundary work overflows")?;
        compute = compute
            .checked_add(spill)
            .ok_or("parent total native price overflows")?;
        let mut retirement_work = 0u64;
        for (_, values) in &frames.retirements {
            for &value in values {
                retirement_work = retirement_work
                    .checked_add(
                        retirements
                            .get(value as usize)
                            .map_or(32, |(price, _)| price),
                    )
                    .ok_or("parent retirement price overflows")?;
            }
        }
        Ok(Self {
            views,
            actions,
            words,
            compute,
            retirement_work,
        })
    }
}
