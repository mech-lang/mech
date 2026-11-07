//! Reassemble the same compiler-produced plan, never an alternative lowering.
use super::{collections, iteration, library, loops, nodes};
use crate::native_artifact::{
    BuildOwnedImageReader, BuildOwnedImageRecord, NativeProgramArtifactPlan,
};
use mech_core::{FunctionCatalog, NodeId, ProgramRevision, SchemaTable};
use serde::Deserialize;
use serde_json::Value as Json;
#[cfg(test)]
use serde_json::value::RawValue;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan<'a> {
    revision: ProgramRevision,
    #[serde(borrow)]
    resource_requirements: Vec<BuildOwnedImageRecord<'a>>,
    #[serde(borrow)]
    observations: Vec<BuildOwnedImageRecord<'a>>,
    #[serde(borrow)]
    effects: Vec<BuildOwnedImageRecord<'a>>,
    #[serde(borrow)]
    inputs: Vec<BuildOwnedImageRecord<'a>>,
    #[serde(borrow)]
    constants: Vec<BuildOwnedImageRecord<'a>>,
    #[serde(borrow)]
    values: Vec<BuildOwnedImageRecord<'a>>,
    #[serde(borrow)]
    value_scopes: Vec<BuildOwnedImageRecord<'a>>,
    #[serde(borrow)]
    blocks: Vec<BuildOwnedImageRecord<'a>>,
    #[serde(borrow)]
    iterations: Vec<BuildOwnedImageRecord<'a>>,
    #[serde(borrow)]
    collections: Vec<BuildOwnedImageRecord<'a>>,
    #[serde(borrow)]
    outputs: Vec<BuildOwnedImageRecord<'a>>,
    entry: u32,
    activation_nodes: Box<[NodeId]>,
    activation_boundary: Option<u32>,
    #[serde(borrow, default)]
    template_definitions: Option<BuildOwnedImageRecord<'a>>,
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
    let mut reader = BuildOwnedImageReader::from_image(bytes).map_err(|e| e.to_string())?;
    let image: Image<'_> = Image::deserialize(&mut reader).map_err(|e| e.to_string())?;
    reader.end().map_err(|e| e.to_string())?;
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
/// Records stream directly into private typed wire readers. Only nested fields
/// which still require a value-based validator materialize temporary JSON;
/// neither the plan nor the completed descriptor borrows the image buffer.
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
    let mut reader = BuildOwnedImageReader::from_image(bytes).map_err(|e| e.to_string())?;
    let plan: Plan<'_> = Plan::deserialize(&mut reader).map_err(|e| e.to_string())?;
    reader.end().map_err(|e| e.to_string())?;
    restore(plan, schemas, catalog)
}

/// Restore a record borrowed from typed Object metadata or its inspectable JSON.
///
/// # Safety
/// The exact checked-build provenance, confinement and runtime-admission
/// obligations of `decode_build_owned_native_plan_body` all apply.
pub unsafe fn decode_build_owned_native_plan_record(
    image: BuildOwnedImageRecord<'_>,
    schemas: &SchemaTable,
    catalog: &FunctionCatalog,
) -> Result<NativeProgramArtifactPlan, String> {
    let plan = image.decode().map_err(|error| error.to_string())?;
    restore(plan, schemas, catalog)
}

/// Restore compiler-emitted type-reference rows against this image's actual
/// canonical schema table. This representation grants no proof authority.
///
/// # Safety
/// All checked-build provenance, confinement and runtime-admission obligations
/// of `decode_build_owned_native_plan_record` apply. The enclosing image must
/// establish the table-format version before selecting this representation.
pub unsafe fn decode_build_owned_native_table_plan_record(
    image: BuildOwnedImageRecord<'_>,
    schemas: &SchemaTable,
    catalog: &FunctionCatalog,
) -> Result<NativeProgramArtifactPlan, String> {
    let plan = image.decode().map_err(|error| error.to_string())?;
    restore_inner::<true, false>(plan, schemas, catalog)
}

/// Restore shared immutable library definitions and exact per-site headers.
///
/// # Safety
/// All unchanged checked Object/image provenance, confinement and runtime
/// admission duties of `decode_build_owned_native_table_plan_record` apply.
/// Only the enclosing descriptor's explicit shared-table version selects this.
pub unsafe fn decode_build_owned_native_shared_table_plan_record(
    image: BuildOwnedImageRecord<'_>,
    schemas: &SchemaTable,
    catalog: &FunctionCatalog,
) -> Result<NativeProgramArtifactPlan, String> {
    restore_phase(31,true);
    let plan = image.decode().map_err(|error| error.to_string())?;
    restore_phase(31,false);
    restore_inner::<true, true>(plan, schemas, catalog)
}

fn records<T>(
    images: Vec<BuildOwnedImageRecord<'_>>,
    mut restore: impl for<'image> FnMut(&mut BuildOwnedImageReader<'image>) -> Result<T, String>,
) -> Result<Box<[T]>, String> {
    images
        .into_iter()
        .map(|image| {
            let mut reader = image.reader();
            let value = restore(&mut reader)?;
            reader.end().map_err(|error| error.to_string())?;
            Ok(value)
        })
        .collect()
}

