use std::collections::BTreeMap;

use mech_engine::{decode_program_artifact_sections, encode_program_artifact_sections};
use mech_gpu::ComputeLowerer;
use mech_runtime::{RuntimeBuilder, SourceDocument};
use mech_syntax::document::{ParseConfig, Revision};

#[test]
fn canonical_shipped_ekf_region_lowers_from_source_and_bytecode() {
    let shipped = include_str!("../../../examples/ekf/localization.mec");
    let start = shipped.find("5. ekf-batch @compute\n").unwrap();
    let end = shipped.find("6. Live Tracking Field\n").unwrap();
    let source = format!(
        "+> math/*\n@filters := compute://filters/kernel{{:write(input/control), :write(input/camera), :write(input/measurement), :write(turn)}}\n\
         @filters/input/control <- [0.05<f32>; 1f32; 0f32]\n\
         @filters/input/camera <- [1f32; 1f32]\n\
         @filters/input/measurement <- [1f32; 0f32; 0f32]\n\
         @filters/turn <- 1\n\n{}",
        &shipped[start..end]
    );
    let document = SourceDocument::parse_resolved(
        "test://canonical-shipped-ekf-lowering",
        Revision(0),
        source,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(
        document.is_strictly_clean(),
        "{:#?}",
        document.snapshot().diagnostics
    );
    let mixed = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap()
        .compile_mixed_document(&document)
        .unwrap();
    let decoded = decode_program_artifact_sections(
        &encode_program_artifact_sections(&mixed.compute.artifact).unwrap(),
    )
    .unwrap();
    let inputs = BTreeMap::from([
        ("control".to_owned(), vec![0.05, 1.0, 0.0]),
        ("camera".to_owned(), vec![1.0, 1.0]),
        ("measurement".to_owned(), vec![1.0, 0.0, 0.0]),
    ]);
    let mut layouts = Vec::new();
    let mut accepted_states = Vec::new();
    for artifact in [&mixed.compute.artifact, &decoded] {
        let operations = artifact
            .nodes()
            .iter()
            .filter_map(|node| node.as_operation())
            .map(|node| {
                node.operation
                    .module_path
                    .iter()
                    .chain(std::iter::once(&node.operation.operation_name))
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join("/")
            })
            .collect::<Vec<_>>();
        assert!(
            operations
                .iter()
                .any(|operation| operation == "matrix/matmul")
        );
        assert!(
            operations
                .iter()
                .all(|operation| operation != "matrix/multiply")
        );
        let kernel = ComputeLowerer
            .compile_broadcast(artifact, &inputs)
            .unwrap_or_else(|error| panic!("{error:#?}"));
        let constraint_names = kernel
            .compute_program()
            .fixed_shape_storage()
            .unwrap()
            .constraints
            .iter()
            .map(|constraint| constraint.name.as_ref())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            constraint_names,
            std::collections::BTreeSet::from([
                "finite-state!",
                "matrix solve requires a finite nonsingular coefficient matrix",
                "positive-covariance!",
                "symmetric-covariance!",
            ])
        );
        layouts.push((
            format!("{:?}", kernel.compute_program().interface()),
            kernel.wgsl().to_owned(),
        ));
        let mut cpu = kernel.prepare_cpu(&inputs).unwrap();
        cpu.dispatch_turns(2).unwrap();
        accepted_states.push(cpu.state().clone());
        let before = cpu.state().clone();
        let mut invalid = inputs.clone();
        invalid.insert("measurement".to_owned(), vec![f32::NAN, 0.0, 1.0]);
        cpu.update_inputs(&invalid).unwrap();
        assert!(matches!(
            cpu.dispatch_turns(1),
            Err(mech_gpu::BatchedExecutionError::Integrity(_))
        ));
        assert_eq!(cpu.state(), &before);
    }
    assert_eq!(layouts[0], layouts[1]);
    assert_eq!(accepted_states[0], accepted_states[1]);
}
