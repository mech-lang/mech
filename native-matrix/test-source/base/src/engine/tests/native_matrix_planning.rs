//! Actual descriptors and NativeHost plans; no kernel/binder is executed here.
#![cfg(feature = "native-artifact")]

use mech_core::{
    ConstantStoreBuilder, DimensionExpr, DimensionLifetime, DimensionParameterDeclaration,
    DimensionParameterId, DimensionParameterOrigin, ExecutionTarget, FloatWidth, FunctionCatalog,
    FunctionCatalogBuilder, MemoryFootprintWitness, MemoryTargetKind, ResolvedValueDescriptor,
    Schema, SchemaBody, SchemaDraft, SchemaTableBuilder,
};
use mech_engine::{
    ArtifactBuildContext, OperationReference, ProgramArtifact, SourceInput, SourceNode,
    SourceNodeBody, SourceNodeOutput, SourceOutput, SourceProgram, SourceValue,
    compile_source_program_with_contracts,
    native_artifact::{NativeMatrixPlanError, lower_matrix_artifact},
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

// Count cumulative requested metadata allocations, a stronger bound than peak
// live bytes. TLS excludes allocations on other test threads; assertions run
// only after the probe has stopped. Payload arrays are never constructed.
struct MetadataAllocator;
thread_local! {
    static TRACKING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATED: Cell<u64> = const { Cell::new(0) };
}
fn record(bytes: usize) {
    TRACKING.with(|enabled| {
        if enabled.get() {
            ALLOCATED.with(|total| total.set(total.get().saturating_add(bytes as u64)));
        }
    });
}
unsafe impl GlobalAlloc for MetadataAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record(size);
        unsafe { System.realloc(pointer, layout, size) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: MetadataAllocator = MetadataAllocator;

fn observed<T>(f: impl FnOnce() -> T) -> (T, u64) {
    ALLOCATED.with(|value| value.set(0));
    TRACKING.with(|value| value.set(true));
    let result = f();
    TRACKING.with(|value| value.set(false));
    (result, ALLOCATED.with(Cell::get))
}

fn matrix(bounds: Option<[u64; 2]>, compound: bool) -> Schema {
    SchemaDraft {
        dimension_parameters: (0..2)
            .map(|index| DimensionParameterDeclaration {
                id: DimensionParameterId::new(index),
                origin: DimensionParameterOrigin::Explicit,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: bounds.map(|bounds| DimensionExpr::Constant(bounds[index as usize])),
            })
            .collect(),
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
            dimensions: Box::new([
                if compound {
                    DimensionExpr::Add(Box::new([
                        DimensionExpr::Parameter(DimensionParameterId::new(0)),
                        DimensionExpr::Constant(1),
                    ]))
                } else {
                    DimensionExpr::Parameter(DimensionParameterId::new(0))
                },
                DimensionExpr::Parameter(DimensionParameterId::new(1)),
            ]),
        },
    }
    .finalize()
    .unwrap()
}

fn artifacts(input: Schema, transpose: bool) -> [ProgramArtifact; 2] {
    let SchemaBody::Matrix {
        element,
        dimensions,
    } = input.body()
    else {
        unreachable!()
    };
    let output = if transpose {
        SchemaDraft {
            dimension_parameters: input
                .dimension_parameters()
                .iter()
                .enumerate()
                .map(|(index, parameter)| DimensionParameterDeclaration {
                    id: DimensionParameterId::new(index as u32),
                    origin: DimensionParameterOrigin::Explicit,
                    lifetime: parameter.lifetime(),
                    lower_bound: parameter.lower_bound().clone(),
                    upper_bound: parameter.upper_bound().cloned(),
                })
                .collect(),
            body: SchemaBody::Matrix {
                element: element.clone(),
                dimensions: Box::new([dimensions[1].clone(), dimensions[0].clone()]),
            },
        }
        .finalize()
        .unwrap()
    } else {
        matrix(None, false)
    };
    let mut builder = SchemaTableBuilder::new();
    let input = builder.insert(input).unwrap();
    let output = builder.insert(output).unwrap();
    let build = builder.finish().unwrap();
    let input = build.resolve(input).unwrap();
    let output = build.resolve(output).unwrap();
    let schemas = build.into_parts().0;
    let constants = ConstantStoreBuilder::new(&schemas)
        .finish()
        .unwrap()
        .into_parts()
        .0;
    let arity = if transpose { 1 } else { 2 };
    let name = if transpose { "transpose" } else { "matmul" };
    let graph = SourceProgram {
        inputs: (0..arity)
            .map(|ordinal| SourceInput {
                name: format!("input{ordinal}"),
                schema: input,
            })
            .collect(),
        nodes: Box::new([SourceNode {
            body: SourceNodeBody::Operation {
                operation: OperationReference {
                    module_path: Box::new(["matrix".into()]),
                    operation_name: name.into(),
                },
                requirement: None,
            },
            inputs: (0..arity).map(SourceValue::Input).collect(),
            outputs: Box::new([SourceNodeOutput::Derived { schema: output }]),
        }]),
        outputs: Box::new([SourceOutput {
            name: "result".into(),
            interactive_symbol: None,
            source: SourceValue::NodeOutput {
                node: 0,
                output_ordinal: 0,
            },
            schema: output,
        }]),
        ..SourceProgram::default()
    };
    let contract =
        mech_core::maintained_operation_contract(&format!("matrix/{name}"), arity as usize, true)
            .unwrap();
    let original = compile_source_program_with_contracts(
        &graph,
        &mut ArtifactBuildContext::new(&schemas, &constants),
        &[&contract],
    )
    .unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    [original, decoded]
}
fn catalog() -> FunctionCatalog {
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut builder).unwrap();
    builder.build().unwrap()
}
fn descriptor(schema: &Schema, extents: [u64; 2]) -> ResolvedValueDescriptor {
    let shape = mech_core::shape_for_resolved_extents(schema, &extents).unwrap();
    ResolvedValueDescriptor::from_schema(schema.clone(), shape).unwrap()
}

