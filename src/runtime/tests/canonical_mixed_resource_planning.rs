//! R25/G28: compute sample schemas come from the compiled region, not a host.
#![cfg(all(
    feature = "full_source",
    feature = "resident-routing-source",
    feature = "compute"
))]

use mech_runtime::RuntimeBuilder;

#[test]
fn canonical_mixed_sample_read_is_planned_without_a_compute_provider() {
    let source = "@compute := compute://worker/kernel{:write(turn), :read(sample/result)}\n@compute/turn <- 1\nanswer := @compute/sample/result\nanswer\n\ncalculation @compute\n-------------------\n~counter := 0f32\ncounter += 1f32\ncounter\n";
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    let mixed = compiler.compile_mixed_source(source).unwrap();
    assert_eq!(mixed.compute.interface.outputs.len(), 1);
    assert_eq!(mixed.compute.interface.outputs[0].name.as_ref(), "result");
    assert!(mixed.compute.interface.outputs[0].dimensions.is_empty());
}

#[test]
fn mixed_compute_projection_inherits_document_kind_declarations() {
    let source = "<amount> := <f32>\n@compute := compute://worker/kernel{:write(turn), :read(sample/value)}\n@compute/turn <- 1\nanswer := @compute/sample/value\nanswer\n\ncalculation @compute\n-------------------\n~value<amount> := 0f32\nvalue += 1f32\nvalue\n";
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    let mixed = compiler
        .compile_mixed_source(source)
        .expect("the compute and initializer projections inherit the document alias");
    assert_eq!(mixed.compute.interface.outputs.len(), 1);
}

#[test]
fn mixed_coordinator_projection_inherits_compute_kind_declarations() {
    let source = "@compute := compute://worker/kernel{:write(turn), :read(sample/value)}\n@compute/turn <- 1\nanswer<amount> := @compute/sample/value\nanswer\n\ncalculation @compute\n-------------------\n<amount> := <f32>\n~value<amount> := 0f32\nvalue += 1f32\nvalue\n";
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    let mixed = compiler
        .compile_mixed_source(source)
        .expect("the coordinator sees the kind declared in the compute section");
    assert_eq!(mixed.compute.interface.outputs.len(), 1);
}

fn document(path: &str, compute: &str) -> String {
    format!(
        "@compute := compute://worker/kernel{{:write(turn), :read({path})}}\n@compute/turn <- 1\nanswer := @compute/{path}\nanswer\n\ncalculation @compute\n-------------------\n~counter := {compute}\ncounter += {compute}\ncounter\n"
    )
}

fn assert_answer_schema(artifact: &mech_engine::ProgramArtifact, expected: &mech_core::SchemaBody) {
    let output = artifact
        .outputs()
        .iter()
        .find(|output| {
            output
                .interactive_binding
                .as_ref()
                .is_some_and(|binding| binding.lexical_name == "answer")
        })
        .expect("coordinator exposes the sampled answer");
    assert_eq!(
        artifact.schemas().get(output.schema).unwrap().body(),
        expected
    );
}

#[test]
fn canonical_mixed_sample_shapes_survive_bytecode() {
    use mech_core::{DimensionExpr, FloatWidth, SchemaBody};
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    for (body, dimensions) in [
        ("1f32", vec![]),
        ("[1f32 2f32 3f32]", vec![1, 3]),
        ("[1f32 2f32 3f32; 4f32 5f32 6f32]", vec![2, 3]),
    ] {
        let mixed = compiler
            .compile_mixed_source(&document("sample/result", body))
            .unwrap();
        assert_eq!(
            mixed.compute.interface.outputs[0].dimensions.as_ref(),
            dimensions.as_slice()
        );
        let expected = if dimensions.is_empty() {
            SchemaBody::FloatingPoint(FloatWidth::W64)
        } else {
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                dimensions: dimensions
                    .into_iter()
                    .map(|n| DimensionExpr::Constant(n as u64))
                    .collect(),
            }
        };
        let artifact = mixed.coordinator.artifact();
        assert_answer_schema(artifact, &expected);
        let encoded = mech_engine::encode_program_artifact_bytecode_v1(artifact).unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
        assert_answer_schema(&decoded, &expected);
    }
}

