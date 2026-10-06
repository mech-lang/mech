//! Named signed-scalar transport over the public resident source APIs.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use mech_core::snapshot::SnapshotValidationContext;
use mech_core::{
    IntegerWidth, ReactiveInstanceId, SchemaBody, SchemaDraft, SchemaTableBuilder, Value,
    ValueDataDraft, ValueDraft,
};
use mech_engine::resident::{ActivationFacts, CapturedValueInput, ReactiveInstance, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_runtime::SourceDocument;
use mech_syntax::document::{ParseConfig, Revision};
use serde_json::{Value as Json, json};
use wasm_bindgen::prelude::*;

const INVENTORY_SOURCE: &str = include_str!("fixtures/inventory.mec");

/// The retained source fixture compiled by the inventory browser demonstration.
#[wasm_bindgen(js_name = inventorySource)]
pub fn inventory_source() -> String {
    INVENTORY_SOURCE.to_owned()
}

/// One resident instance whose named external inputs are signed i64 scalars.
/// Source supplies computation and constraints. Updates supply decimal text.
#[wasm_bindgen]
pub struct I64PublicationSession {
    source: String,
    artifact: ProgramArtifact,
    instance: ReactiveInstance,
    names: Vec<String>,
    bytecode_bytes: usize,
}

#[wasm_bindgen]
impl I64PublicationSession {
    #[wasm_bindgen(constructor)]
    pub fn new(source: &str, input_names_json: &str) -> Result<I64PublicationSession, JsValue> {
        Self::create(source, input_names_json).map_err(|error| JsValue::from_str(&error))
    }
    pub fn source(&self) -> String {
        self.source.clone()
    }
    pub fn snapshot(&self) -> String {
        self.snapshot_value().to_string()
    }
    pub fn update(&mut self, inputs_json: &str) -> String {
        self.update_value(inputs_json).to_string()
    }
}

fn scalar(value: i64) -> Result<Value, String> {
    let mut builder = SchemaTableBuilder::new();
    let handle = builder
        .insert(
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body: SchemaBody::SignedInteger(IntegerWidth::W64),
            }
            .finalize()
            .map_err(|e| format!("{e:?}"))?,
        )
        .map_err(|e| format!("{e:?}"))?;
    let built = builder.finish().map_err(|e| format!("{e:?}"))?;
    ValueDraft {
        schema: built.resolve(handle).map_err(|e| format!("{e:?}"))?,
        shape_values: Box::new([]),
        data: ValueDataDraft::I64(value),
    }
    .finalize(&SnapshotValidationContext::new(&built.table))
    .map_err(|e| format!("{e:?}"))
}

