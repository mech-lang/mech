#![cfg(feature = "full_compiler")]

use mech_core::{
    DimensionExpr, ExecutionTarget, FunctionInvocation, FunctionValueRepresentation, OperationId,
    ReactiveInstanceId, ResidentValueRef, ResolvedOperationDescriptor, RuntimeBindingSelector,
    RuntimeFunctionId, SchemaBody, SpecializedFunction, Value, ValueCell, ValueData,
    snapshot::{SequenceView, ValueDataDraft},
};
use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn compile(source: &str) -> (ProgramArtifact, usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x54455854), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let program = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    let output = program
        .document_outputs()
        .iter()
        .rev()
        .find(|output| output.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    (program.compile_artifact().unwrap(), output)
}
fn matrix_strings(value: &Value) -> Vec<&str> {
    let ValueData::Matrix(matrix) = value.data() else {
        panic!("{value:?}")
    };
    let SequenceView::String(values) = matrix.elements() else {
        panic!("{value:?}")
    };
    values.iter().map(|value| value.as_ref()).collect()
}
fn live(source: &str, inputs: &[&str], expected: impl Fn(usize, &Value)) {
    let (original, output) = compile(source);
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    for artifact in [&original, &decoded] {
        assert!(artifact.requirements().is_empty());
        let mut instance = activate(
            ReactiveInstanceId::new(0x54455854, 0),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        assert_eq!(instance.plan.inputs.len(), 1);
        let slot = instance.plan.inputs[0].slot;
        for (turn, text) in inputs.iter().enumerate() {
            let input = [(*text).to_owned()];
            instance
                .turn(&[CapturedSignalInput {
                    slot,
                    value: ResidentValueRef::String(&input),
                }])
                .unwrap();
            expected(turn, &instance.copied_output(output).unwrap());
        }
    }
}

#[test]
fn text_family_has_real_ordinary_exports_contracts_and_native_linkage() {
    let catalog = mech_stdlib::source_catalog();
    for (item, arity, matrix) in [("lines", 1, true), ("join", 2, false)] {
        let name = format!("text/{item}");
        let id = OperationId::from_name(&name);
        assert_eq!(catalog.module_export("text", item).unwrap().operation, id);
        assert!(catalog.specializer(id).is_some());
        assert!(catalog.resident_factory(&["text".into()], item).is_some());
        let entries = catalog
            .runtime_entries_for_binding(
                RuntimeBindingSelector::Operation(id),
                ExecutionTarget::DirectRuntime,
            )
            .collect::<Vec<_>>();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].operation_contract(id),
            Some(&mech_core::maintained_operation_contract(&name, arity, matrix).unwrap())
        );
        assert_eq!(
            entries[0].native_linkage.as_ref().unwrap().installer_path,
            format!("mech_engine::__mech_native::install_text_{item}")
        );
    }
}

#[test]
fn lines_preserve_unicode_blank_lines_and_lone_cr_with_live_cardinality_changes() {
    live(
        "+> text\ntext/lines(input<string>)\n",
        &["", "\n", "a\r\nb\n", "é\n\n🧑‍🚀\r\n", "x\ry"],
        |turn, value| {
            assert_eq!(
                matrix_strings(value),
                [
                    vec![],
                    vec![""],
                    vec!["a", "b"],
                    vec!["é", "", "🧑‍🚀"],
                    vec!["x\ry"]
                ][turn]
            );
            let body = value
                .schemas()
                .unwrap()
                .get(value.schema())
                .unwrap()
                .closed_body(value.shape())
                .unwrap();
            assert!(matches!(body, SchemaBody::Matrix {dimensions, ..}
                if dimensions.as_ref() == [DimensionExpr::Constant(1), DimensionExpr::Constant(matrix_strings(value).len() as u64)]));
        },
    );
}