#[test]
fn canonical_mixed_telemetry_uses_its_declared_types() {
    use mech_core::{FloatWidth, SchemaBody};
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    for path in [
        "backend",
        "last-fault",
        "turns",
        "dispatch-ms",
        "fault-count",
    ] {
        let mixed = compiler
            .compile_mixed_source(&document(path, "1f32"))
            .unwrap();
        let expected = if matches!(path, "backend" | "last-fault") {
            SchemaBody::String
        } else {
            SchemaBody::FloatingPoint(FloatWidth::W64)
        };
        assert_answer_schema(mixed.coordinator.artifact(), &expected);
    }
}

#[test]
fn canonical_mixed_unknown_compute_reads_reject_at_the_interface() {
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    for (path, message) in [
        (
            "sample/missing",
            "source-semantics/unknown-published-binding: document does not define requested output missing",
        ),
        ("unknown", "unknown compute telemetry path `unknown`"),
    ] {
        let error = compiler
            .compile_mixed_source(&document(path, "1f32"))
            .err()
            .expect("invalid interface path must fail");
        assert!(format!("{error:?}").contains(message), "{error:?}");
    }
}

#[test]
fn canonical_mixed_validates_unread_declared_telemetry_paths() {
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    let source = "@compute := compute://worker/kernel{:write(turn), :read(unknown)}\n@compute/turn <- 1\n42\n\ncalculation @compute\n-------------------\n~counter := 0f32\ncounter += 1f32\ncounter\n";
    let error = compiler
        .compile_mixed_source(source)
        .err()
        .expect("an invalid declared telemetry path must fail without a read expression");
    assert!(
        format!("{error:?}").contains("unknown compute telemetry path `unknown`"),
        "{error:?}"
    );
}

#[derive(Debug)]
struct OrdinaryProvider(std::sync::Arc<std::sync::atomic::AtomicUsize>);
impl mech_runtime::RuntimeResourceProvider for OrdinaryProvider {
    fn scheme(&self) -> &str {
        "test"
    }
    fn base_uris(&self) -> Vec<String> {
        vec!["test://clock/value".to_owned()]
    }
    fn semantic_read_contract(&self) -> Option<&'static mech_core::OperationContractDeclaration> {
        Some(mech_runtime::resource_observation_contract())
    }
    fn plan_read(
        &self,
        request: mech_runtime::RuntimeResourceReadRequest,
    ) -> mech_core::MResult<mech_core::Value> {
        assert_eq!(request.base_uri, "test://clock/value");
        assert_eq!(request.path, "sample");
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        mech_core::ValueCell::from_exact(7.0_f64)?.snapshot()
    }
    fn read(
        &self,
        _: mech_runtime::RuntimeResourceReadRequest,
    ) -> mech_core::MResult<mech_core::Value> {
        panic!("compilation must not perform a live provider read")
    }
}

#[test]
fn canonical_mixed_keeps_ordinary_read_planning_in_its_provider() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let plans = Arc::new(AtomicUsize::new(0));
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .resource_provider(Box::new(OrdinaryProvider(plans.clone())))
        .build_compiler()
        .unwrap();
    let source = document("sample/result", "1f32")
        .replace("answer := @compute/sample/result", "@clock := test://clock/value{:read(sample)}\nclock := @clock/sample\nanswer := @compute/sample/result + clock");
    let mixed = compiler.compile_mixed_source(&source).unwrap();
    assert_eq!(plans.load(Ordering::SeqCst), 1);
    assert_answer_schema(
        mixed.coordinator.artifact(),
        &mech_core::SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
    );
}

fn read_resource_bases(artifact: &mech_engine::ProgramArtifact) -> Vec<String> {
    artifact
        .requirements()
        .iter()
        .filter_map(|(_, requirement)| match requirement {
            mech_core::ApplicationRequirement::Resource(request)
                if request.intent == mech_core::ResourceIntent::Read =>
            {
                Some(request.base_uri.clone())
            }
            _ => None,
        })
        .collect()
}

