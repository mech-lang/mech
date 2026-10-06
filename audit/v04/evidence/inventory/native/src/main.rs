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
use std::collections::BTreeMap;
use std::sync::Arc;
const SOURCE: &str = include_str!("../../inventory.mec");
fn scalar(n: i64) -> Value {
    let mut builder = SchemaTableBuilder::new();
    let handle = builder
        .insert(
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body: SchemaBody::SignedInteger(IntegerWidth::W64),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let built = builder.finish().unwrap();
    ValueDraft {
        schema: built.resolve(handle).unwrap(),
        shape_values: Box::new([]),
        data: ValueDataDraft::I64(n),
    }
    .finalize(&SnapshotValidationContext::new(&built.table))
    .unwrap()
}
fn snapshot(instance: &ReactiveInstance) -> Json {
    json!({"epoch":format!("{:?}",instance.published_epoch()),"state_hash":instance.published_state_hash().to_string(),"output":instance.copied_output(0).map(|v|format!("{:?}",v.canonical_data_draft())).unwrap_or_else(|e|format!("{e:?}"))})
}
fn update(
    instance: &mut ReactiveInstance,
    artifact: &ProgramArtifact,
    names: &[String],
    arrivals: i64,
    demand: i64,
) -> Result<(), String> {
    let values = names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let n = match name.as_str() {
                "arrivals" => arrivals,
                "demand" => demand,
                other => panic!("unexpected input {other}"),
            };
            let input = &instance.plan.inputs[i];
            scalar(n)
                .rebind(input.schema, &input.shape, artifact.schemas())
                .map_err(|e| format!("input admission: {e:?}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let inputs = values
        .iter()
        .zip(&instance.plan.inputs)
        .map(|(value, input)| CapturedValueInput {
            slot: input.slot,
            value,
        })
        .collect::<Vec<_>>();
    instance
        .prepare_turn_values(&inputs)
        .and_then(|prepared| prepared.publish())
        .map_err(|e| format!("execution/publication: {e:?}"))?;
    Ok(())
}
fn main() {
    let document = SourceDocument::parse_resolved(
        "audit:inventory",
        Revision(0),
        SOURCE,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(
        document.is_strictly_clean(),
        "{:?}",
        document.snapshot().diagnostics
    );
    let catalog = mech_stdlib::source_catalog();
    let input_schema = SchemaBody::SignedInteger(IntegerWidth::W64);
    let program = CanonicalSourceFrontend
        .compile_document_with_catalog_and_input_schemas(
            &document.document(),
            Arc::clone(&catalog),
            BTreeMap::from([
                ("arrivals".into(), input_schema.clone()),
                ("demand".into(), input_schema),
            ]),
        )
        .unwrap();
    let names = program
        .program()
        .inputs
        .iter()
        .map(|p| p.name.clone())
        .collect::<Vec<_>>();
    println!("INPUTS={names:?}");
    let artifact = program.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    if let Some(path) = std::env::args().nth(1) {
        std::fs::write(path, &bytes).unwrap();
    }
    let artifact = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0xA041, 1),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    // Seed publication exercises the same generic turn path with a zero delta.
    update(&mut instance, &artifact, &names, 0, 0).unwrap();
    assert_eq!(
        instance
            .copied_output(0)
            .unwrap()
            .canonical_data_draft()
            .unwrap(),
        ValueDataDraft::I64(100)
    );
    let mut accepted = 100i64;
    let mut records = Vec::new();
    for (arrivals, demand) in [
        (20, 15),
        (0, 110),
        (10, 25),
        (0, 90),
        (5, 0),
        (-1, 0),
        (0, -1),
        (1000000, 0),
        (10, 3),
    ] {
        let before = snapshot(&instance);
        let candidate = accepted + arrivals - demand;
        let outcome = update(&mut instance, &artifact, &names, arrivals, demand);
        let after = snapshot(&instance);
        if arrivals >= 0 && demand >= 0 && (0..=1000000).contains(&candidate) {
            assert!(outcome.is_ok(), "{outcome:?}");
            assert_eq!(
                instance
                    .copied_output(0)
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                ValueDataDraft::I64(candidate)
            );
            accepted = candidate;
        } else {
            assert!(outcome.is_err());
            assert_eq!(before, after);
        }
        records.push(json!({"arrivals":arrivals,"demand":demand,"independent_candidate":candidate,"accepted_stock":accepted,"outcome":if outcome.is_ok(){"accepted"}else{"rejected"},"error":outcome.err(),"before":before,"after":after}));
    }
    println!(
        "INVENTORY_RESULT={}",
        json!({"source":SOURCE,"bytecode_bytes":bytes.len(),"target":"native resident CPU","source_compile":"passed","bytecode_roundtrip":"passed","same_instance":true,"records":records})
    );
}
