use std::collections::BTreeMap;

use mech_compute::{
    BackendRequest, ComputeDispatchRequest, ComputeElementType, ComputeInitializerSet,
    ComputeOutputSelection, ComputePlatform, ComputeValue, TensorLayout,
};
use mech_core::{
    Body, ComputePlacement, MechCode, ParsedProgram, Program, ResolvedOperationContract, Section,
    SectionElement, ValueData,
};
use mech_engine::{SlotRole, decode_program_artifact_sections, encode_program_artifact_sections};
use mech_gpu::{
    ComputeHostFactory, ComputeLowerer, ElementwiseKernel, ExecutionTarget, GpuBindingRole,
    GpuDiagnosticCode, GpuExecutionBindingRole, GpuExecutionPlan, GpuKernelPlanSource,
    GpuPlanKernelKind, SlotResidence, TransferDirection, native_compute_backend_registry,
};
use mech_runtime::{
    ConfigValue, PreparedRuntimeEffect, ProgramCompiler, RuntimeBuilder,
    RuntimeCapabilityOperation, RuntimeHostFactory, RuntimeHostInputValue,
    RuntimeResourceReadRequest, RuntimeResourceWriteIntent, RuntimeResourceWriteRequest,
};
use std::num::NonZeroU32;

const PARTICLE_SOURCE: &str = r#"
~positions := host-positions
~velocities := host-velocities
origin := host-origin
attraction := host-attraction
drag := host-drag
dt := host-dt
acceleration := (origin - positions) * attraction
next-velocities := (velocities + acceleration * dt) * drag
next-positions := positions + next-velocities * dt
velocities = next-velocities
positions = next-positions
(positions, velocities)
"#;

const STANDALONE_PARTICLE_SOURCE: &str = r#"
particle-count := 10f32
particle-index := 1f32..=particle-count
particle-x := (particle-index / particle-count) * 2f32 - 1f32
particle-y := particle-x * particle-x - 0.5<f32>
~positions := [particle-x; particle-y]
~velocities := [(0f32 - particle-y); particle-x] * 0.18<f32>
acceleration := (0f32 - positions) * 0.34<f32>
next-velocities := (velocities + acceleration * 0.008333333<f32>) * 0.997<f32>
next-positions := positions + next-velocities * 0.008333333<f32>
velocities = next-velocities
positions = next-positions
(positions, velocities)
"#;

const SERVED_PARTICLE_SOURCE: &str = include_str!("../../../examples/gpu-particles/particles.mec");

fn compile_source(
    source: &str,
    inputs: impl IntoIterator<Item = (&'static str, RuntimeHostInputValue)>,
) -> mech_engine::ProgramArtifact {
    let tree = mech_syntax::parse(source).expect("source must parse");
    let inputs = inputs
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value))
        .collect::<BTreeMap<_, _>>();
    let external_input_names = inputs
        .keys()
        .map(|name| name.strip_prefix("host-").unwrap_or(name).to_owned())
        .collect();
    compiler()
        .compile_tree_artifact_with_inputs(&tree, &inputs, &external_input_names)
        .expect("source must compile")
        .into_artifact()
}

fn compiler() -> ProgramCompiler {
    RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .expect("source compiler must build")
}

fn isolated_gpu_tree(source: &str) -> Program {
    let tree = mech_syntax::parse(source).expect("complete mixed source must parse");
    let imports = tree
        .body
        .sections
        .iter()
        .flat_map(|section| &section.elements)
        .filter_map(|element| {
            let SectionElement::MechCode(code) = element else {
                return None;
            };
            let imports = code
                .iter()
                .filter(|(code, _)| matches!(code, MechCode::Import(_)))
                .cloned()
                .collect::<Vec<_>>();
            (!imports.is_empty()).then_some(SectionElement::MechCode(imports))
        })
        .collect::<Vec<_>>();
    let region = tree
        .body
        .sections
        .iter()
        .find(|section| !section.annotations.is_empty())
        .expect("mixed source must contain a compute region")
        .clone();
    Program {
        title: None,
        body: Body {
            sections: vec![
                Section {
                    subtitle: None,
                    annotations: Vec::new(),
                    elements: imports,
                },
                region,
            ],
        },
    }
}

