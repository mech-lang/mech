//! Execute existing frame retirements without rediscovering immutable layouts.
use mech_engine::native_artifact::{
    NativeBufferFrameField as Field, NativeBufferFrameLane as Lane, NativeBufferFrames as Layout,
};
use std::ops::Range;

#[derive(Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) enum Operation {
    Scalars(Range<usize>),
    Managed(usize),
}
#[derive(Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Recipe {
    work: u64,
    operations: Range<usize>,
}
/// Immutable compiler-derived data. Independent images rederive this inventory;
/// a genuine bound Object may restore it. Runtime liveness checks still apply.
#[derive(Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct FrameRetirements {
    recipes: Box<[Option<Recipe>]>,
    operations: Box<[Operation]>,
}
impl FrameRetirements {
    pub(in crate::native_artifact) fn derive(
        layout: &Layout,
        values: usize,
    ) -> Result<Self, &'static str> {
        if layout.elements() == 0 {
            return Ok(Self::default());
        }
        let mut recipes = Vec::with_capacity(values);
        let mut operations = Vec::new();
        for value in 0..values {
            let Some(fields) = layout.fields_at(value) else {
                recipes.push(None);
                continue;
            };
            let start = operations.len();
            let mut current = Vec::new();
            let count = flatten(layout, fields, &mut current)?;
            let work = u64::try_from(count)
                .ok()
                .and_then(|n| n.checked_mul(32))
                .and_then(|n| n.checked_add(32))
                .ok_or("frame retirement work overflows")?;
            operations.extend(current);
            recipes.push(Some(Recipe {
                work,
                operations: start..operations.len(),
            }));
        }
        Ok(Self {
            recipes: recipes.into_boxed_slice(),
            operations: operations.into_boxed_slice(),
        })
    }
    #[inline]
    pub(in crate::native_artifact) fn get(&self, value: usize) -> Option<(u64, &[Operation])> {
        let recipe = self.recipes.get(value)?.as_ref()?;
        Some((recipe.work, &self.operations[recipe.operations.clone()]))
    }
}

fn flatten(
    layout: &Layout,
    fields: &[Field],
    operations: &mut Vec<Operation>,
) -> Result<usize, &'static str> {
    let mut count = 0usize;
    for field in fields {
        // Composite nodes count toward the ORIGINAL per-value charge too.
        count = count.checked_add(1).ok_or("frame field count overflows")?;
        match field.lane() {
            Lane::Scalar { at, .. } => {
                let start = at.checked_mul(2).ok_or("frame word index overflows")?;
                let end = start.checked_add(2).ok_or("frame word end overflows")?;
                if at >= layout.scalar_fields() {
                    return Err("frame scalar retirement exceeds bank");
                }
                // Coalesce only physically adjacent consecutive scalar writes.
                // Managed releases stay interleaved in the same DFS order.
                if let Some(Operation::Scalars(previous)) = operations.last_mut()
                    && previous.end == start
                {
                    previous.end = end;
                } else {
                    operations.push(Operation::Scalars(start..end));
                }
            }
            Lane::Managed { at } => {
                if at >= layout.managed_fields() {
                    return Err("frame owner retirement exceeds bank");
                }
                operations.push(Operation::Managed(at));
            }
            Lane::Composite { .. } => {
                let children = layout.children(*field).ok_or("frame child span missing")?;
                count = count
                    .checked_add(flatten(layout, children, operations)?)
                    .ok_or("frame field count overflows")?;
            }
            Lane::Tagged { tag, .. } => {
                // Every variant has a disjoint bank. Inactive owners are
                // normally empty, but retirement clears the complete recipe.
                let start = tag.checked_mul(2).ok_or("frame tag index overflows")?;
                let end = start.checked_add(2).ok_or("frame tag end overflows")?;
                if tag >= layout.scalar_fields() {
                    return Err("frame tag retirement exceeds bank");
                }
                operations.push(Operation::Scalars(start..end));
                let children = layout
                    .children(*field)
                    .ok_or("frame variant span missing")?;
                count = count
                    .checked_add(flatten(layout, children, operations)?)
                    .ok_or("frame variant count overflows")?;
            }
        }
    }
    Ok(count)
}
