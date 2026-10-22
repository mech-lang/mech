//! Reassemble the same compiler-produced plan, never an alternative lowering.
use super::{collections, iteration, loops, nodes};
use crate::native_artifact::NativeProgramArtifactPlan;
use mech_core::{FunctionCatalog, NodeId, ProgramRevision, SchemaTable};
use serde::Deserialize;
use serde_json::Value as Json;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    revision: ProgramRevision,
    resource_requirements: Vec<Json>,
    observations: Vec<Json>,
    effects: Vec<Json>,
    inputs: Vec<Json>,
    constants: Vec<Json>,
    values: Vec<Json>,
    value_scopes: Vec<Json>,
    blocks: Vec<Json>,
    iterations: Vec<Json>,
    collections: Vec<Json>,
    outputs: Vec<Json>,
    entry: u32,
    activation_nodes: Box<[NodeId]>,
    activation_boundary: Option<u32>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Image {
    format: String,
    version: u32,
    plan: Plan,
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
    let image = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    // SAFETY: the same exact-build confinement obligation applies to the value
    // parsed from these unchanged bytes; parsing supplies no semantic proof.
    unsafe { decode_build_owned_native_plan_value(image, schemas, catalog) }
}

/// Restore the same plan from an enclosing descriptor's already-parsed value,
/// without serializing and parsing its complete metadata a second time.
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
    let wire: Image = serde_json::from_value(image).map_err(|e| e.to_string())?;
    if wire.format != "mech.build-owned-native-plan" || wire.version != 1 {
        return Err("unsupported native plan image format".into());
    }
    let w = wire.plan;
    let resource_requirements = w
        .resource_requirements
        .iter()
        .map(nodes::request)
        .collect::<Result<_, _>>()?;
    let inputs = w
        .inputs
        .iter()
        .map(|v| nodes::input(v, schemas))
        .collect::<Result<_, _>>()?;
    let observations = w
        .observations
        .iter()
        .map(|v| nodes::observation(v, schemas))
        .collect::<Result<_, _>>()?;
    let effects = w
        .effects
        .iter()
        .map(|v| nodes::effect(v, schemas))
        .collect::<Result<_, _>>()?;
    let constants = w
        .constants
        .iter()
        .map(|v| nodes::constant(v, schemas))
        .collect::<Result<_, _>>()?;
    let values = w
        .values
        .iter()
        .map(|v| nodes::program_type(v, schemas))
        .collect::<Result<_, _>>()?;
    let value_scopes = w
        .value_scopes
        .iter()
        .map(loops::scope)
        .collect::<Result<_, _>>()?;
    let outputs = w
        .outputs
        .iter()
        .map(|v| nodes::output(v, schemas))
        .collect::<Result<_, _>>()?;
    // SAFETY: each private reader inherits the enclosing build-owned contract.
    let blocks = w
        .blocks
        .iter()
        .map(|v| unsafe { iteration::block(v, catalog, schemas) })
        .collect::<Result<_, _>>()?;
    // SAFETY: same confined image, schemas and selected registry.
    let iterations = w
        .iterations
        .iter()
        .map(|v| unsafe { loops::control_loop(v, catalog, schemas) })
        .collect::<Result<_, _>>()?;
    // SAFETY: same confined image, schemas and selected registry.
    let collections = w
        .collections
        .iter()
        .map(|v| unsafe { collections::collection(v, catalog, schemas) })
        .collect::<Result<_, _>>()?;
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
