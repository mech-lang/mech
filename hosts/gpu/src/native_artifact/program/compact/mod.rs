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
mod array_growth;
pub(in crate::native_artifact) mod arrays;
mod backing_owners;
mod body_relocations;
mod checked_region_discovery;
mod checked_traversals;
mod cold_status;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod cold_tail_tests;
mod collections;
mod compile_jobs;
mod constants;
#[cfg(feature = "early-region-planning")]
mod early_regions;
#[cfg(feature = "early-region-emission")]
mod early_units;
mod emit;
mod flow;
mod histogram;
mod immutable_arrays;
mod indexed_stores;
mod inlining;
mod layout;
mod le_scalar_stores;
mod packed;
mod parallel;
#[cfg(feature = "early-region-planning")]
mod partition_plan;
mod phase_bodies;
mod phase_layout;
mod phase_storage;
mod phases;
mod vector_guard_facts;
#[cfg(feature = "early-region-emission")]
mod word_boundary;
pub(in crate::native_artifact) use phases::{
    NoGrowth, PRIVATE_HANDOFF, PrivatePhase, VectorContract,
};
mod allocation_stubs;
mod map_index;
mod maps;
mod object;
mod owner_counts;
mod owner_retirement;
mod private_chains;
mod recipes;
mod release_stubs;
mod strings;
mod terminal_appends;
mod utf8_classes;
mod widening_reduction;
mod word_collections;
mod word_packs;
mod word_pairs;

pub(in crate::native_artifact) use emit::define;
pub(super) use emit::define_with_isa;
pub(super) use object::PreparedObject;

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
        .filter(|t| matches!(t.kind, layout::LayoutKind::Array { .. }))
        .count();
    let owners: usize = layout.types.iter().map(|t| t.owners.len()).sum();
    Ok(
        serde_json::json!({"typed_values":layout.types.len(),"logical_words":layout.words,"unique_layouts":unique.len(),"array_values":arrays,"logical_owner_lanes":owners,"matrix_view_words":layout::ARRAY_WORDS,"layouts":unique.iter().map(|t|t.summary()).collect::<Vec<_>>(),"all_value_bases":layout.base,"recipes":recipe_counts,"scope":"Compile-time logical coordinates only; no source-numbered physical bank or managed owner slots are implied. Pending operations are explicit, never successful runtime stubs."}),
    )
}

mod owner_flow;

mod guards;

mod overwrite_reuse;