#[test]
fn join_composes_with_dynamic_lines_and_uses_canonical_matrix_order() {
    live(
        "+> text\ntext/join(text/lines(input<string>), \"|\")\n",
        &["é\n\nA\r\n", "", "x\ny"],
        |turn, value| {
            assert!(
                matches!(value.data(), ValueData::String(actual) if actual.as_ref() == ["é||A", "", "x|y"][turn]),
                "{value:?}"
            );
        },
    );
    for (source, expected) in [
        (
            "+> text\ntext/join([\"A\" \"B\" \"C\"; \"D\" \"E\" \"F\"], \"/\")\n",
            "A/B/C/D/E/F",
        ),
        (
            "+> text\ntext/join([\"é\"; \"🧑‍🚀\"; \"\"], \"🌿\")\n",
            "é🌿🧑‍🚀🌿",
        ),
    ] {
        let (original, output) = compile(source);
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
        for artifact in [&original, &decoded] {
            let mut instance = activate(
                ReactiveInstanceId::new(0x54455855, 0),
                artifact,
                &mech_stdlib::source_catalog(),
                &ActivationFacts::default(),
            )
            .unwrap();
            instance.turn(&[]).unwrap();
            assert!(
                matches!(instance.copied_output(output).unwrap().data(), ValueData::String(text) if text.as_ref() == expected)
            );
        }
    }
}

#[test]
fn join_admits_length_changes_across_the_comparison_boundary_and_recovers() {
    let source = "+> text\ntext/join(text/lines(input<string>), \"\")\n";
    let (original, output) = compile(source);
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    for (ordinal, artifact) in [&original, &decoded].into_iter().enumerate() {
        for (case, length) in [65_535, 65_536, 65_537, 70_000].into_iter().enumerate() {
            let mut instance = activate(
                ReactiveInstanceId::new(0x54455857, (ordinal * 4 + case) as u32),
                artifact,
                &mech_stdlib::source_catalog(),
                &ActivationFacts::default(),
            )
            .unwrap();
            let slot = instance.plan.inputs[0].slot;
            let text = "x".repeat(length);
            instance
                .turn(&[CapturedSignalInput {
                    slot,
                    value: ResidentValueRef::String(&[text.clone()]),
                }])
                .unwrap_or_else(|error| panic!("length {length}: {error:?}"));
            assert!(
                matches!(instance.copied_output(output).unwrap().data(), ValueData::String(actual) if actual.as_ref() == text),
                "length {length}"
            );
        }
    }
}

#[test]
fn join_equal_length_comparison_denial_is_atomic_and_recoverable() {
    let source = "+> text\ntext/join(signal<[string]:1,1>, \"\")\n";
    let (original, output) = compile(source);
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    for (ordinal, artifact) in [&original, &decoded].into_iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x54455858, ordinal as u32),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        let slot = instance.plan.inputs[0].slot;
        let run = |instance: &mut mech_engine::resident::ReactiveInstance, text: String| {
            instance
                .turn(&[CapturedSignalInput {
                    slot,
                    value: ResidentValueRef::String(&[text]),
                }])
                .map(|_| ())
        };
        run(&mut instance, "seed".into()).unwrap();
        let accepted = "a".repeat(70_000);
        run(&mut instance, accepted.clone()).unwrap();
        let epoch = instance.published_epoch();
        let previous = instance.copied_output(output).unwrap();
        assert!(run(&mut instance, "b".repeat(70_000)).is_err());
        assert_eq!(instance.published_epoch(), epoch);
        assert_eq!(
            instance
                .copied_output(output)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            previous.canonical_data_draft().unwrap()
        );
        run(&mut instance, "recovered".into()).unwrap();
        assert!(
            matches!(instance.copied_output(output).unwrap().data(), ValueData::String(actual) if actual.as_ref() == "recovered")
        );
    }
}

