//! Same-plan structural leaves for numeric frames. Leaf ordinals are private
//! proof keys, distinct from the source's top-level projection ordinals.
use super::*;
use mech_engine::native_artifact::{NativeBufferFrameField as Field, NativeMatchValue as Value};

pub(super) type Key = (u32, Option<usize>);

pub(super) fn leaves(frames: &NativeBufferFrames, fields: &[Field]) -> Option<Vec<Field>> {
    fn visit(frames: &NativeBufferFrames, fields: &[Field], out: &mut Vec<Field>) -> Option<()> {
        for field in fields {
            match field.lane() {
                Lane::Scalar { .. } | Lane::Managed { .. } => out.push(*field),
                Lane::Composite { .. } => visit(frames, frames.children(*field)?, out)?,
                Lane::Tagged { .. } => return None,
            }
        }
        Some(())
    }
    let mut output = Vec::new();
    visit(frames, fields, &mut output)?;
    Some(output)
}

pub(super) fn at(frames: &NativeBufferFrames, value: u32) -> Option<Vec<Field>> {
    leaves(frames, frames.fields_at(value as usize)?)
}

pub(super) fn span(
    frames: &NativeBufferFrames,
    value: u32,
    ordinal: usize,
) -> Option<std::ops::Range<usize>> {
    let fields = frames.fields_at(value as usize)?;
    let field = *fields.get(ordinal)?;
    let start = leaves(frames, &fields[..ordinal])?.len();
    let count = leaves(frames, &[field])?.len();
    Some(start..start.checked_add(count)?)
}

pub(super) fn operand(frames: &NativeBufferFrames, value: Value, relative: usize) -> Option<Key> {
    if frames.fields(value).is_some() {
        (relative < at(frames, value.get())?.len()).then_some((value.get(), Some(relative)))
    } else {
        (relative == 0).then_some((value.get(), None))
    }
}

/// Exact managed-leaf copies for one original structural action. Reused by
/// provenance's current-version and bottom-up child-contract proofs.
pub(super) fn transfers(
    plan: &Plan,
    frames: &NativeBufferFrames,
    site: usize,
) -> Result<Vec<(Key, Key)>, &'static str> {
    let mut copies = Vec::new();
    match frames
        .action(site)
        .ok_or("numeric frame lacks recipe")?
        .operation()
    {
        Frame::Pack { output } => {
            let Action::LibraryCall { operands, .. } = plan.blocks()[site].action() else {
                return Err("numeric pack lacks operands");
            };
            let fields = at(frames, output.get()).ok_or("numeric pack lacks leaves")?;
            for (ordinal, value) in operands.iter().enumerate() {
                let range = span(frames, output.get(), ordinal).ok_or("numeric pack loses span")?;
                for (relative, index) in range.enumerate() {
                    if matches!(fields[index].lane(), Lane::Managed { .. }) {
                        copies.push((
                            (output.get(), Some(index)),
                            operand(frames, *value, relative)
                                .ok_or("numeric pack loses managed source")?,
                        ));
                    }
                }
            }
        }
        Frame::Copy { source, output } => {
            for (index, field) in at(frames, source.get())
                .ok_or("numeric copy loses leaves")?
                .iter()
                .enumerate()
            {
                if matches!(field.lane(), Lane::Managed { .. }) {
                    copies.push(((output.get(), Some(index)), (source.get(), Some(index))));
                }
            }
        }
        Frame::Project {
            source,
            ordinal,
            output,
        } => {
            let fields = at(frames, source.get()).ok_or("numeric projection loses leaves")?;
            let range =
                span(frames, source.get(), *ordinal).ok_or("numeric projection loses span")?;
            for (relative, index) in range.enumerate() {
                if matches!(fields[index].lane(), Lane::Managed { .. }) {
                    copies.push((
                        operand(frames, *output, relative)
                            .ok_or("numeric projection loses output")?,
                        (source.get(), Some(index)),
                    ));
                }
            }
        }
        _ => return Err("numeric leaf copies cannot contain tagged frames"),
    }
    Ok(copies)
}

pub(super) fn maximum_arity(frames: &NativeBufferFrames, fields: &[Field]) -> usize {
    fields
        .iter()
        .filter_map(|field| frames.children(*field))
        .map(|children| maximum_arity(frames, children))
        .fold(fields.len(), usize::max)
}

/// Managed leaves contribute their independently measured retained footprint.
/// Count every scalar and structural node around them once as well.
pub(super) fn logical_nodes(frames: &NativeBufferFrames, fields: &[Field]) -> usize {
    fields
        .iter()
        .map(|field| match field.lane() {
            Lane::Scalar { .. } => 1,
            Lane::Composite { .. } => 1 + logical_nodes(frames, frames.children(*field).unwrap()),
            Lane::Managed { .. } => 0,
            Lane::Tagged { .. } => unreachable!("numeric leaf preparation rejects tags"),
        })
        .sum()
}

pub(super) fn has_nested_managed(frames: &NativeBufferFrames, fields: &[Field]) -> bool {
    fields.iter().any(|field| {
        frames.children(*field).is_some_and(|children| {
            leaves(frames, children).is_some_and(|leaves| {
                leaves
                    .iter()
                    .any(|leaf| matches!(leaf.lane(), Lane::Managed { .. }))
            })
        })
    })
}
