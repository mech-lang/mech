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
/// This compatibility entry cannot recover numeric precision already lost by
/// a `Value` parser; full-width constant images must use the byte entry points.
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
    // Constants may contain U128/I128 at any nesting level. Never route their
    // numeric tokens through serde_json::Value's 64-bit/floating representation.
    let constants = w
        .constants
        .into_iter()
        .map(|v| nodes::constant(v, schemas))
        .collect::<Result<_, _>>()?;
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
        // An image contains semantic selection, not its producer's budget.
        // Shape/ownership proofs and same-Object binding still run on load.
        analysis_byte_limit: u64::MAX,
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

    #[test]
    fn constant_records_preserve_full_width_scalars_and_nested_payloads() {
        use mech_core::{
            IntegerWidth, SchemaBody, SchemaDraft, SchemaId, SchemaTableBuilder, ValueDataDraft,
            ValueDraft,
        };
        let unsigned = SchemaBody::UnsignedInteger(IntegerWidth::W128);
        let signed = SchemaBody::SignedInteger(IntegerWidth::W128);
        for (body, payload) in [
            (unsigned.clone(), ValueDataDraft::U128(u64::MAX as u128 + 1)),
            (unsigned.clone(), ValueDataDraft::U128(u64::MAX as u128 + 2)),
            (unsigned.clone(), ValueDataDraft::U128(u128::MAX)),
            (signed.clone(), ValueDataDraft::I128(i64::MIN as i128 - 1)),
            (signed.clone(), ValueDataDraft::I128(i128::MIN)),
            (signed.clone(), ValueDataDraft::I128(i128::MAX)),
            (
                SchemaBody::Tuple(Box::new([unsigned, signed])),
                ValueDataDraft::Tuple(Box::new([
                    ValueDataDraft::U128(u128::MAX),
                    ValueDataDraft::I128(i128::MIN),
                ])),
            ),
        ] {
            let definition = SchemaDraft {
                dimension_parameters: Box::new([]),
                body,
            }
            .finalize()
            .unwrap();
            let mut builder = SchemaTableBuilder::new();
            builder.insert(definition).unwrap();
            let schemas = builder.finish().unwrap().table;
            #[derive(serde::Serialize)]
            struct Record<'a> {
                value: u32,
                constant: &'a ValueDraft,
            }
            let draft = ValueDraft {
                schema: SchemaId::new(0),
                shape_values: Box::new([]),
                data: payload.clone(),
            };
            let bytes = serde_json::to_vec(&Record {
                value: 0,
                constant: &draft,
            })
            .unwrap();
            // Inert wire fixture: it may be decoded/inspected, never admitted
            // or executed as a synthetic source/native proof.
            let envelope = format!(
                r#"{{"revision":{},"resource_requirements":[],"observations":[],"effects":[],"inputs":[],"constants":[{}],"values":[],"value_scopes":[],"blocks":[],"iterations":[],"collections":[],"outputs":[],"entry":0,"activation_nodes":[],"activation_boundary":null}}"#,
                serde_json::to_string(&[0u8; 32]).unwrap(),
                std::str::from_utf8(&bytes).unwrap()
            );
            let wire: Plan<'_> = serde_json::from_str(&envelope).unwrap();
            let restored = restore(
                wire,
                &schemas,
                &mech_core::FunctionCatalogBuilder::new().build().unwrap(),
            )
            .unwrap();
            assert_eq!(
                restored.constants()[0]
                    .constant()
                    .canonical_data_draft()
                    .unwrap(),
                payload
            );
            let raw = serde_json::to_string(&restored.constants()[0]).unwrap();
            assert_eq!(raw, std::str::from_utf8(&bytes).unwrap());
        }
    }
}