fn compile_isolated_gpu_source(source: &str) -> mech_engine::ProgramArtifact {
    let external_input_names = ["force-point", "force-strength", "dt"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    compiler()
        .compile_tree_artifact_with_inputs(
            &isolated_gpu_tree(source),
            &Default::default(),
            &external_input_names,
        )
        .expect("isolated GPU source must compile")
        .into_artifact()
}

fn particle_inputs() -> Vec<(&'static str, RuntimeHostInputValue)> {
    vec![
        (
            "host-positions",
            RuntimeHostInputValue::F32Matrix {
                rows: 4,
                columns: 2,
                values: vec![1.0, -1.0, 2.0, -2.0, 0.5, -0.5, 4.0, -4.0],
            },
        ),
        (
            "host-velocities",
            RuntimeHostInputValue::F32Matrix {
                rows: 4,
                columns: 2,
                values: vec![0.0; 8],
            },
        ),
        ("host-origin", RuntimeHostInputValue::F32(0.0)),
        ("host-attraction", RuntimeHostInputValue::F32(0.5)),
        ("host-drag", RuntimeHostInputValue::F32(0.9)),
        ("host-dt", RuntimeHostInputValue::F32(0.1)),
    ]
}

#[test]
fn lowered_program_exposes_exact_typed_region_ports() {
    let artifact = compile_source(
        "left := host-left\nright := host-right\nresult := left + right\nresult",
        [
            (
                "host-left",
                RuntimeHostInputValue::F32Matrix {
                    rows: 2,
                    columns: 3,
                    values: vec![1.0; 6],
                },
            ),
            (
                "host-right",
                RuntimeHostInputValue::F32Matrix {
                    rows: 2,
                    columns: 3,
                    values: vec![2.0; 6],
                },
            ),
        ],
    );
    let program = ComputeLowerer
        .compile(&artifact)
        .expect("particle source must lower");
    let interface = program.compute_program().interface();
    let left = interface
        .input_named("left")
        .expect("left must be a live region input");

    assert_eq!(left.element, ComputeElementType::F32);
    assert_eq!(left.dimensions.as_ref(), [2, 3]);
    assert_eq!(left.layout(), TensorLayout::RowMajor);
    assert_eq!(interface.outputs[0].dimensions.as_ref(), [2, 3]);
}

#[test]
fn compiler_product_runs_through_the_backend_neutral_cpu_session() {
    let artifact = compile_source(STANDALONE_PARTICLE_SOURCE, []);
    let lowered = ComputeLowerer
        .compile(&artifact)
        .expect("standalone particle source must lower");
    let reconstructed = ElementwiseKernel::from_compute_program(lowered.compute_program())
        .expect("the compute program must be self-contained");
    assert_eq!(reconstructed.wgsl(), lowered.wgsl());

    let registry = native_compute_backend_registry();
    let backend = registry
        .resolve(
            &BackendRequest::Cpu,
            ComputePlatform::Native,
            ComputePlacement::Compute,
            reconstructed.compute_program(),
        )
        .expect("the scalar CPU backend must accept the compiler product");
    assert_eq!(backend.descriptor().id.as_str(), "cpu-scalar");
    let executable = backend
        .compile(reconstructed.compute_program())
        .expect("the scalar CPU backend must compile the compute program");
    let mut session = executable
        .create_session(&ComputeInitializerSet::default())
        .expect("static particle source needs no live-input initializers");
    let report = session
        .dispatch(&ComputeDispatchRequest::new(NonZeroU32::new(2).unwrap()))
        .expect("resident CPU turns must execute");
    assert_eq!(report.completed_turns, 2);
    let outputs = session
        .read_outputs(&ComputeOutputSelection::All)
        .expect("resident outputs must be readable on request");
    assert!(!outputs.values.is_empty());
}

#[cfg(feature = "native")]
#[test]
fn particle_source_and_bytecode_share_cpu_and_wgpu_results() {
    let artifact = compile_source(STANDALONE_PARTICLE_SOURCE, []);
    let encoded = encode_program_artifact_sections(&artifact).unwrap();
    let decoded = decode_program_artifact_sections(&encoded).unwrap();
    let source = ComputeLowerer.compile(&artifact).unwrap();
    let bytecode = ComputeLowerer.compile(&decoded).unwrap();
    assert_eq!(source.wgsl(), bytecode.wgsl());

    let registry = native_compute_backend_registry();
    for backend in ["cpu-scalar", "wgpu"] {
        let request = BackendRequest::parse(backend).unwrap();
        let run = |program: &mech_compute::ComputeProgram| {
            let factory = match registry.resolve(
                &request,
                ComputePlatform::Native,
                ComputePlacement::Compute,
                program,
            ) {
                Ok(factory) => factory,
                Err(error) if backend == "wgpu" && error.to_string().contains("adapter") => {
                    return None;
                }
                Err(error) => panic!("{backend} rejected particle bytecode: {error}"),
            };
            let executable = factory.compile(program).unwrap();
            let mut session = executable
                .create_session(&ComputeInitializerSet::default())
                .unwrap();
            session
                .dispatch(&ComputeDispatchRequest::new(NonZeroU32::new(2).unwrap()))
                .unwrap();
            Some(session.read_outputs(&ComputeOutputSelection::All).unwrap())
        };
        let Some(source_outputs) = run(source.compute_program()) else {
            continue;
        };
        let bytecode_outputs = run(bytecode.compute_program()).unwrap();
        assert_eq!(
            source_outputs.values.keys().collect::<Vec<_>>(),
            bytecode_outputs.values.keys().collect::<Vec<_>>()
        );
        for (port, source_value) in source_outputs.values {
            let source_values = match source_value {
                ComputeValue::ScalarF32(value) => vec![value],
                ComputeValue::TensorF32 { values, .. } => values.to_vec(),
            };
            let bytecode_values = match &bytecode_outputs.values[&port] {
                ComputeValue::ScalarF32(value) => vec![*value],
                ComputeValue::TensorF32 { values, .. } => values.to_vec(),
            };
            assert_close_with_tolerance(&source_values, &bytecode_values, 1.0e-5);
        }
    }
}

#[test]
fn ordinary_compute_host_dispatches_the_compiler_product_after_commit() {
    let artifact = compile_source(STANDALONE_PARTICLE_SOURCE, []);
    let lowered = ComputeLowerer
        .compile(&artifact)
        .expect("standalone particle source must lower");
    let factory = ComputeHostFactory::new(
        "particle-field",
        ComputePlacement::Compute,
        lowered.compute_program().clone(),
        ComputeInitializerSet::default(),
        native_compute_backend_registry(),
        ComputePlatform::Native,
    )
    .unwrap();
    let settings = ConfigValue::Map(BTreeMap::from([
        (
            "region".to_owned(),
            ConfigValue::String("particle-field".to_owned()),
        ),
        ("backend".to_owned(), ConfigValue::String("cpu".to_owned())),
    ]));
    let installation = factory.instantiate("particles", &settings).unwrap();
    let provider = &installation.resource_providers[0];
    let effect = provider
        .prepare_write(RuntimeResourceWriteRequest {
            base_uri: "compute://particles/kernel".to_owned(),
            path: "turn".to_owned(),
            context_name: "particles".to_owned(),
            operation: RuntimeCapabilityOperation::Write,
            value: RuntimeHostInputValue::F32(1.0).into_value().unwrap(),
            intent: RuntimeResourceWriteIntent::Send,
        })
        .unwrap();
    let PreparedRuntimeEffect::AfterCommit(mut effect) = effect else {
        panic!("compute dispatch must be deferred until transaction commit")
    };
    effect.deliver().unwrap();

    let turns = provider
        .read(RuntimeResourceReadRequest {
            base_uri: "compute://particles/kernel".to_owned(),
            path: "turns".to_owned(),
            context_name: "particles".to_owned(),
        })
        .unwrap();
    let ValueData::F64(turns) = turns.data() else {
        panic!("turn telemetry must be f64")
    };
    assert_eq!(turns.to_f64(), 1.0);
}

#[test]
fn named_mechdown_region_reaches_neutral_compute_placement_and_gpu_lowering() {
    let source = SERVED_PARTICLE_SOURCE.replacen("1000000f32", "64f32", 1);
    let complete = mech_syntax::parse(&source).expect("complete source must parse");
    assert!(
        complete
            .body
            .sections
            .iter()
            .any(|section| section.annotations.is_empty())
    );
    let product = compiler()
        .compile_tree(&isolated_gpu_tree(&source))
        .expect("named source must compile");

    assert_eq!(product.artifact().compute_regions().len(), 1);
    let region = &product.artifact().compute_regions()[0];
    assert_eq!(region.name.as_ref(), "particle-field");
    assert_eq!(region.placement, ComputePlacement::Compute);
    assert!(!region.nodes.is_empty());
    let bytecode = ParsedProgram::from_bytes(product.bytecode()).unwrap();
    let bytecode_artifact = decode_program_artifact_sections(&bytecode.artifact).unwrap();
    assert_eq!(
        bytecode_artifact.compute_regions(),
        product.artifact().compute_regions()
    );

    let lowering_artifact = compile_isolated_gpu_source(&source);
    let placement = ComputeLowerer.plan(&lowering_artifact);
    assert!(placement.violations.is_empty());
    assert_eq!(placement.regions.len(), 1);
    assert_eq!(placement.regions[0].name.as_deref(), Some("particle-field"),);
    assert_eq!(
        placement.regions[0].requested,
        Some(ComputePlacement::Compute),
    );
    let lowered = ComputeLowerer
        .compile(&lowering_artifact)
        .expect("one named compute region must lower to one GPU program");
    let cpu_lowered = ComputeLowerer
        .compile_cpu(&lowering_artifact)
        .expect("the same neutral region must lower to the CPU executor");
    assert_eq!(cpu_lowered.wgsl(), lowered.wgsl());
    assert_eq!(cpu_lowered.bindings().len(), lowered.bindings().len());
    assert_eq!(cpu_lowered.dispatch_elements(), lowered.dispatch_elements());
    assert!(
        lowered.bindings().len() <= 8,
        "the example must fit WebGPU's guaranteed storage-buffer limit"
    );
    for binding in lowered
        .bindings()
        .iter()
        .filter(|binding| binding.role() == GpuBindingRole::Input && binding.elements == 1)
    {
        assert!(
            lowered
                .wgsl()
                .contains(&format!("input_{}[0u]", binding.slot().get())),
            "scalar input {} must be broadcast from element zero",
            binding.name
        );
    }
}

#[test]
fn hard_cpu_region_is_not_silently_sent_to_gpu() {
    let source = SERVED_PARTICLE_SOURCE
        .replacen("1000000f32", "16f32", 1)
        .replacen("@compute", "@cpu", 1);
    let artifact = compile_isolated_gpu_source(&source);
    let placement = ComputeLowerer.plan(&artifact);

    assert!(
        placement
            .nodes
            .iter()
            .filter(|node| artifact.compute_regions()[0].nodes.contains(&node.node))
            .all(|node| node.target != ExecutionTarget::Gpu)
    );
    ComputeLowerer
        .compile_cpu(&artifact)
        .expect("hard CPU region must run under the CPU executor");
    let error = ComputeLowerer.compile(&artifact).unwrap_err();
    assert!(error.to_string().contains("requires CPU execution"));
}

#[test]
fn hard_gpu_region_is_not_silently_sent_to_cpu() {
    let source = SERVED_PARTICLE_SOURCE
        .replacen("1000000f32", "16f32", 1)
        .replacen("@compute", "@gpu", 1);
    let artifact = compile_isolated_gpu_source(&source);

    ComputeLowerer
        .compile(&artifact)
        .expect("hard GPU region must run under the GPU executor");
    let error = ComputeLowerer.compile_cpu(&artifact).unwrap_err();
    assert!(error.to_string().contains("requires GPU execution"));
}

#[test]
fn particle_program_is_lowered_from_mech_to_fused_wgsl() {
    let artifact = compile_source(PARTICLE_SOURCE, particle_inputs());
    assert_eq!(
        artifact
            .slots()
            .iter()
            .filter(|slot| slot.role == SlotRole::State)
            .count(),
        2
    );
    let state_initializers = artifact
        .slots()
        .iter()
        .filter(|slot| slot.role == SlotRole::State)
        .filter_map(|slot| slot.initializer)
        .collect::<Vec<_>>();
    assert_eq!(state_initializers.len(), 2);
    assert!(
        state_initializers
            .iter()
            .all(|initializer| match initializer {
                mech_engine::InitializerReference::Constant(constant) =>
                    artifact.constants().get(*constant).is_some(),
            })
    );
    let placement = ComputeLowerer.plan(&artifact);
    assert!(
        placement.fully_accelerated,
        "unexpected CPU placement: {:#?}",
        placement.nodes
    );
    assert_eq!(placement.regions.len(), 1);
    assert_eq!(
        placement
            .slots
            .iter()
            .filter(|slot| slot.residence == SlotResidence::DeviceState)
            .count(),
        2
    );
    assert!(
        placement
            .transfers
            .iter()
            .any(|transfer| transfer.direction == TransferDirection::Upload)
    );
    assert_eq!(
        placement
            .transfers
            .iter()
            .filter(|transfer| transfer.direction == TransferDirection::Readback)
            .count(),
        2
    );
    let program = ComputeLowerer
        .compile(&artifact)
        .unwrap_or_else(|error| panic!("particle source must be admitted: {error}"));

    assert!(
        program
            .wgsl()
            .contains("// Generated from a typed Mech ComputeProgram.")
    );
    assert!(program.wgsl().contains("@compute @workgroup_size(64)"));
    assert!(!program.wgsl().contains("gravity"));
    assert_eq!(program.dispatch_elements(), 8);
    assert_eq!(program.workgroup_count(), 1);
    assert_eq!(
        program
            .bindings()
            .iter()
            .filter(|binding| binding.role() == GpuBindingRole::StateRead)
            .count(),
        2
    );
    assert_eq!(
        program
            .bindings()
            .iter()
            .filter(|binding| binding.role() == GpuBindingRole::StateWrite)
            .count(),
        2
    );
    for (_, slot, elements) in program.outputs() {
        assert_eq!(elements, 8);
        assert!(program.bindings().iter().any(|binding| {
            binding.role() == GpuBindingRole::StateWrite && binding.slot() == slot
        }));
    }

    let mut inputs = BTreeMap::new();
    inputs.insert(
        "positions".to_owned(),
        vec![1.0, -1.0, 2.0, -2.0, 0.5, -0.5, 4.0, -4.0],
    );
    inputs.insert("velocities".to_owned(), vec![0.0; 8]);
    inputs.insert("origin".to_owned(), vec![0.0]);
    inputs.insert("attraction".to_owned(), vec![0.5]);
    inputs.insert("drag".to_owned(), vec![0.9]);
    inputs.insert("dt".to_owned(), vec![0.1]);
    let plan = GpuExecutionPlan::build(GpuKernelPlanSource::Elementwise(&program), &inputs)
        .expect("particle kernel must produce a physical GPU execution plan");
    assert_eq!(plan.kernel_kind, GpuPlanKernelKind::Elementwise);
    assert_eq!(plan.wgsl, program.wgsl());
    assert_eq!(plan.bindings.len(), program.bindings().len());
    assert_eq!(plan.states.len(), 2);
    assert_eq!(plan.physical_outputs.len(), 2);
    assert!(
        plan.bindings
            .iter()
            .filter(|binding| binding.role == GpuExecutionBindingRole::StateRead)
            .all(|binding| binding.initial_values.is_some()),
        "the physical plan must carry resident state initializers"
    );
    assert!(
        plan.physical_outputs
            .iter()
            .all(|output| output.binding.is_none()),
        "resident particle outputs must read from the selected ping-pong state buffer"
    );
    let outputs = program.run_cpu(&inputs).expect("CPU backend must run");

    let expected_velocities = [-0.045, -0.0225, 0.045, 0.0225, -0.09, -0.18, 0.09, 0.18];
    let expected_positions = [
        0.9955, 0.49775, -0.9955, -0.49775, 1.991, 3.982, -1.991, -3.982,
    ];
    assert_close(&outputs["result.1"], &expected_velocities);
    assert_close(&outputs["result.0"], &expected_positions);
}

#[test]
fn particle_execution_plan_groups_logical_output_aliases_once() {
    let source = PARTICLE_SOURCE.replacen(
        "(positions, velocities)",
        "(positions, positions, velocities)",
        1,
    );
    let artifact = compile_source(&source, particle_inputs());
    let program = ComputeLowerer
        .compile(&artifact)
        .expect("aliased particle outputs must lower");
    let inputs = BTreeMap::from([
        ("positions".to_owned(), vec![0.0; 8]),
        ("velocities".to_owned(), vec![0.0; 8]),
        ("origin".to_owned(), vec![0.0]),
        ("attraction".to_owned(), vec![0.5]),
        ("drag".to_owned(), vec![0.9]),
        ("dt".to_owned(), vec![0.1]),
    ]);
    let plan = GpuExecutionPlan::build(GpuKernelPlanSource::Elementwise(&program), &inputs)
        .expect("aliased particle outputs must produce one physical readback plan");

    assert_eq!(plan.outputs.len(), 3);
    assert_eq!(plan.physical_outputs.len(), 2);
    assert!(
        plan.physical_outputs
            .iter()
            .any(|output| output.aliases.len() == 2),
        "two names for the same resident value must share one physical transfer"
    );
}

#[test]
fn standalone_particle_program_needs_no_host_inputs() {
    let artifact = compile_source(STANDALONE_PARTICLE_SOURCE, []);
    let program = ComputeLowerer
        .compile(&artifact)
        .unwrap_or_else(|error| panic!("standalone particle source must be admitted: {error}"));
    assert!(
        program
            .bindings()
            .iter()
            .all(|binding| binding.role() != GpuBindingRole::Input),
        "initialization-only source values must not become turn inputs"
    );
    let position_slot = program
        .outputs()
        .find_map(|(name, slot, _)| (name == "result.0").then_some(slot))
        .expect("particle position output must exist");
    assert_eq!(
        program.output_dimensions(position_slot),
        Some([2, 10].as_slice())
    );
    let mut cpu = program
        .prepare_cpu(&BTreeMap::new())
        .expect("standalone CPU executor must prepare");
    let initial = cpu.outputs().expect("standalone initial outputs must read");
    assert_eq!(initial["result.0"].len(), 20);
    assert_eq!(initial["result.1"].len(), 20);
    assert!(initial["result.0"].iter().any(|value| *value != 0.0));
    assert!(initial["result.1"].iter().any(|value| *value != 0.0));
    let expected_x = (1..=10)
        .map(|index| (index as f32 / 10.0) * 2.0 - 1.0)
        .collect::<Vec<_>>();
    let expected_positions = expected_x
        .iter()
        .copied()
        .chain(expected_x.iter().map(|x| x * x - 0.5))
        .collect::<Vec<_>>();
    assert_close(&initial["result.0"], &expected_positions);

    cpu.dispatch_turns(6)
        .expect("standalone CPU executor must advance");
    let cycled = cpu.outputs().expect("cycled outputs must read");
    assert_ne!(cycled["result.0"], initial["result.0"]);
    assert_ne!(cycled["result.1"], initial["result.1"]);
}

#[test]
fn derived_block_broadcast_requires_materialization() {
    let artifact = compile_source(
        r#"
column := [1f32; 2f32] + 1f32
matrix := [1f32 2f32 3f32; 4f32 5f32 6f32]
result := matrix + column
result
"#,
        [],
    );
    let error = ComputeLowerer
        .compile(&artifact)
        .expect_err("a derived block broadcast must not use thread-local remapping");
    assert!(error.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == GpuDiagnosticCode::DerivedBroadcastRequiresMaterialization
    }));
}

