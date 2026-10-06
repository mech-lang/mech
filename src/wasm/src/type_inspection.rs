//! Thin inspection adapter over canonical compilation and resident publication.
//! Numeric payloads are exported as Rust-formatted strings to preserve u128.
use std::collections::BTreeMap;
use std::sync::Arc;

use mech_core::snapshot::SnapshotValidationContext;
use mech_core::{
    CanonicalNominalPath, IntegerInterval, IntegerWidth, ReactiveInstanceId, SchemaBody,
    SchemaDraft, SchemaTableBuilder, Value, ValueDataDraft, ValueDraft,
};
use mech_engine::resident::{ActivationFacts, CapturedValueInput, ReactiveInstance, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_runtime::SourceDocument;
use mech_syntax::document::{ParseConfig, Revision, TextSnapshot, render_source_excerpt};
use serde_json::{Value as Json, json};
use wasm_bindgen::prelude::*;

const INTERVAL_SOURCE: &str = "~state⟨u8:1..10⟩ := 2\nstate = signal\nstate\n";

fn semantic_error(error: mech_engine::SourceSemanticError, source: &TextSnapshot) -> Json {
    let excerpt = render_source_excerpt(source, error.anchor.range, &error.message);
    let severity = if error.code == "source-semantics/empty-document" {
        "info"
    } else {
        "error"
    };
    let report = format!(
        "{}[{}]: {}\n{}",
        if severity == "info" { "Info" } else { "Error" },
        error.code,
        error.message,
        excerpt
    );
    json!({"phase":"semantic_checking", "code":error.code, "message":error.message,
        "document":error.anchor.document.0.to_string(), "revision":error.anchor.revision.0.to_string(),
        "range":[error.anchor.range.start.0,error.anchor.range.end.0], "severity":severity, "presentation":{"report":report,"excerpt":excerpt}})
}
fn scalar_text(value: &Value) -> Option<String> {
    use mech_core::ValueData;
    match value.data() {
        ValueData::U8(value) => Some(value.to_string()),
        ValueData::U64(value) => Some(value.to_string()),
        ValueData::U128(value) => Some(value.to_string()),
        ValueData::F64(value) => Some(value.to_f64().to_string()),
        _ => None,
    }
}
fn describe(value: &Value) -> Json {
    let mut result = json!({"schema":value.schemas().and_then(|schemas|schemas.get(value.schema()).map(|schema|format!("{:?}",schema.body()))),"shape":format!("{:?}",value.shape()),
        "value":format!("{:?}",value.data()),"scalar_text":scalar_text(value),"transport":"exact Rust value text; no JavaScript Number conversion"});
    match mech_runtime::RuntimeValueSnapshot::try_from(value) {
        Ok(snapshot) => {
            let limit = mech_runtime::DEFAULT_REPL_VALUE_ELEMENT_LIMIT;
            result["html"] = json!(snapshot.format_repl_html(limit));
            result["text"] = json!(snapshot.format_repl_inline(limit));
            result["kind"] = json!(snapshot.format_repl_kind());
        }
        Err(error) => result["format_error"] = json!(error.display_message()),
    }
    result
}

/// One editable standalone document inspected through canonical compilation.
/// Reuse this owner for subsequent checks and edits of the same document.
#[wasm_bindgen]
pub struct WasmTypeInspector {
    nominal_origin: CanonicalNominalPath,
}

#[wasm_bindgen]
impl WasmTypeInspector {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            nominal_origin: SourceDocument::new_standalone_origin(),
        }
    }

    /// Parse, check, construct, activate and execute using this document's owner.
    /// Source diagnostics retain anchors, and enum identity survives edits.
    pub fn inspect(&self, source: &str) -> String {
        inspect(source, &self.nominal_origin).to_string()
    }
}

impl Default for WasmTypeInspector {
    fn default() -> Self {
        Self::new()
    }
}