#[test]
fn canonical_mixed_binds_only_called_function_resource_reads() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    let source = |tail: &str| {
        format!(
            "@compute := compute://worker/kernel{{:write(turn), :read(sample/result)}}\n@clock := test://clock/value{{:read(sample)}}\npeek() = out<f64> :=\n  out := (@compute/sample/result).\n\ntick() = out<f64> :=\n  out := (@clock/sample).\n\n@compute/turn <- 1\n{tail}\n\ncalculation @compute\n-------------------\n~counter := 0f32\ncounter += 1f32\ncounter\n"
        )
    };

    let uncalled_plans = Arc::new(AtomicUsize::new(0));
    let mut uncalled_compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .resource_provider(Box::new(OrdinaryProvider(uncalled_plans.clone())))
        .build_compiler()
        .unwrap();
    let uncalled = uncalled_compiler
        .compile_mixed_source(&source("42"))
        .unwrap();
    assert_eq!(uncalled_plans.load(Ordering::SeqCst), 1);
    assert!(read_resource_bases(uncalled.coordinator.artifact()).is_empty());

    let called_plans = Arc::new(AtomicUsize::new(0));
    let mut called_compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .resource_provider(Box::new(OrdinaryProvider(called_plans.clone())))
        .build_compiler()
        .unwrap();
    let called = called_compiler
        .compile_mixed_source(&source("answer := peek() + tick()\nanswer"))
        .unwrap();
    assert_eq!(called_plans.load(Ordering::SeqCst), 1);
    assert_eq!(
        read_resource_bases(called.coordinator.artifact()),
        vec![
            "compute://worker/kernel".to_owned(),
            "test://clock/value".to_owned(),
        ]
    );
    assert_answer_schema(
        called.coordinator.artifact(),
        &mech_core::SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
    );
}

#[test]
fn canonical_mixed_tuple_sample_paths_publish_their_producer() {
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    for path in ["result.0", "result.1.0", "packed.0", "packed.1.0"] {
        let source = format!(
            "@compute := compute://worker/kernel{{:write(turn), :read(sample/{path})}}\n@compute/turn <- 1\nanswer := @compute/sample/{path}\nanswer\n\ncalculation @compute\n-------------------\n~counter := 0f32\ncounter += 1f32\npacked := (counter, (counter + 1f32, counter + 2f32))\npacked\n"
        );
        let mixed = compiler.compile_mixed_source(&source).unwrap();
        let names = |interface: &mech_compute::ComputeRegionInterface| {
            interface
                .outputs
                .iter()
                .map(|port| port.name.to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(&mixed.compute.interface), vec![path]);
        let bytes =
            mech_engine::encode_program_artifact_bytecode_v1(&mixed.compute.artifact).unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
        let decoded_interface = mech_compute::build_compute_region_interface(
            &decoded,
            decoded.compute_regions().first(),
        )
        .unwrap();
        assert_eq!(names(&decoded_interface), vec![path]);
        let bytes =
            mech_engine::encode_program_artifact_bytecode_v1(mixed.coordinator.artifact()).unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
        assert_answer_schema(
            &decoded,
            &mech_core::SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
        );
        assert_eq!(
            mixed.retained_outputs,
            std::collections::BTreeSet::from([path.to_owned()])
        );
        assert_answer_schema(
            mixed.coordinator.artifact(),
            &mech_core::SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
        );
    }
}

#[test]
fn canonical_mixed_retained_tuple_port_is_validated_even_without_a_read() {
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    for path in ["result.9", "result.1.9", "result.0.0", "result.01"] {
        let source = format!(
            "@compute := compute://worker/kernel{{:write(turn), :read(sample/{path})}}\n@compute/turn <- 1\n42\n\ncalculation @compute\n-------------------\n~counter := 0f32\ncounter += 1f32\n(counter, (counter + 1f32, counter + 2f32))\n"
        );
        let error = compiler
            .compile_mixed_source(&source)
            .err()
            .expect("an invalid retained port must fail even without a coordinator read");
        assert!(
            format!("{error:?}").contains(&format!("unknown sampled compute output `{path}`")),
            "{error:?}"
        );
    }
}
