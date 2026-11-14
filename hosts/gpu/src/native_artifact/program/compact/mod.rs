//! Compact native computation derived from the ordinary canonical Plan.
//! Diagnostic-only until the ordinary resource/recovery contract is restored.
//!
//! UTF8 lanes borrow immutable bytes for the entire invocation, including
//! output materialization. Inputs keep their original storage alive; constant
//! pointers relocate into the retained Object. The raw native entry does not
//! accept process addresses from serialized artifacts. Fixed String elements
//! copy span descriptors, never managed String identities or transient owners.
//! Operations that need a new String allocation remain explicit compiler gaps
//! until an invocation-owned native String representation is implemented.
pub(in crate::native_artifact) mod arrays;
mod collections;
mod constants;
mod emit;
mod flow;
mod histogram;
mod immutable_arrays;
mod layout;
mod packed;
mod parallel;
mod phase_storage;
mod phases;
pub(in crate::native_artifact) use phases::{
    NoGrowth, PRIVATE_HANDOFF, PrivatePhase, VectorContract,
};
mod map_index;
mod maps;
mod private_chains;
mod recipes;
mod strings;
mod terminal_appends;
mod utf8_classes;
mod widening_reduction;
mod word_collections;
mod word_packs;

pub(in crate::native_artifact) use emit::define;

use crate::native_artifact::NativeCompileError;
use mech_engine::native_artifact::NativeProgramArtifactPlan;

pub(in crate::native_artifact) fn inventory(
    plan: &NativeProgramArtifactPlan,
) -> Result<serde_json::Value, NativeCompileError> {
    let layout = layout::Layout::derive(plan)?;
    let mut recipe_counts = std::collections::BTreeMap::new();
    for site in 0..plan.blocks().len() {
        let recipe = recipes::derive(plan, &layout, site)?;
        let label = match recipe {
            recipes::Recipe::StringEqual { .. } => "native:StringEqual".into(),
            recipes::Recipe::Aggregate(ref action) => format!("aggregate:{action:?}")
                .split('{')
                .next()
                .unwrap()
                .trim()
                .to_owned(),
            recipes::Recipe::Length { source, output } => {
                assert_eq!(layout.ty(output.get()).words, 1);
                let _ = layout.slot(source.get());
                "length".into()
            }
            recipes::Recipe::Read {
                source,
                index,
                output,
            } => {
                let _ = (
                    layout.slot(source.get()),
                    layout.slot(index.get()),
                    layout.slot(output.get()),
                );
                "read".into()
            }
            recipes::Recipe::Buffer { operation, .. } => format!("native:{operation:?}"),
            recipes::Recipe::Pending(op) => format!("pending:{op:?}"),
            recipes::Recipe::None => "scalar/control".into(),
        };
        *recipe_counts.entry(label).or_insert(0usize) += 1;
    }
    let mut unique = Vec::new();
    for ty in &layout.types {
        if !unique
            .iter()
            .any(|t: &std::sync::Arc<layout::Type>| **t == **ty)
        {
            unique.push(std::sync::Arc::clone(ty))
        }
    }
    let arrays = layout
        .types
        .iter()
        .filter(|t| matches!(t.kind, layout::Kind::Array { .. }))
        .count();
    let owners: usize = layout.types.iter().map(|t| t.owners.len()).sum();
    Ok(
        serde_json::json!({"typed_values":layout.types.len(),"logical_words":layout.words,"unique_layouts":unique.len(),"array_values":arrays,"logical_owner_lanes":owners,"matrix_view_words":layout::ARRAY_WORDS,"layouts":unique.iter().map(|t|t.summary()).collect::<Vec<_>>(),"all_value_bases":layout.base,"recipes":recipe_counts,"scope":"Compile-time logical coordinates only; no source-numbered physical bank or managed owner slots are implied. Pending operations are explicit, never successful runtime stubs."}),
    )
}

mod owner_flow;

mod guards;