impl I64PublicationSession {
    fn create(source: &str, input_names_json: &str) -> Result<Self, String> {
        let requested: Vec<String> =
            serde_json::from_str(input_names_json).map_err(|e| e.to_string())?;
        let unique = requested.iter().collect::<BTreeSet<_>>();
        if unique.len() != requested.len() {
            return Err("Duplicate input names".into());
        }
        let document = SourceDocument::parse_resolved(
            "audit:i64-publication",
            Revision(0),
            source,
            ParseConfig::default(),
        )
        .map_err(|e| format!("{e:?}"))?;
        if !document.is_strictly_clean() {
            return Err(format!(
                "Strict syntax validation: {:?}",
                document.snapshot().diagnostics
            ));
        }
        let catalog = mech_stdlib::source_catalog();
        let input_schemas = requested
            .iter()
            .map(|name| (name.clone(), SchemaBody::SignedInteger(IntegerWidth::W64)))
            .collect::<BTreeMap<_, _>>();
        let program = CanonicalSourceFrontend
            .compile_document_with_catalog_and_input_schemas(
                &document.document(),
                Arc::clone(&catalog),
                input_schemas,
            )
            .map_err(|e| format!("{e:?}"))?;
        let names = program
            .program()
            .inputs
            .iter()
            .map(|input| input.name.clone())
            .collect::<Vec<_>>();
        if names.iter().collect::<BTreeSet<_>>() != unique {
            return Err("Declared input names differ from compiled external inputs".into());
        }
        let artifact = program.compile_artifact().map_err(|e| format!("{e:?}"))?;
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact)
            .map_err(|e| format!("{e:?}"))?;
        let artifact = mech_engine::decode_program_artifact_bytecode_v1(&bytes)
            .map_err(|e| format!("{e:?}"))?;
        let instance = activate(
            ReactiveInstanceId::new(0xA041, 1),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .map_err(|e| format!("{e:?}"))?;
        Ok(Self {
            source: source.into(),
            artifact,
            instance,
            names,
            bytecode_bytes: bytes.len(),
        })
    }
    fn snapshot_value(&self) -> Json {
        let outputs = (0..self.artifact.outputs().len()).map(|index| match self.instance.copied_output(index) {
            Ok(value) => json!({"schema":value.schemas().and_then(|schemas| schemas.get(value.schema()).map(|schema| format!("{:?}", schema.body()))), "shape":format!("{:?}",value.shape()), "scalar_text":match value.canonical_data_draft() { Ok(ValueDataDraft::I64(n))=>Some(n.to_string()), _=>None }, "value":format!("{:?}",value.canonical_data_draft())}),
            Err(error) => json!({"error":format!("{error:?}")}),
        }).collect::<Vec<_>>();
        json!({"source":self.source,"inputs":self.names,"instance_scope":"this session object", "epoch":format!("{:?}",self.instance.published_epoch()),"state_hash":self.instance.published_state_hash().to_string(),"outputs":outputs,"bytecode_bytes":self.bytecode_bytes,"artifact_transport":"bytecode-v1 encoded and decoded before activation"})
    }
    fn update_value(&mut self, inputs_json: &str) -> Json {
        let before = self.snapshot_value();
        let update = (|| -> Result<(), (&str, String)> {
            let inputs: BTreeMap<String, String> = serde_json::from_str(inputs_json)
                .map_err(|e| ("external_input_admission", e.to_string()))?;
            if inputs.keys().collect::<BTreeSet<_>>() != self.names.iter().collect::<BTreeSet<_>>()
            {
                return Err((
                    "external_input_admission",
                    "Input names differ from this session's compiled ports".into(),
                ));
            }
            let values = self
                .names
                .iter()
                .zip(&self.instance.plan.inputs)
                .map(|(name, port)| {
                    let n = inputs[name]
                        .parse::<i64>()
                        .map_err(|e| ("external_input_admission", format!("{name}: {e}")))?;
                    scalar(n)
                        .and_then(|value| {
                            value
                                .rebind(port.schema, &port.shape, self.artifact.schemas())
                                .map_err(|e| format!("{e:?}"))
                        })
                        .map_err(|e| ("external_input_admission", e))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let captured = values
                .iter()
                .zip(&self.instance.plan.inputs)
                .map(|(value, port)| CapturedValueInput {
                    slot: port.slot,
                    value,
                })
                .collect::<Vec<_>>();
            self.instance
                .prepare_turn_values(&captured)
                .and_then(|prepared| prepared.publish())
                .map_err(|e| ("execution_or_publication", format!("{e:?}")))?;
            Ok(())
        })();
        let after = self.snapshot_value();
        match update {
            Ok(()) => {
                json!({"input_json":inputs_json,"outcome":"accepted","phase":"publication","before":before,"after":after})
            }
            Err((phase, message)) => {
                json!({"input_json":inputs_json,"outcome":"rejected","phase":phase,"message":message,"accepted_state_unchanged":before==after,"before":before,"after":after})
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_inventory_obeys_source_constraints_and_recovers() {
        let mut session =
            I64PublicationSession::create(INVENTORY_SOURCE, r#"["arrivals","demand"]"#).unwrap();
        for (arrivals, demand, accepted, expected) in [
            (0, 0, true, 100),
            (20, 15, true, 105),
            (0, 110, false, 105),
            (10, 25, true, 90),
            (0, 90, true, 0),
            (5, 0, true, 5),
            (-1, 0, false, 5),
            (0, -1, false, 5),
            (1000000, 0, false, 5),
            (10, 3, true, 12),
        ] {
            let result = session.update_value(
                &json!({"arrivals":arrivals.to_string(),"demand":demand.to_string()}).to_string(),
            );
            assert_eq!(
                result["outcome"],
                if accepted { "accepted" } else { "rejected" },
                "{result}"
            );
            assert_eq!(
                result["after"]["outputs"][0]["scalar_text"],
                expected.to_string()
            );
            if !accepted {
                assert_eq!(result["accepted_state_unchanged"], true);
            }
        }
    }
    #[test]
    fn source_and_port_names_determine_computation() {
        let mut session = I64PublicationSession::create(
            "~total⟨i64⟩ := 7\ntotal = total + change\ntotal\n",
            r#"["change"]"#,
        )
        .unwrap();
        assert_eq!(
            session.update_value(r#"{"change":"3"}"#)["after"]["outputs"][0]["scalar_text"],
            "10"
        );
        for invalid in [r#"{"other":"2"}"#, r#"{"change":"9223372036854775808"}"#] {
            assert_eq!(
                session.update_value(invalid)["accepted_state_unchanged"],
                true
            );
        }
        assert_eq!(
            session.update_value(r#"{"change":"-4"}"#)["after"]["outputs"][0]["scalar_text"],
            "6"
        );
    }
}