#[test]
fn particle_example_is_one_mixed_mech_document() {
    let tree = mech_syntax::parse(SERVED_PARTICLE_SOURCE).expect("complete source must parse");
    let regions = tree
        .body
        .sections
        .iter()
        .filter(|section| !section.annotations.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(regions.len(), 1);
    assert_eq!(
        mech_engine::section_compute_placement(regions[0]).unwrap(),
        Some(ComputePlacement::Compute)
    );
    assert_eq!(
        regions[0].subtitle.as_ref().unwrap().to_string().trim(),
        "particle-field"
    );
    assert!(SERVED_PARTICLE_SOURCE.contains("pointer://pointer/frame"));
    assert!(SERVED_PARTICLE_SOURCE.contains("compute://particles/kernel"));
    assert!(SERVED_PARTICLE_SOURCE.contains("@particles/input/force-point <- force-point"));
    assert!(SERVED_PARTICLE_SOURCE.contains("@particles/turn <- pulse"));
    assert!(!SERVED_PARTICLE_SOURCE.contains("host-positions"));
}

#[test]
fn served_particle_field_stays_bounded() {
    let source = SERVED_PARTICLE_SOURCE.replacen("1000000f32", "512f32", 1);
    let artifact = compile_isolated_gpu_source(&source);
    let program = ComputeLowerer
        .compile(&artifact)
        .unwrap_or_else(|error| panic!("served particle source must be admitted: {error}"));

    assert!(!program.wgsl().contains("sin("));
    assert!(!program.wgsl().contains("cos("));
    assert!(!program.wgsl().contains("%"));

    let inputs = BTreeMap::from([
        ("force-point".to_owned(), vec![0.0, 0.0]),
        ("force-strength".to_owned(), vec![0.0]),
        ("dt".to_owned(), vec![0.016]),
    ]);
    let mut cpu = program
        .prepare_cpu(&inputs)
        .expect("particle field must prepare from its captured constants");
    cpu.dispatch_turns(1)
        .expect("particle field must establish its first output");
    let initial = cpu.outputs().expect("initial particle field must read");
    let initial_radius = root_mean_square_radius(&initial["result.0"]);

    cpu.dispatch_turns(600)
        .expect("conservative particle field must advance");
    let evolved = cpu.outputs().expect("evolved particle field must read");
    let evolved_radius = root_mean_square_radius(&evolved["result.0"]);

    assert!(evolved["result.0"].iter().all(|value| value.is_finite()));
    assert!(evolved["result.0"].iter().all(|value| value.abs() < 2.0));
    assert!(
        evolved_radius > initial_radius * 0.7,
        "particle field collapsed: initial RMS radius {initial_radius}, evolved {evolved_radius}"
    );
}

#[test]
fn served_pointer_press_materially_changes_the_particle_trajectory() {
    let source = SERVED_PARTICLE_SOURCE.replacen("1000000f32", "512f32", 1);
    let artifact = compile_isolated_gpu_source(&source);
    let program = ComputeLowerer
        .compile(&artifact)
        .expect("served particle source must be admitted");
    let inputs = |strength| {
        BTreeMap::from([
            ("force-point".to_owned(), vec![0.7, 0.6]),
            ("force-strength".to_owned(), vec![strength]),
            ("dt".to_owned(), vec![0.016]),
        ])
    };
    let mut released = program
        .prepare_cpu(&inputs(0.0))
        .expect("released particle field must prepare");
    let mut pressed = program
        .prepare_cpu(&inputs(1.25))
        .expect("pressed particle field must prepare");

    released
        .dispatch_turns(60)
        .expect("released field must run");
    pressed.dispatch_turns(60).expect("pressed field must run");
    let released = released.outputs().expect("released positions must read");
    let pressed = pressed.outputs().expect("pressed positions must read");
    let displacement = pressed["result.0"]
        .iter()
        .zip(&released["result.0"])
        .map(|(pressed, released)| (pressed - released).abs())
        .fold(0.0_f32, f32::max);

    assert!(
        displacement > 0.1,
        "one second of pointer input was not visible: maximum displacement {displacement}"
    );
}

#[test]
fn owned_cpu_session_accepts_new_inputs_without_resetting_state() {
    let source = SERVED_PARTICLE_SOURCE.replacen("1000000f32", "512f32", 1);
    let artifact = compile_isolated_gpu_source(&source);
    let program = ComputeLowerer
        .compile(&artifact)
        .expect("served particle source must be admitted");
    let initial = BTreeMap::from([
        ("force-point".to_owned(), vec![0.0, 0.0]),
        ("force-strength".to_owned(), vec![0.0]),
        ("dt".to_owned(), vec![0.016]),
    ]);
    let mut session = program
        .into_cpu(&initial)
        .expect("CPU session must prepare");
    session.dispatch_turns(30).expect("unforced turns must run");
    let before = session.output("result.0").expect("positions must read");
    session
        .update_inputs(&BTreeMap::from([
            ("force-point".to_owned(), vec![0.7, 0.6]),
            ("force-strength".to_owned(), vec![1.25]),
        ]))
        .expect("changed scalar inputs must update");
    session.dispatch_turns(30).expect("forced turns must run");
    let after = session.output("result.0").expect("positions must read");

    assert_ne!(before, after);
}

fn root_mean_square_radius(positions: &[f32]) -> f32 {
    let particles = positions.len() / 2;
    let squared_radius = (0..particles)
        .map(|index| {
            let x = positions[index];
            let y = positions[particles + index];
            x * x + y * y
        })
        .sum::<f32>();
    (squared_radius / particles as f32).sqrt()
}

#[cfg(feature = "native")]
#[test]
fn native_gpu_matches_the_cpu_backend_when_an_adapter_is_available() {
    let artifact = compile_source(PARTICLE_SOURCE, particle_inputs());
    let program = ComputeLowerer
        .compile(&artifact)
        .expect("particle source must be admitted");
    let inputs = BTreeMap::from([
        (
            "positions".to_owned(),
            vec![1.0, -1.0, 2.0, -2.0, 0.5, -0.5, 4.0, -4.0],
        ),
        ("velocities".to_owned(), vec![0.0; 8]),
        ("origin".to_owned(), vec![0.0]),
        ("attraction".to_owned(), vec![0.5]),
        ("drag".to_owned(), vec![0.9]),
        ("dt".to_owned(), vec![0.1]),
    ]);
    let cpu = program.run_cpu(&inputs).expect("CPU backend must run");
    let gpu = match program.run_gpu(&inputs) {
        Ok(gpu) => gpu,
        Err(mech_gpu::GpuExecutionError::AdapterUnavailable) => return,
        Err(error) => panic!("GPU dispatch failed: {error}"),
    };
    assert_eq!(
        cpu.keys().collect::<Vec<_>>(),
        gpu.keys().collect::<Vec<_>>()
    );
    for (name, cpu_values) in cpu {
        assert_close(&gpu[&name], &cpu_values);
    }
}

#[cfg(feature = "native")]
#[test]
fn served_particle_shader_matches_cpu_with_pointer_force() {
    let artifact = compile_isolated_gpu_source(SERVED_PARTICLE_SOURCE);
    let program = ComputeLowerer
        .compile(&artifact)
        .expect("served particle source must lower");
    let inputs = BTreeMap::from([
        ("force-point".to_owned(), vec![0.3, -0.2]),
        ("force-strength".to_owned(), vec![1.25]),
        ("dt".to_owned(), vec![0.016]),
    ]);
    let cpu = program.run_cpu(&inputs).expect("CPU reference must run");
    let gpu = match program.run_gpu_profiled(&inputs) {
        Ok(gpu) => gpu,
        Err(mech_gpu::GpuExecutionError::AdapterUnavailable) => return,
        Err(error) => panic!("served particle shader failed: {error}"),
    };
    eprintln!("served particle adapter: {}", gpu.adapter);
    assert_eq!(
        cpu.keys().collect::<Vec<_>>(),
        gpu.outputs.keys().collect::<Vec<_>>()
    );
    let max_error = cpu
        .iter()
        .map(|(name, cpu_values)| maximum_absolute_error(&gpu.outputs[name], cpu_values))
        .fold(0.0_f32, f32::max);
    eprintln!("served particle maximum CPU/GPU error: {max_error:.3e}");
    assert!(
        max_error <= 1.0e-6,
        "CPU/GPU error {max_error} is too large"
    );
}

#[cfg(feature = "native")]
#[test]
fn resident_gpu_feeds_particle_outputs_into_the_next_turn() {
    let artifact = compile_source(PARTICLE_SOURCE, particle_inputs());
    let program = ComputeLowerer
        .compile(&artifact)
        .expect("particle source must be admitted");
    let inputs = BTreeMap::from([
        (
            "positions".to_owned(),
            vec![1.0, -1.0, 2.0, -2.0, 0.5, -0.5, 4.0, -4.0],
        ),
        ("velocities".to_owned(), vec![0.0; 8]),
        ("origin".to_owned(), vec![0.0]),
        ("attraction".to_owned(), vec![0.5]),
        ("drag".to_owned(), vec![0.9]),
        ("dt".to_owned(), vec![0.1]),
    ]);
    let mut cpu = program.prepare_cpu(&inputs).expect("CPU must prepare");
    cpu.dispatch_turns(3).expect("CPU turns must run");
    let expected = cpu.outputs().expect("CPU outputs must read");

    let initial_inputs = BTreeMap::from([
        (
            "positions".to_owned(),
            vec![1.0, -1.0, 2.0, -2.0, 0.5, -0.5, 4.0, -4.0],
        ),
        ("velocities".to_owned(), vec![0.0; 8]),
        ("origin".to_owned(), vec![0.0]),
        ("attraction".to_owned(), vec![0.5]),
        ("drag".to_owned(), vec![0.9]),
        ("dt".to_owned(), vec![0.1]),
    ]);
    let mut resident = match program.prepare_resident(&initial_inputs) {
        Ok(resident) => resident,
        Err(mech_gpu::GpuExecutionError::AdapterUnavailable) => return,
        Err(error) => panic!("resident GPU preparation failed: {error}"),
    };
    let gpu = resident.run_turns(3).expect("resident turns must run");
    assert_close(&gpu.outputs["result.0"], &expected["result.0"]);
    assert_close(&gpu.outputs["result.1"], &expected["result.1"]);
}

#[cfg(feature = "native")]
#[test]
fn resident_gpu_accepts_new_inputs_without_resetting_state() {
    let source = SERVED_PARTICLE_SOURCE.replacen("1000000f32", "512f32", 1);
    let artifact = compile_isolated_gpu_source(&source);
    let program = ComputeLowerer
        .compile(&artifact)
        .expect("served particle source must be admitted");
    let initial = BTreeMap::from([
        ("force-point".to_owned(), vec![0.0, 0.0]),
        ("force-strength".to_owned(), vec![0.0]),
        ("dt".to_owned(), vec![0.016]),
    ]);
    let changed = BTreeMap::from([
        ("force-point".to_owned(), vec![0.7, 0.6]),
        ("force-strength".to_owned(), vec![1.25]),
    ]);

    let mut cpu = program
        .prepare_cpu(&initial)
        .expect("CPU reference must prepare");
    cpu.dispatch_turns(30).expect("initial CPU turns must run");
    cpu.update_inputs(&changed)
        .expect("CPU inputs must update in place");
    cpu.dispatch_turns(30).expect("updated CPU turns must run");
    let expected = cpu.outputs().expect("CPU outputs must read");

    let mut resident = match program.prepare_resident(&initial) {
        Ok(resident) => resident,
        Err(mech_gpu::GpuExecutionError::AdapterUnavailable) => return,
        Err(error) => panic!("resident GPU preparation failed: {error}"),
    };
    resident
        .dispatch_turns(30)
        .expect("initial GPU turns must run");
    for (name, values) in &changed {
        resident
            .update_input(name, values)
            .expect("resident GPU input must update in place");
    }
    let actual = resident.run_turns(30).expect("updated GPU turns must run");

    assert_close(&actual.outputs["result.0"], &expected["result.0"]);
    assert_close(&actual.outputs["result.1"], &expected["result.1"]);
}

#[test]
fn resident_cpu_advances_artifact_state_without_host_feedback() {
    let artifact = compile_source(PARTICLE_SOURCE, particle_inputs());
    let program = ComputeLowerer
        .compile(&artifact)
        .expect("particle source must be admitted");
    let inputs = BTreeMap::from([
        ("origin".to_owned(), vec![0.0]),
        ("attraction".to_owned(), vec![0.5]),
        ("drag".to_owned(), vec![0.9]),
        ("dt".to_owned(), vec![0.1]),
    ]);
    let mut cpu = program.prepare_cpu(&inputs).expect("CPU must prepare");
    cpu.dispatch_turns(3).expect("CPU turns must run");
    let outputs = cpu.outputs().expect("CPU outputs must read");

    let mut position = 1.0_f32;
    let mut velocity = 0.0_f32;
    for _ in 0..3 {
        velocity = (velocity + (0.0 - position) * 0.5 * 0.1) * 0.9;
        position += velocity * 0.1;
    }
    assert!((outputs["result.0"][0] - position).abs() < 1.0e-6);
    assert!((outputs["result.1"][0] - velocity).abs() < 1.0e-6);
}

#[test]
fn unsupported_program_reports_why_instead_of_falling_back() {
    let artifact = compile_source(
        "answer := left ^ right",
        [
            ("left", RuntimeHostInputValue::F32(1.0)),
            ("right", RuntimeHostInputValue::F32(2.0)),
        ],
    );
    let error = ComputeLowerer
        .compile(&artifact)
        .expect_err("power is outside the first GPU capability set");

    assert!(error.diagnostics().iter().any(|diagnostic| {
        matches!(
            diagnostic.code,
            GpuDiagnosticCode::OpaqueOperationContract | GpuDiagnosticCode::OperationUnsupported
        ) && diagnostic.node.is_some()
            && diagnostic.operation.is_some()
    }));
    let placement = ComputeLowerer.plan(&artifact);
    assert!(!placement.fully_accelerated);
    assert!(placement.nodes.iter().any(|node| {
        node.target == ExecutionTarget::Cpu && node.reason.contains("has no GPU lowering")
    }));
}

#[test]
fn mixed_graph_reports_gpu_regions_and_cpu_transfer_boundaries() {
    let artifact = compile_source(
        "sum := left + right\npowered := sum ^ exponent\nresult := powered * scale\nresult",
        [
            ("left", RuntimeHostInputValue::F32(1.0)),
            ("right", RuntimeHostInputValue::F32(2.0)),
            ("exponent", RuntimeHostInputValue::F32(3.0)),
            ("scale", RuntimeHostInputValue::F32(4.0)),
        ],
    );
    let placement = ComputeLowerer.plan(&artifact);

    assert!(!placement.fully_accelerated);
    assert_eq!(placement.regions.len(), 2);
    assert_eq!(
        placement
            .nodes
            .iter()
            .filter(|node| node.target == ExecutionTarget::Cpu)
            .count(),
        1
    );
    assert!(placement.transfers.iter().any(|transfer| {
        transfer.direction == TransferDirection::Readback && transfer.consumer.is_some()
    }));
    assert!(placement.transfers.iter().any(|transfer| {
        transfer.direction == TransferDirection::Upload && transfer.consumer.is_some()
    }));
}

#[test]
fn particle_arithmetic_reaches_artifact_with_declared_contracts() {
    let artifact = compile_source(PARTICLE_SOURCE, particle_inputs());
    assert!(!artifact.nodes().is_empty());
    for node in artifact.nodes() {
        assert_ne!(node.operation.module_path.as_ref(), ["runtime"]);
        assert!(matches!(
            artifact.contracts().get(node.contract),
            Some(ResolvedOperationContract::Declared(_))
        ));
    }
}

#[test]
fn specialized_matrix_factories_do_not_leak_into_artifact_operations() {
    let artifact = compile_source(
        r#"
+> math
input := host-input
projection := [1f32 2f32 3f32
               4f32 5f32 6f32]
~result := [0f32; 0f32]
result = projection ** input
result
"#,
        [(
            "host-input",
            RuntimeHostInputValue::F32Matrix {
                rows: 3,
                columns: 1,
                values: vec![1.0, 10.0, 100.0],
            },
        )],
    );
    let operations = artifact
        .nodes()
        .iter()
        .map(|node| {
            node.operation
                .module_path
                .iter()
                .chain(std::iter::once(&node.operation.operation_name))
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect::<std::collections::BTreeSet<_>>();

    assert!(operations.contains("matrix/multiply"));
    assert!(operations.contains("core/assign"));
    assert!(operations.iter().all(|name| !name.starts_with("runtime/")));
}

fn assert_close(actual: &[f32], expected: &[f32]) {
    assert_close_with_tolerance(actual, expected, 1.0e-6);
}

fn assert_close_with_tolerance(actual: &[f32], expected: &[f32], tolerance: f32) {
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (actual - expected).abs() < tolerance,
            "element {index}: expected {expected}, got {actual}"
        );
    }
}

#[cfg(feature = "native")]
fn maximum_absolute_error(actual: &[f32], expected: &[f32]) -> f32 {
    assert_eq!(actual.len(), expected.len());
    actual
        .iter()
        .zip(expected)
        .map(|(actual, expected)| (actual - expected).abs())
        .fold(0.0, f32::max)
}