fn text(value: &str) -> ValueCell {
    ValueCell::from_schema_data(SchemaBody::String, ValueDataDraft::String(value.into())).unwrap()
}
fn managed(name: &str, output: &ValueCell, inputs: &[ValueCell]) -> SpecializedFunction {
    let catalog = mech_stdlib::source_catalog();
    let id = OperationId::from_name(name);
    let entry = catalog
        .runtime_entry(RuntimeFunctionId::from_name(name))
        .unwrap();
    let invocation = match inputs {
        [input] => FunctionInvocation::unary(output.clone(), input.clone()),
        [first, second] => {
            FunctionInvocation::binary(output.clone(), first.clone(), second.clone())
        }
        _ => unreachable!(),
    };
    let parts = entry
        .bind_resolved_invocation(id, ExecutionTarget::DirectRuntime, invocation)
        .unwrap();
    SpecializedFunction::syntax_directed(
        parts,
        ResolvedOperationDescriptor::from_name(name, entry.operation_contract(id).unwrap().clone())
            .unwrap(),
        entry.id,
        ExecutionTarget::DirectRuntime,
        entry.implementation_memory_class(),
    )
    .unwrap()
}

#[test]
fn managed_lines_adopt_actual_turn_shape_and_feed_the_same_prebound_join() {
    let template =
        ValueCell::dynamic_matrix(SchemaBody::String, Box::new([1, 0]), Box::new([])).unwrap();
    let lines = ValueCell::allocate_for_descriptor(
        &template.resolved_descriptor().unwrap(),
        FunctionValueRepresentation::AnyValue,
    )
    .unwrap();
    let domain = lines.memory_domain().unwrap();
    let input = text("").import_owned_in(&domain).unwrap();
    let separator = text("|").import_owned_in(&domain).unwrap();
    let joined = text("").import_owned_in(&domain).unwrap();
    let split = managed("text/lines", &lines, &[input.clone()]);
    let join = managed("text/join", &joined, &[lines.clone(), separator]);
    for (value, expected) in [("", ""), ("é\r\n\nA\n", "é||A"), ("X\nY", "X|Y"), ("", "")] {
        input.replace(&text(value).snapshot().unwrap()).unwrap();
        split.instance().solve_result().unwrap();
        assert_eq!(
            matrix_strings(&lines.snapshot().unwrap()),
            value.lines().collect::<Vec<_>>()
        );
        join.instance().solve_result().unwrap();
        assert!(
            matches!(joined.snapshot().unwrap().data(), ValueData::String(value) if value.as_ref() == expected)
        );
    }
    for (index, point) in [
        mech_core::MemoryFailurePoint::Admission,
        mech_core::MemoryFailurePoint::HostAllocation,
    ]
    .into_iter()
    .enumerate()
    {
        let candidate = (0..4 * (index + 1))
            .map(|line| format!("line-{line}\n"))
            .collect::<String>();
        input
            .replace(&text(&candidate).snapshot().unwrap())
            .unwrap();
        let before = lines.snapshot().unwrap();
        let version = lines.published_version();
        let ledger = domain.ledger();
        domain.inject_failure_after(point, 0).unwrap();
        assert!(split.instance().solve_result().is_err());
        assert_eq!(lines.published_version(), version);
        assert_eq!(domain.ledger(), ledger);
        let after = lines.snapshot().unwrap();
        assert!(
            before
                .snapshot_eq(
                    &before.schemas().unwrap(),
                    &after,
                    &after.schemas().unwrap()
                )
                .unwrap()
        );
        split.instance().solve_result().unwrap();
        assert_eq!(
            matrix_strings(&lines.snapshot().unwrap()),
            candidate.lines().collect::<Vec<_>>()
        );
    }
}

#[test]
fn public_type_schemes_reject_non_string_lines_and_join_inputs() {
    for source in [
        "+> text\ntext/lines(3u64)\n",
        "+> text\ntext/join([1u64 2u64], \"|\")\n",
        "+> text\ntext/join(\"AB\", \"|\")\n",
    ] {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x54455856), Revision(1), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean());
        assert!(
            CanonicalSourceFrontend
                .compile_document_with_catalog(
                    &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                    mech_stdlib::source_catalog()
                )
                .is_err(),
            "{source}"
        );
    }
}