/// Inspect one fresh standalone document. Retain a `WasmTypeInspector` when
/// repeated calls represent edits to the same document.
#[wasm_bindgen(js_name = inspectMechTypes)]
pub fn inspect_mech_types(source: &str) -> String {
    WasmTypeInspector::new().inspect(source)
}
fn inspect(source: &str, nominal_origin: &CanonicalNominalPath) -> Json {
    let mut result = json!({"source":source,"target":if cfg!(target_arch="wasm32"){"WASM-hosted resident CPU"}else{"native resident CPU"},"stages":{},"diagnostics":[]});
    let document = match SourceDocument::parse_resolved(
        "audit:types",
        Revision(0),
        source,
        ParseConfig::default(),
    ) {
        Ok(value) => value.with_nominal_origin(nominal_origin.clone()),
        Err(error) => {
            result["diagnostics"] = json!([{"phase":"parsing","message":format!("{error:?}")}]);
            return result;
        }
    };
    result["stages"]["parsing"] = json!("completed");
    if !document.is_strictly_clean() {
        result["stages"]["semantic_checking"] = json!("blocked by strict syntax validation");
        result["diagnostics"] =
            serde_json::to_value(document.snapshot().diagnostics.as_slice()).unwrap_or(json!([]));
        return result;
    }
    let catalog = mech_stdlib::source_catalog();
    let program = match document
        .canonical_frontend()
        .compile_document_with_catalog(&document.document(), Arc::clone(&catalog))
    {
        Ok(value) => value,
        Err(error) => {
            result["stages"]["semantic_checking"] = json!("rejected");
            result["diagnostics"] = json!([semantic_error(error, &document.snapshot().source)]);
            // Exercise the product handoff as well as direct frontend inspection.
            result["product_diagnostic"] = match mech_runtime::RuntimeBuilder::new()
                .function_catalog(Arc::clone(&catalog))
                .build_compiler()
                .and_then(|mut compiler| compiler.compile_document(&document))
            {
                Err(error) => json!({
                    "semantic":error.kind_as::<mech_engine::SourceSemanticError>().cloned().map(|error| semantic_error(error, &document.snapshot().source)),
                    "presentation_range":error.primary_range().map(|range|format!("{range:?}")),
                    "message":error.display_message(),
                }),
                Ok(_) => {
                    json!({"unexpected":"product compiler accepted a rejected semantic candidate"})
                }
            };
            return result;
        }
    };
    result["stages"]["semantic_checking"] = json!("completed");
    result["inputs"]=json!(program.program().inputs.iter().map(|input|json!({"name":input.name,"schema":format!("{:?}",program.schemas().get(input.schema))})).collect::<Vec<_>>());
    result["outputs"] = json!(
        program
            .program()
            .outputs
            .iter()
            .map(|output| format!("{:?}", program.schemas().get(output.schema)))
            .collect::<Vec<_>>()
    );
    let artifact = match program.compile_artifact() {
        Ok(value) => value,
        Err(error) => {
            result["stages"]["artifact_construction"] = json!("rejected");
            result["diagnostics"] =
                json!([{"phase":"artifact_construction","message":format!("{error:?}")}]);
            return result;
        }
    };
    result["stages"]["artifact_construction"] = json!("completed");
    let encoded = match mech_engine::encode_program_artifact_bytecode_v1(&artifact) {
        Ok(value) => value,
        Err(error) => {
            result["diagnostics"] =
                json!([{"phase":"artifact_transport","message":format!("{error:?}")}]);
            return result;
        }
    };
    result["artifact_bytes"] = json!(encoded.len());
    let decoded = match mech_engine::decode_program_artifact_bytecode_v1(&encoded) {
        Ok(value) => value,
        Err(error) => {
            result["diagnostics"] =
                json!([{"phase":"artifact_transport","message":format!("{error:?}")}]);
            return result;
        }
    };
    result["artifact_transport"] = json!("bytecode-v1 encoded and decoded before activation");
    let mut instance = match activate(
        ReactiveInstanceId::new(0xA04, 0),
        &decoded,
        &catalog,
        &ActivationFacts::default(),
    ) {
        Ok(value) => value,
        Err(error) => {
            result["stages"]["activation"] = json!("rejected");
            result["diagnostics"] = json!([{"phase":"activation","message":format!("{error:?}")}]);
            return result;
        }
    };
    result["stages"]["activation"] = json!("completed");
    if !instance.plan.inputs.is_empty() {
        result["stages"]["execution"] = json!("awaiting external inputs");
        return result;
    }
    match instance.turn(&[]) {
        Ok(_) => {
            result["stages"]["execution"] = json!("completed");
            result["stages"]["correct_publication"] =
                json!("published; independent expected-value comparison required");
            result["values"] = json!(
                (0..decoded.outputs().len())
                    .map(|index| match instance.copied_output(index) {
                        Ok(value) => describe(&value),
                        Err(error) => json!({"error":format!("{error:?}")}),
                    })
                    .collect::<Vec<_>>()
            );
            result["epoch"] = json!(format!("{:?}", instance.published_epoch()));
        }
        Err(error) => {
            result["stages"]["execution"] = json!("rejected");
            result["diagnostics"] = json!([{"phase":"execution","message":format!("{error:?}")}]);
        }
    }
    result
}