fn restore(
    w: Plan<'_>,
    schemas: &SchemaTable,
    catalog: &FunctionCatalog,
) -> Result<NativeProgramArtifactPlan, String> {
    restore_inner::<false, false>(w, schemas, catalog)
}

// Diagnostic-only synchronous observation. The executable supplies the hook;
// this checkout changes no validation, allocation or source/native operation.
unsafe extern "C" { fn mech_restore_phase_event(event:u64); }
fn restore_phase(id:u64,enter:bool) { unsafe { mech_restore_phase_event(if enter{id}else{id|(1u64<<63)}) }; }

fn restore_inner<const TABLES: bool, const SHARED_TEMPLATES: bool>(
    w: Plan<'_>,
    schemas: &SchemaTable,
    catalog: &FunctionCatalog,
) -> Result<NativeProgramArtifactPlan, String> {
    if w.template_definitions.is_some() != SHARED_TEMPLATES {
        return Err("plan image version differs from its template definitions".into());
    }
    // SAFETY: this constructor remains inside the unchanged checked-build
    // loader. The table validates the actual linked factory and canonical
    // schemas for every definition; each source header remains independent.
    restore_phase(32,true);
    let templates = unsafe { library::Templates::new(w.template_definitions, catalog, schemas)? };
    restore_phase(32,false);
    restore_phase(33,true);
    let resource_requirements = records(w.resource_requirements, |reader| nodes::request(reader))?;
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
    restore_phase(33,false);
    restore_phase(34,true);
    let mut types = nodes::TypeTable::new(schemas);
    let values = records(w.values, |v| {
        if TABLES {
            types.read(v)
        } else {
            nodes::program_type(v, schemas)
        }
    })?;
    let value_scopes = records(w.value_scopes, |reader| loops::scope(reader))?;
    let outputs = records(w.outputs, |v| nodes::output(v, schemas))?;
    restore_phase(34,false);
    restore_phase(35,true);
    // SAFETY: each private reader inherits the enclosing build-owned contract.
    let blocks = records(w.blocks, |v| unsafe { iteration::block(v, &templates) })?;
    restore_phase(35,false);
    // SAFETY: same confined image, schemas and selected registry.
    restore_phase(36,true);
    let iterations = records(w.iterations, |v| unsafe {
        loops::control_loop(v, &templates)
    })?;
    restore_phase(36,false);
    // SAFETY: same confined image, schemas and selected registry.
    restore_phase(37,true);
    let collections = records(w.collections, |v| unsafe {
        collections::collection(v, &templates)
    })?;
    restore_phase(37,false);
    restore_phase(38,true);
    let plan = NativeProgramArtifactPlan {
        // An image contains semantic selection, not its producer's budget.
        // Shape/ownership proofs and same-Object binding still run on load.
        definition_table_byte_limit: u64::MAX,
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
    restore_phase(38,false);
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
            let BuildOwnedImageRecord::Json(text) = block else {
                panic!("JSON record expected")
            };
            let address = text.as_ptr() as usize;
            assert!(address >= start && address + text.len() <= end);
        }
        let records = records(plan.blocks, |reader| {
            Json::deserialize(reader).map_err(|error| error.to_string())
        })
        .unwrap();
        assert_eq!(records.as_ref(), image["blocks"].as_array().unwrap());

        let mut changed = image;
        changed["unexpected"] = true.into();
        let bytes = serde_json::to_vec(&changed).unwrap();
        assert!(serde_json::from_slice::<Plan<'_>>(&bytes).is_err());
    }

    #[test]
    fn streamed_records_reject_duplicate_fields_and_invalid_typed_payloads() {
        // These are wire-only scopes, never executable loop proofs. In
        // particular duplicate fields must not be lost in an intermediate map.
        for text in [
            r#"{"Immutable":{"iteration":null}}"#,
            r#"{"Carrier":{"iteration":4294967295}}"#,
        ] {
            let image: Box<RawValue> = serde_json::from_str(text).unwrap();
            let loaded = records(vec![BuildOwnedImageRecord::Json(image.get())], |reader| {
                loops::scope(reader)
            })
            .unwrap();
            assert_eq!(serde_json::to_string(&loaded[0]).unwrap(), text);
        }
        for text in [
            r#"{"Immutable":{"iteration":0,"iteration":1}}"#,
            r#"{"Carrier":{"iteration":4294967296}}"#,
            r#"{"Carrier":{"iteration":-1}}"#,
            r#"{"Carrier":{"iteration":0,"unknown":true}}"#,
            r#"{"Immutable":{"iteration":null},"Carrier":{"iteration":0}}"#,
            "null",
            "[]",
        ] {
            let image: Box<RawValue> = serde_json::from_str(text).unwrap();
            assert!(
                records(vec![BuildOwnedImageRecord::Json(image.get())], |reader| {
                    loops::scope(reader)
                })
                .is_err(),
                "{text}"
            );
        }
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
            let native =
                crate::native_artifact::encode_native_metadata_image(envelope.as_bytes()).unwrap();
            for image in [envelope.as_bytes(), native.as_slice()] {
                let mut reader = BuildOwnedImageReader::from_image(image).unwrap();
                let wire: Plan<'_> = Plan::deserialize(&mut reader).unwrap();
                reader.end().unwrap();
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
}
