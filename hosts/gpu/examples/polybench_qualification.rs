//! Canonical PolyBench source qualification and checked-call timing driver.
use mech_core::ReactiveInstanceId;
use mech_core::{
    ManagedMemoryBudget, MemoryBudgetLimits, Value, ValueDataDraft as Data,
    snapshot::{F64Bits, SnapshotValidationContext, ValueDraft},
};
use mech_engine::{
    CanonicalSourceFrontend, ProgramArtifact,
    resident::{ActivationFacts, CapturedValueInput, activate},
};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeExecutionLimits, NativeProgramExecutionLimits,
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use serde_json::{Value as Json, json};
use std::{hint::black_box, time::Instant};
fn err(e: impl std::fmt::Debug) -> String {
    format!("{e:?}")
}
fn numbers(data: &Data, out: &mut Vec<Json>) -> Result<(), String> {
    match data {
        Data::F64(v) => out.push(if v.to_f64().is_finite() {
            json!(v.to_f64())
        } else {
            json!(format!("{}", v.to_f64()))
        }),
        Data::U64(v) => out.push(json!(v)),
        Data::I64(v) => out.push(json!(v)),
        Data::Matrix(v) | Data::Tuple(v) => {
            for x in v.iter() {
                numbers(x, out)?;
            }
        }
        _ => return Err(format!("unsupported output: {data:?}")),
    }
    Ok(())
}
fn inputs(
    artifact: &ProgramArtifact,
    config: &Json,
    memory: &ManagedMemoryBudget,
) -> Result<Vec<Value>, String> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name)
                .ok_or_else(|| format!("invalid source input {}", input.name))?;
            let spec = config
                .get(&name)
                .ok_or_else(|| format!("missing input {name}"))?;
            let schema = artifact.schemas().get(input.schema).unwrap();
            let (data, shape_values) = if let Some(values) = spec.get("values") {
                let data = Data::Matrix(
                    values
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| Data::F64(F64Bits::from_f64(v.as_f64().unwrap())))
                        .collect(),
                );
                let dims: Vec<u64> = spec["shape"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap())
                    .collect();
                let shape = mech_core::shape_for_resolved_extents(schema, &dims)
                    .map_err(err)?
                    .parameter_values()
                    .to_vec()
                    .into_boxed_slice();
                (data, shape)
            } else if spec.get("u64").is_some() {
                (
                    Data::U64(spec["u64"].as_u64().unwrap()),
                    Box::new([]) as Box<[u64]>,
                )
            } else {
                (
                    Data::F64(F64Bits::from_f64(spec.as_f64().unwrap())),
                    Box::new([]) as Box<[u64]>,
                )
            };
            let value = ValueDraft {
                schema: input.schema,
                shape_values,
                data,
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .map_err(err)?;
            let mut reservation = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).map_err(err)?)
                .map_err(err)?;
            value.into_memory_budget(&mut reservation).map_err(err)
        })
        .collect()
}
fn run() -> Result<Json, String> {
    let args: Vec<_> = std::env::args().collect();
    if !(3..=5).contains(&args.len()) {
        return Err(
            "usage: polybench_qualification SOURCE.mec INPUTS.json [SAMPLES] [resident-reference|emit-object=PATH]"
                .into(),
        );
    }
    if args
        .get(4)
        .is_some_and(|mode| mode != "resident-reference" && !mode.starts_with("emit-object="))
    {
        return Err("the optional mode must be resident-reference or emit-object=PATH".into());
    }
    let source = std::fs::read_to_string(&args[1]).map_err(err)?;
    let config: Json =
        serde_json::from_slice(&std::fs::read(&args[2]).map_err(err)?).map_err(err)?;
    let samples: usize = args
        .get(3)
        .map(|x| x.parse())
        .transpose()
        .map_err(err)?
        .unwrap_or(0);
    let catalog_start = Instant::now();
    let catalog = mech_stdlib::source_catalog();
    let catalog_seconds = catalog_start.elapsed().as_secs_f64();
    let start = Instant::now();
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x504f4c59), Revision(1), source.as_str()).map_err(err)?,
        ParseConfig::default(),
    );
    if !parsed.is_strictly_clean() {
        return Err(format!("parse: {:?}", parsed.diagnostics));
    }
    let syntax = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let product = CanonicalSourceFrontend
        .compile_document_with_catalog(&syntax, catalog.clone())
        .map_err(|e| format!("frontend: {e:?}"))?;
    let artifact = product
        .compile_artifact()
        .map_err(|e| format!("artifact: {e:?}"))?;
    let frontend_seconds = start.elapsed().as_secs_f64();
    if let Some(path) = args
        .get(4)
        .and_then(|mode| mode.strip_prefix("emit-object="))
    {
        if path.is_empty() {
            return Err("Object output path is empty".into());
        }
        let object =
            mech_gpu::native_artifact::emit_program_object(&artifact, &catalog).map_err(err)?;
        let path = std::path::Path::new(path);
        std::fs::write(path, object.bytes()).map_err(err)?;
        std::fs::write(
            path.with_extension("metadata.json"),
            serde_json::to_vec_pretty(&object.metadata()).map_err(err)?,
        )
        .map_err(err)?;
        return Ok(
            json!({"status":"ok","mode":"emit-object","path":path,"metadata":path.with_extension("metadata.json")}),
        );
    }
    let memory = ManagedMemoryBudget::new(2 * 1024 * 1024 * 1024);
    let args_values = inputs(&artifact, &config, &memory)?;
    if args.get(4).map(String::as_str) == Some("resident-reference") {
        let mut instance = activate(
            ReactiveInstanceId::new(0x504f4c59, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .map_err(err)?;
        let captured = instance
            .plan
            .inputs
            .iter()
            .map(|input| CapturedValueInput {
                slot: input.slot,
                value: &args_values[artifact
                    .inputs()
                    .iter()
                    .position(|v| v.slot == input.artifact_slot)
                    .unwrap()],
            })
            .collect::<Vec<_>>();
        instance
            .prepare_turn_values(&captured)
            .map_err(err)?
            .publish()
            .map_err(err)?;
        let mut values = Vec::new();
        numbers(
            &instance
                .copied_output(artifact.outputs().len() - 1)
                .map_err(err)?
                .canonical_data_draft()
                .map_err(err)?,
            &mut values,
        )?;
        return Ok(
            json!({"status":"ok","mode":"resident-reference-only","frontend_seconds":frontend_seconds,"values":values}),
        );
    }
    let start = Instant::now();
    let jit = JitProgramArtifact::compile(&artifact, &catalog).map_err(|e| {
        let diagnostic = std::path::Path::new(&args[2]).with_extension("artifact-nodes.txt");
        let _ = std::fs::write(diagnostic, format!("{:#?}", artifact.nodes()));
        format!("native: {e:?}")
    })?;
    let native_seconds = start.elapsed().as_secs_f64();
    let args = args_values;
    let cap = 1_000_000_000_000_000u64;
    let limits = NativeProgramExecutionLimits {
        execution: NativeExecutionLimits {
            max_work: cap,
            max_transport_bytes: 2 * 1024 * 1024 * 1024,
            max_native_stack_bytes: 64 * 1024 * 1024,
        },
        library: MemoryBudgetLimits {
            max_output_elements: Some(100_000_000),
            max_output_bytes: Some(2 * 1024 * 1024 * 1024),
            max_temporary_bytes: Some(2 * 1024 * 1024 * 1024),
            max_cloned_bytes: Some(cap),
            max_retained_nodes: Some(100_000_000),
            max_comparison_work: Some(cap),
            max_compute_work: Some(cap),
            max_scalar_instructions: Some(cap),
            max_transfer_bytes: Some(cap),
            max_storage_buffer_bytes: Some(2 * 1024 * 1024 * 1024),
            max_storage_bindings: Some(100_000),
        },
    };
    let mut output = vec![None; artifact.outputs().len()];
    let report = jit
        .evaluate_into(&args, &mut output, limits, &memory)
        .map_err(|e| format!("execution: {e:?}"))?;
    let mut values = Vec::new();
    numbers(
        &output
            .last()
            .ok_or("no outputs")?
            .as_ref()
            .unwrap()
            .canonical_data_draft()
            .map_err(err)?,
        &mut values,
    )?;
    let mut times = Vec::new();
    for _ in 0..samples {
        output.fill(None);
        let cache = black_box(vec![0f64; 32770 * 1024 / 8]);
        let mut sum = 0.0;
        for value in black_box(cache.as_slice()) {
            sum += *value;
        }
        black_box(sum);
        drop(cache);
        let start = Instant::now();
        black_box(
            jit.evaluate_into(black_box(&args), black_box(&mut output), limits, &memory)
                .map_err(err)?,
        );
        times.push(start.elapsed().as_secs_f64());
    }
    Ok(
        json!({"status":"ok","catalog_seconds":catalog_seconds,"frontend_seconds":frontend_seconds,"native_seconds":native_seconds,"values":values,"samples_seconds":times,"report":format!("{report:?}"),"inputs":artifact.inputs().iter().map(|i|&i.name).collect::<Vec<_>>() }),
    )
}
fn main() {
    let result = match std::panic::catch_unwind(run) {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => json!({"status":"error","error":e}),
        Err(_) => json!({"status":"error","error":"compiler panic"}),
    };
    println!("{result}");
}