#[test]
fn actual_dynamic_descriptors_replan_native_geometry_including_zero_axes() {
    let catalog = catalog();
    for artifact in artifacts(matrix(None, false), true) {
        let plan = lower_matrix_artifact(&artifact, &catalog).unwrap();
        let call = &plan.calls()[0];
        let input = &plan.values()[plan.inputs()[0].value().get() as usize];
        for [rows, columns] in [[2, 3], [0, 4], [4, 0], [1, 1], [1000, 1000]] {
            let inputs = [descriptor(input.schema(), [rows, columns])];
            let (memory, allocated) = observed(|| plan.instantiate_call(0, &inputs));
            let memory = memory.unwrap();
            assert!(
                allocated <= call.planning_demand().temporary_bytes,
                "{allocated} > {:?}",
                call.planning_demand()
            );
            assert_eq!(memory.bound_call.target(), ExecutionTarget::Native);
            assert_eq!(memory.target.kind, MemoryTargetKind::NativeHost);
            assert!(memory.deferred_witnesses.is_empty());
            assert_eq!(memory.inputs[0].value.current_elements, rows * columns);
            assert_eq!(
                memory.outputs[0]
                    .descriptor
                    .current_extents()
                    .unwrap()
                    .as_ref(),
                &[columns, rows]
            );
            assert_eq!(memory.outputs[0].value.current_elements, rows * columns);
            assert!(
                matches!(memory.input_witnesses[0], MemoryFootprintWitness::Known(value)
                if value.logical_elements == rows * columns)
            );
            assert_eq!(
                memory.outputs[0].value.strides_bytes.as_ref(),
                &[8, columns * 8]
            );
        }
        assert!(plan.setup_demand().work > 0);
        assert!(plan.setup_demand().temporary_bytes >= call.planning_demand().temporary_bytes / 2);
    }
}