/// One activated interval-state instance. Rejected inputs never replace it.
#[wasm_bindgen]
pub struct TypePublicationSession {
    artifact: ProgramArtifact,
    instance: ReactiveInstance,
}
#[wasm_bindgen]
impl TypePublicationSession {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<TypePublicationSession, JsValue> {
        Self::create().map_err(|error| JsValue::from_str(&error))
    }
    pub fn source(&self) -> String {
        INTERVAL_SOURCE.to_owned()
    }
    pub fn snapshot(&self) -> String {
        self.snapshot_value().to_string()
    }
    pub fn update(&mut self, decimal: &str) -> String {
        self.update_value(decimal).to_string()
    }
}
impl TypePublicationSession {
    fn create() -> Result<Self, String> {
        let document = SourceDocument::parse_resolved(
            "audit:interval-session",
            Revision(0),
            INTERVAL_SOURCE,
            ParseConfig::default(),
        )
        .map_err(|e| format!("{e:?}"))?;
        let catalog = mech_stdlib::source_catalog();
        let body = SchemaBody::IntegerInterval(IntegerInterval::Unsigned {
            width: IntegerWidth::W8,
            lower: 1,
            upper: 10,
            upper_inclusive: false,
        });
        let artifact = CanonicalSourceFrontend
            .compile_document_with_catalog_and_input_schemas(
                &document.document(),
                Arc::clone(&catalog),
                BTreeMap::from([("signal".to_owned(), body)]),
            )
            .map_err(|e| format!("{e:?}"))?
            .compile_artifact()
            .map_err(|e| format!("{e:?}"))?;
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact)
            .map_err(|e| format!("{e:?}"))?;
        let artifact = mech_engine::decode_program_artifact_bytecode_v1(&bytes)
            .map_err(|e| format!("{e:?}"))?;
        let instance = activate(
            ReactiveInstanceId::new(0xA04, 1),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .map_err(|e| format!("{e:?}"))?;
        let mut session = Self { artifact, instance };
        let initial = session.update_value("2");
        if initial["outcome"] != "accepted" {
            return Err(initial.to_string());
        }
        Ok(session)
    }
    fn snapshot_value(&self) -> Json {
        json!({"source":INTERVAL_SOURCE,"instance":"0xa04:1 (this object; no replacement on update)",
            "epoch":format!("{:?}",self.instance.published_epoch()),"state_hash":self.instance.published_state_hash().to_string(),
            "output":match self.instance.copied_output(0){Ok(value)=>describe(&value),Err(error)=>json!({"error":format!("{error:?}")})}})
    }
    fn update_value(&mut self, decimal: &str) -> Json {
        let before = self.snapshot_value();
        let update = (|| -> Result<(), (String, String)> {
            let byte = decimal
                .parse::<u8>()
                .map_err(|e| ("external_input_admission".into(), e.to_string()))?;
            let mut builder = SchemaTableBuilder::new();
            let schema = SchemaDraft {
                dimension_parameters: Box::new([]),
                body: SchemaBody::UnsignedInteger(IntegerWidth::W8),
            }
            .finalize()
            .map_err(|e| ("external_input_admission".into(), format!("{e:?}")))?;
            let handle = builder
                .insert(schema)
                .map_err(|e| ("external_input_admission".into(), format!("{e:?}")))?;
            let built = builder
                .finish()
                .map_err(|e| ("external_input_admission".into(), format!("{e:?}")))?;
            let schema = built
                .resolve(handle)
                .map_err(|e| ("external_input_admission".into(), format!("{e:?}")))?;
            let (schemas, _) = built.into_parts();
            let base = ValueDraft {
                schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::U8(byte),
            }
            .finalize(&SnapshotValidationContext::new(&schemas))
            .map_err(|e| ("external_input_admission".into(), format!("{e:?}")))?;
            let input = &self.instance.plan.inputs[0];
            let admitted = base
                .rebind(input.schema, &input.shape, self.artifact.schemas())
                .map_err(|e| ("external_input_admission".into(), format!("{e:?}")))?;
            self.instance
                .prepare_turn_values(&[CapturedValueInput {
                    slot: input.slot,
                    value: &admitted,
                }])
                .and_then(|prepared| prepared.publish())
                .map_err(|e| ("execution_or_publication".into(), format!("{e:?}")))?;
            Ok(())
        })();
        let after = self.snapshot_value();
        match update {
            Ok(()) => {
                json!({"input":decimal,"outcome":"accepted","phase":"correct_publication","before":before,"after":after})
            }
            Err((phase, message)) => {
                json!({"input":decimal,"outcome":"rejected","phase":phase,"message":message,"accepted_state_unchanged":before==after,"before":before,"after":after})
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inspect(source: &str) -> Json {
        serde_json::from_str(&inspect_mech_types(source)).unwrap()
    }

    #[cfg(all(feature = "enum", feature = "kind_define", feature = "variable_define"))]
    mod standalone_enums {
        use super::*;

        const SOURCE: &str = "<color> := :red | :green | :blue\nmy-color<color> := :red\n";

        fn inspect_with(inspector: &WasmTypeInspector, source: &str) -> Json {
            serde_json::from_str(&inspector.inspect(source)).unwrap()
        }

        #[test]
        fn exact_enum_source_publishes_red_and_edits_keep_its_identity() {
            let inspector = WasmTypeInspector::new();
            let first = inspect_with(&inspector, SOURCE);
            assert_eq!(first["stages"]["execution"], "completed", "{first}");
            assert_eq!(first["diagnostics"], json!([]), "{first}");
            assert_eq!(
                first["values"][0]["value"], "Enum(EnumValue { ordinal: 0, payload: None })",
                "{first}"
            );
            let schema = first["values"][0]["schema"].as_str().unwrap();
            assert!(schema.contains("name: \"red\""), "{schema}");
            let path = CanonicalNominalPath::new(
                inspector
                    .nominal_origin
                    .segments()
                    .iter()
                    .cloned()
                    .chain(std::iter::once("color".to_owned()))
                    .collect::<Vec<_>>(),
            )
            .unwrap();
            let expected = mech_core::NominalKey::from_path(mech_core::NominalKind::Enum, &path);
            assert!(schema.contains(&format!("{expected:?}")), "{schema}");

            for source in [
                SOURCE.replace("my-color<color> := :red", "my-color<color> := :green"),
                SOURCE.to_owned(),
            ] {
                let next = inspect_with(&inspector, &source);
                assert_eq!(next["stages"]["execution"], "completed", "{next}");
                assert_eq!(next["values"][0]["schema"], first["values"][0]["schema"]);
            }
        }

        #[test]
        fn independent_inspectors_and_one_shot_calls_have_distinct_enum_owners() {
            let first = inspect_with(&WasmTypeInspector::new(), SOURCE);
            let independent = inspect_with(&WasmTypeInspector::new(), SOURCE);
            let one_shot = inspect(SOURCE);
            let next_one_shot = inspect(SOURCE);
            for result in [&first, &independent, &one_shot, &next_one_shot] {
                assert_eq!(result["stages"]["execution"], "completed", "{result}");
            }
            assert_ne!(
                first["values"][0]["schema"],
                independent["values"][0]["schema"]
            );
            assert_ne!(
                one_shot["values"][0]["schema"],
                next_one_shot["values"][0]["schema"]
            );
        }

        #[test]
        fn invalid_enum_variant_is_rejected_without_changing_the_owner() {
            let inspector = WasmTypeInspector::new();
            let first = inspect_with(&inspector, SOURCE);
            let rejected = inspect_with(
                &inspector,
                &SOURCE.replace("my-color<color> := :red", "my-color<color> := :yellow"),
            );
            assert_eq!(
                rejected["stages"]["semantic_checking"], "rejected",
                "{rejected}"
            );
            assert_eq!(
                rejected["diagnostics"][0]["code"],
                "source-semantics/unknown-enum-variant"
            );
            assert_eq!(
                rejected["product_diagnostic"]["semantic"]["code"],
                rejected["diagnostics"][0]["code"]
            );
            assert_eq!(
                rejected["product_diagnostic"]["semantic"]["range"],
                rejected["diagnostics"][0]["range"]
            );
            assert!(rejected.get("values").is_none());
            let restored = inspect_with(&inspector, SOURCE);
            assert_eq!(restored["stages"]["execution"], "completed", "{restored}");
            assert_eq!(
                restored["values"][0]["schema"],
                first["values"][0]["schema"]
            );
        }
    }

    #[test]
    fn same_instance_rejects_and_recovers() {
        let mut session = TypePublicationSession::create().unwrap();
        assert_eq!(session.update_value("9")["outcome"], "accepted");
        let rejected = session.update_value("10");
        assert_eq!(rejected["outcome"], "rejected");
        assert_eq!(rejected["accepted_state_unchanged"], true);
        assert_eq!(session.update_value("3")["outcome"], "accepted");
    }
    #[cfg(feature = "u128")]
    #[test]
    fn fixtures_compare_independent_scalar_values() {
        let mut records = Vec::new();
        for (source, expected) in [
            ("answer := 1⟨u8:1..10⟩\nanswer\n", "1"),
            ("answer := 9⟨u8:1..10⟩\nanswer\n", "9"),
            ("answer := 10⟨u8:1..=10⟩\nanswer\n", "10"),
            ("answer := math/sub(right: 3, left: 10)\nanswer\n", "7"),
            (
                "answer := 340282366920938463463374607431768211455u128\nanswer\n",
                "340282366920938463463374607431768211455",
            ),
        ] {
            let result = inspect(source);
            assert!(
                result["values"].as_array().is_some_and(|values| values
                    .iter()
                    .any(|value| value["scalar_text"] == expected)),
                "{result}"
            );
            records.push(result);
        }
        println!("NATIVE_TYPE_FIXTURES={}", json!(records));
    }

    #[test]
    fn semantic_interval_rejection_preserves_source_anchor() {
        let result = inspect("10⟨u8:1..10⟩\n");
        assert_eq!(
            result["diagnostics"][0]["code"],
            "source-semantics/integer-interval-violation"
        );
        assert!(result["diagnostics"][0]["range"].is_array());
        assert_eq!(
            result["product_diagnostic"]["semantic"]["range"],
            result["diagnostics"][0]["range"]
        );
        assert!(result["product_diagnostic"]["presentation_range"].is_string());
    }

    #[test]
    fn inspection_exports_canonical_record_html() {
        let result = inspect("point := {x: 1, y: 2}\npoint\n");
        assert_eq!(result["stages"]["execution"], "completed", "{result}");
        let value = &result["values"][0];
        let html = value["html"].as_str().expect("canonical HTML projection");
        assert!(html.contains("<table class='mech-record'>"), "{html}");
        assert!(html.contains("<th scope='row'>x</th>"), "{html}");
        assert!(html.contains("<th scope='row'>y</th>"), "{html}");
        assert!(value["text"].is_string());
        assert_eq!(value["kind"], "record");
        assert!(value["value"].is_string());
    }
}
