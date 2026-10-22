//! Reassemble the same compiler-produced plan, never an alternative lowering.
use super::{collections, iteration, loops, nodes};
use crate::native_artifact::NativeProgramArtifactPlan;
use mech_core::{FunctionCatalog, NodeId, ProgramRevision, SchemaTable};
use serde::Deserialize;
use serde_json::{Value as Json, value::RawValue};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan<'a> {
    revision: ProgramRevision,
    #[serde(borrow)]
    resource_requirements: Vec<&'a RawValue>,
    #[serde(borrow)]
    observations: Vec<&'a RawValue>,
    #[serde(borrow)]
    effects: Vec<&'a RawValue>,
    #[serde(borrow)]
    inputs: Vec<&'a RawValue>,
    #[serde(borrow)]
    constants: Vec<&'a RawValue>,
    #[serde(borrow)]
    values: Vec<&'a RawValue>,
    #[serde(borrow)]
    value_scopes: Vec<&'a RawValue>,
    #[serde(borrow)]
    blocks: Vec<&'a RawValue>,
    #[serde(borrow)]
    iterations: Vec<&'a RawValue>,
    #[serde(borrow)]
    collections: Vec<&'a RawValue>,
    #[serde(borrow)]
    outputs: Vec<&'a RawValue>,
    entry: u32,
    activation_nodes: Box<[NodeId]>,
    activation_boundary: Option<u32>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Image<'a> {
    format: String,
    version: u32,
    #[serde(borrow)]
    plan: Plan<'a>,
}

/// Restore an unchanged build-produced native plan without source lowering.
/// This does not authorize native entry, resource acquisition, or library calls.
///
/// # Safety
/// Keep the result confined until the caller establishes that `bytes`, `schemas`,
/// and the linked native Object come from the SAME checked compiler build.
/// Verify the build-owned source,
/// bytecode, Object and descriptor pins before linking; preserve that provenance
/// through loading. A caller-provided digest alone is not authentication. The
/// returned plan must not reach execution with arbitrary or changed metadata or
/// code. Runtime admission, current-visit checks, and capability enforcement
/// remain mandatory. This function validates wire consistency, not source proofs.
pub unsafe fn decode_build_owned_native_plan_image(
    bytes: &[u8],
    schemas: &SchemaTable,
    catalog: &FunctionCatalog,
) -> Result<NativeProgramArtifactPlan, String> {
    let image: Image<'_> = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if image.format != "mech.build-owned-native-plan" || image.version != 1 {
        return Err("unsupported native plan image format".into());
    }
    restore(image.plan, schemas, catalog)
}

/// Restore the same plan from a caller's already-parsed value. Linked launchers
/// use the borrowed-body entry point instead, so no complete JSON tree is held
/// alive while the executable descriptor is being constructed.
///
/// # Safety
/// The unchanged-build provenance, Object association, confinement and runtime
/// admission obligations of `decode_build_owned_native_plan_image` all apply.
/// The input must be the unchanged parsed value of that compiler-produced image.
pub unsafe fn decode_build_owned_native_plan_value(
    image: Json,
    schemas: &SchemaTable,
    catalog: &FunctionCatalog,
) -> Result<NativeProgramArtifactPlan, String> {
    let bytes = serde_json::to_vec(&image).map_err(|e| e.to_string())?;
    // SAFETY: serialization preserves the confined, unchanged compiler image.
    unsafe { decode_build_owned_native_plan_image(&bytes, schemas, catalog) }
}

/// Restore the plan body borrowed directly from an enclosing checked image.
/// Only one record's temporary JSON tree is materialized at a time; neither
/// the plan nor the completed descriptor borrows the image buffer.
///
/// # Safety
/// The enclosing descriptor must validate its format and version and preserve
/// all unchanged-build provenance, confinement, Object association and runtime
/// admission obligations of `decode_build_owned_native_plan_image`.
pub unsafe fn decode_build_owned_native_plan_body(
    bytes: &[u8],
    schemas: &SchemaTable,
    catalog: &FunctionCatalog,
) -> Result<NativeProgramArtifactPlan, String> {
    let plan: Plan<'_> = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    restore(plan, schemas, catalog)
}

fn records<T>(
    images: Vec<&RawValue>,
    mut restore: impl FnMut(&Json) -> Result<T, String>,
) -> Result<Box<[T]>, String> {
    images
        .into_iter()
        .map(|image| {
            let value = serde_json::from_str(image.get()).map_err(|e| e.to_string())?;
            restore(&value)
        })
        .collect()
}

fn restore(
    w: Plan<'_>,
    schemas: &SchemaTable,
    catalog: &FunctionCatalog,
) -> Result<NativeProgramArtifactPlan, String> {
    let resource_requirements = records(w.resource_requirements, nodes::request)?;
    let inputs = records(w.inputs, |v| nodes::input(v, schemas))?;
    let observations = records(w.observations, |v| nodes::observation(v, schemas))?;
    let effects = records(w.effects, |v| nodes::effect(v, schemas))?;
    let constants = records(w.constants, |v| nodes::constant(v, schemas))?;
    let values = records(w.values, |v| nodes::program_type(v, schemas))?;
    let value_scopes = records(w.value_scopes, loops::scope)?;
    let outputs = records(w.outputs, |v| nodes::output(v, schemas))?;
    // SAFETY: each private reader inherits the enclosing build-owned contract.
    let blocks = records(w.blocks, |v| unsafe {
        iteration::block(v, catalog, schemas)
    })?;
    // SAFETY: same confined image, schemas and selected registry.
    let iterations = records(w.iterations, |v| unsafe {
        loops::control_loop(v, catalog, schemas)
    })?;
    // SAFETY: same confined image, schemas and selected registry.
    let collections = records(w.collections, |v| unsafe {
        collections::collection(v, catalog, schemas)
    })?;
    let plan = NativeProgramArtifactPlan {
        revision: w.revision,
        resource_requirements,
        observations,
        effects,
        inputs,
        constants,
        values,
        value_scopes,
        blocks,
        iterations,
        collections,
        outputs,
        entry: w.entry,
        activation_nodes: w.activation_nodes,
        activation_boundary: w.activation_boundary,
    };
    // Full encode/decode equality belongs to build-side regression tests, not
    // startup. Re-encoding cannot authenticate a proof copied from this image.
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_wire_borrows_record_bytes_without_expanding_the_complete_tree() {
        // Wire-only data; no native proof or executable is constructed here.
        let revision = [0u8; 32];
        let image = serde_json::json!({
            "revision": revision, "resource_requirements": [], "observations": [],
            "effects": [], "inputs": [], "constants": [], "values": [],
            "value_scopes": [], "blocks": [{"one": [1, 2]}, {"two": [3]}],
            "iterations": [], "collections": [], "outputs": [], "entry": 0,
            "activation_nodes": [], "activation_boundary": null,
        });
        let bytes = serde_json::to_vec(&image).unwrap();
        let plan: Plan<'_> = serde_json::from_slice(&bytes).unwrap();
        let start = bytes.as_ptr() as usize;
        let end = start + bytes.len();
        for block in &plan.blocks {
            let address = block.get().as_ptr() as usize;
            assert!(address >= start && address + block.get().len() <= end);
        }
        let records = records(plan.blocks, |value| Ok(value.clone())).unwrap();
        assert_eq!(records.as_ref(), image["blocks"].as_array().unwrap());

        let mut changed = image;
        changed["unexpected"] = true.into();
        let bytes = serde_json::to_vec(&changed).unwrap();
        assert!(serde_json::from_slice::<Plan<'_>>(&bytes).is_err());
    }
}