#[test]
fn semantic_upper_bounds_retain_real_reserved_capacity_and_padded_strides() {
    let catalog = catalog();
    for artifact in artifacts(matrix(Some([8, 9]), false), true) {
        let plan = lower_matrix_artifact(&artifact, &catalog).unwrap();
        let input = &plan.values()[plan.inputs()[0].value().get() as usize];
        let memory = plan
            .instantiate_call(0, &[descriptor(input.schema(), [2, 3])])
            .unwrap();
        assert_eq!(memory.inputs[0].value.capacity_bytes, 8 * 9 * 8);
        assert_eq!(memory.inputs[0].value.strides_bytes.as_ref(), &[8, 8 * 8]);
        assert_eq!(memory.inputs[0].value.current_address_span_bytes, 144);
        assert_eq!(memory.outputs[0].value.capacity_bytes, 8 * 9 * 8);
        assert_eq!(memory.outputs[0].value.strides_bytes.as_ref(), &[8, 9 * 8]);
        assert_eq!(memory.outputs[0].value.current_address_span_bytes, 96);
        // Runtime must separately admit compact leaf buffers and these actual
        // conservative core allocations. turn_peak_bytes counts live spans;
        // it is not a replacement for bounded capacity reservation.
        let reserved: u64 = memory
            .allocations
            .iter()
            .map(|allocation| allocation.capacity_bytes)
            .sum();
        assert!(reserved >= 3 * 8 * 9 * 8);
        let compact = plan.call_bounds(0, &[[2, 3]]).unwrap();
        assert_eq!(compact.output_bytes, 48);
        assert!(reserved > compact.output_bytes);
    }
}

#[test]
fn product_rejects_current_inner_mismatch_then_accepts_later_actual_descriptors() {
    let catalog = catalog();
    for artifact in artifacts(matrix(None, false), false) {
        let plan = lower_matrix_artifact(&artifact, &catalog).unwrap();
        let schemas = plan
            .inputs()
            .iter()
            .map(|input| plan.values()[input.value().get() as usize].schema())
            .collect::<Vec<_>>();
        let invalid = [
            descriptor(schemas[0], [2, 3]),
            descriptor(schemas[1], [4, 5]),
        ];
        assert!(matches!(
            plan.instantiate_call(0, &invalid),
            Err(NativeMatrixPlanError::Shape { .. })
        ));
        let valid = [
            descriptor(schemas[0], [2, 3]),
            descriptor(schemas[1], [3, 5]),
        ];
        let memory = plan.instantiate_call(0, &valid).unwrap();
        assert_eq!(
            memory.outputs[0]
                .descriptor
                .current_extents()
                .unwrap()
                .as_ref(),
            &[2, 5]
        );
        let other_schema = matrix(Some([9, 9]), false);
        let wrong = [descriptor(&other_schema, [2, 3]), valid[1].clone()];
        assert!(matches!(
            plan.instantiate_call(0, &wrong),
            Err(NativeMatrixPlanError::Planning { .. })
        ));
        assert!(plan.instantiate_call(0, &valid[..1]).is_err());
        assert_eq!(
            plan.call_bounds(0, &[[2, 3], [3, 5]]).unwrap().compute_work,
            64 + 10 * (32 + 16 * 3)
        );
        assert!(plan.call_bounds(0, &[[u64::MAX, 1], [1, 2]]).is_err());
    }
}

#[test]
fn compound_dimension_metadata_remains_a_separate_unsupported_native_obligation() {
    let catalog = catalog();
    for artifact in artifacts(matrix(None, true), true) {
        // The typed ordinary artifact and unchanged symbolic template remain
        // valid; the executable planning slice has its own bounded eligibility.
        assert_eq!(
            mech_engine::native_artifact::matrix_operation_templates(&artifact, &catalog)
                .unwrap()
                .len(),
            1
        );
        assert!(matches!(
            lower_matrix_artifact(&artifact, &catalog),
            Err(NativeMatrixPlanError::UnsupportedDimensionMetadata { .. })
        ));
    }
}

#[test]
fn activation_scoped_parameter_needs_real_binding_before_native_execution_support() {
    let schema = matrix(None, false);
    let activation = SchemaDraft {
        dimension_parameters: schema
            .dimension_parameters()
            .iter()
            .enumerate()
            .map(|(index, parameter)| DimensionParameterDeclaration {
                id: DimensionParameterId::new(index as u32),
                origin: DimensionParameterOrigin::Explicit,
                lifetime: DimensionLifetime::Activation,
                lower_bound: parameter.lower_bound().clone(),
                upper_bound: parameter.upper_bound().cloned(),
            })
            .collect(),
        body: schema.body().clone(),
    }
    .finalize()
    .unwrap();
    let catalog = catalog();
    for artifact in artifacts(activation, true) {
        assert_eq!(
            mech_engine::native_artifact::matrix_operation_templates(&artifact, &catalog)
                .unwrap()
                .len(),
            1
        );
        assert!(matches!(
            lower_matrix_artifact(&artifact, &catalog),
            Err(NativeMatrixPlanError::UnsupportedDimensionMetadata { .. })
        ));
    }
}