#[test]
fn managed_text_shares_imported_large_input_arena_with_small_output_arena() {
    use mech_core::{
        CurrentMemoryFootprint, ExecutionDemandPlanningRequest, KernelMemoryFrame, MResult,
        MechExecutionServices, MechFunction, MechFunctionCompiler, MechFunctionImpl, MemoryDomain,
        OperationContractDeclaration, PayloadOutputPlanPolicy, PlannedExecutionDemand,
        PlannedSlotKind, ReactiveSolveStatus, SchemaDraft, SchemaTable, SchemaTableBuilder,
        ShapeInstance, ValueDraft, snapshot::SnapshotValidationContext,
    };
    use std::{cell::Cell, rc::Rc, sync::Arc};

    // Observe the actual leased input before delegating to the unchanged
    // maintained factory. No substitute implementation performs the text work.
    struct SharedInputProbe {
        inner: Box<dyn MechFunction>,
        input: ValueCell,
        retained: Value,
        owner: Arc<SchemaTable>,
        observations: Rc<Cell<usize>>,
    }
    impl MechFunctionImpl for SharedInputProbe {
        fn planned_output_shapes(&self) -> MResult<Option<Box<[ShapeInstance]>>> {
            self.inner.planned_output_shapes()
        }
        fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
            self.inner.planned_output_footprints()
        }
        fn planned_output_footprints_with_request(
            &self,
            request: &ExecutionDemandPlanningRequest<'_>,
        ) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
            self.inner.planned_output_footprints_with_request(request)
        }
        fn planned_execution_demand(
            &self,
            request: &ExecutionDemandPlanningRequest<'_>,
        ) -> MResult<PlannedExecutionDemand> {
            self.inner.planned_execution_demand(request)
        }
        fn payload_output_plan_policy(&self) -> PayloadOutputPlanPolicy {
            self.inner.payload_output_plan_policy()
        }
        fn semantic_operation_name(&self) -> Option<&str> {
            self.inner.semantic_operation_name()
        }
        fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
            self.inner.semantic_operation_contract()
        }
        fn solve_managed(
            &self,
            frame: &mut KernelMemoryFrame<'_>,
            services: &mut dyn MechExecutionServices,
        ) -> MResult<ReactiveSolveStatus> {
            let value = frame.snapshot_input_cell(&self.input, 0)?;
            assert!(value.shares_frozen_storage(&self.retained));
            assert!(Arc::ptr_eq(&self.owner, &value.schemas().unwrap()));
            assert_eq!(
                frame.input_cell_footprint(&self.input, 0)?.schema_bytes,
                self.owner.clone_allocation_bound_bytes().unwrap()
            );
            self.observations.set(self.observations.get() + 1);
            self.inner.solve_managed(frame, services)
        }
        fn to_string(&self) -> String {
            self.inner.to_string()
        }
    }
    impl MechFunctionCompiler for SharedInputProbe {
        fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
            self.inner.compiler_owned_value_cells()
        }
        fn reserve_bytecode_registers(
            &self,
            context: &mut dyn mech_core::BytecodeCompilerContext,
        ) -> MResult<()> {
            self.inner.reserve_bytecode_registers(context)
        }
        fn compile(
            &self,
            context: &mut dyn mech_core::BytecodeCompilerContext,
        ) -> MResult<mech_core::Register> {
            self.inner.compile(context)
        }
    }

    let mut builder = SchemaTableBuilder::new();
    let mut add = |body| {
        builder
            .insert(
                SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body,
                }
                .finalize()
                .unwrap(),
            )
            .unwrap()
    };
    let scalar = add(SchemaBody::String);
    let matrix = add(SchemaBody::Matrix {
        element: Box::new(SchemaBody::String),
        dimensions: Box::new([DimensionExpr::Constant(1), DimensionExpr::Constant(3)]),
    });
    for columns in 0..512 {
        add(SchemaBody::Matrix {
            element: Box::new(SchemaBody::Bool),
            dimensions: Box::new([DimensionExpr::Constant(1), DimensionExpr::Constant(columns)]),
        });
    }
    let build = builder.finish().unwrap();
    let scalar = build.resolve(scalar).unwrap();
    let matrix = build.resolve(matrix).unwrap();
    let (schemas, _) = build.into_parts();
    assert_eq!(schemas.len(), 514);
    let catalog = mech_stdlib::source_catalog();
    for (name, schema, data) in [
        (
            "text/lines",
            scalar,
            ValueDataDraft::String("é\n\n🧑‍🚀\n".into()),
        ),
        (
            "text/join",
            matrix,
            ValueDataDraft::Matrix(
                ["é", "", "🧑‍🚀"]
                    .into_iter()
                    .map(|value| ValueDataDraft::String(value.into()))
                    .collect(),
            ),
        ),
    ] {
        let domain = MemoryDomain::new().unwrap();
        let imported = ValueDraft {
            schema,
            shape_values: Box::new([]),
            data,
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        let input = ValueCell::from_snapshot_in(&domain, imported).unwrap();
        assert!(input.requires_canonical_output_builder().unwrap());
        let retained = input.snapshot().unwrap();
        let owner = retained.schemas().unwrap();
        let output = if name == "text/lines" {
            ValueCell::dynamic_matrix(SchemaBody::String, Box::new([1, 0]), Box::new([])).unwrap()
        } else {
            text("")
        }
        .import_owned_in(&domain)
        .unwrap();
        let small_owner = output.snapshot().unwrap().schemas().unwrap();
        assert!(owner.len() > 500);
        assert!(small_owner.len() <= 2);
        assert!(
            owner.clone_allocation_bound_bytes().unwrap()
                > small_owner.clone_allocation_bound_bytes().unwrap() * 32
        );
        assert!(!Arc::ptr_eq(&owner, &small_owner));
        let entry = catalog
            .runtime_entry(RuntimeFunctionId::from_name(name))
            .unwrap();
        let invocation = if name == "text/lines" {
            FunctionInvocation::unary(output.clone(), input.clone())
        } else {
            FunctionInvocation::binary(
                output.clone(),
                input.clone(),
                text("|").import_owned_in(&domain).unwrap(),
            )
        };
        let (inner, invocation) = entry
            .bind_resolved_invocation(
                OperationId::from_name(name),
                ExecutionTarget::DirectRuntime,
                invocation,
            )
            .unwrap();
        let observations = Rc::new(Cell::new(0));
        let function = SpecializedFunction::syntax_directed(
            (
                Box::new(SharedInputProbe {
                    inner,
                    input: input.clone(),
                    retained,
                    owner: owner.clone(),
                    observations: observations.clone(),
                }),
                invocation,
            ),
            ResolvedOperationDescriptor::from_name(
                name,
                entry
                    .operation_contract(OperationId::from_name(name))
                    .unwrap()
                    .clone(),
            )
            .unwrap(),
            entry.id,
            ExecutionTarget::DirectRuntime,
            entry.implementation_memory_class(),
        )
        .unwrap();
        assert_eq!(
            function.instance().memory_plan().input_storage[0].slot,
            PlannedSlotKind::CanonicalValueHandle
        );
        for _ in 0..2 {
            function.instance().solve_result().unwrap();
            let value = output.snapshot().unwrap();
            if name == "text/lines" {
                assert_eq!(matrix_strings(&value), vec!["é", "", "🧑‍🚀"]);
            } else {
                assert!(
                    matches!(value.data(), ValueData::String(value) if value.as_ref() == "é||🧑‍🚀")
                );
            }
            assert!(value.schemas().unwrap().len() <= 2);
            assert!(!Arc::ptr_eq(&owner, &value.schemas().unwrap()));
            assert!(Arc::ptr_eq(
                &owner,
                &input.snapshot().unwrap().schemas().unwrap()
            ));
        }
        assert_eq!(observations.get(), 2);
    }
}
