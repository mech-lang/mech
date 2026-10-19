#![cfg(all(feature = "full_compiler", feature = "native-artifact"))]
use mech_core::{
    BoundResidentKernel, FunctionCatalog, FunctionCatalogBuilder, NodeId, ResidentKernelBindError,
    ResidentKernelBindRequest,
};
use mech_engine::native_artifact::{
    NativeArtifactLoweringError, NativeCollectionLexicalSource, NativeCollectionPrimitive,
    bind_native_collection_lexical, bind_native_collection_operation,
    bind_native_program_library_call, native_collection_lexical_template,
    native_collection_operation_template, prepare_native_program_library_call,
};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

// Opt-in allocator observations, not timing or a smaller admission witness.
#[path = "support/native_optional_plan_cache.rs"]
mod native_optional_plan_cache;

#[path = "support/native_exact_facts_plan_cache.rs"]
mod native_exact_facts_plan_cache;
#[path = "support/native_metadata_allocations.rs"]
mod native_metadata_allocations;
#[path = "support/native_snapshot_string_index.rs"]
mod native_snapshot_string_index;

const SOURCE: &str = r#"+> text
+> sequence
+> number
+> map
words := text/lines(input<string>)
joined := text/join(words, ",")
size := sequence/length(joined)
formatted := number/format(size, 0u64)
counts := map/histogram(words)
keys := map/keys(counts)
values := map/values(counts)
(formatted, keys, values)
"#;

// Every maintained leaf is genuinely inside the canonical counted function,
// not a root call relabelled with a lexical owner.
const LEXICAL_ALL_SOURCE: &str = r#"+> text
+> sequence
+> number
+> map
step(x<string>) => (next<string>) :=
  words := text/lines(x)
  joined := text/join(words, ",")
  size := sequence/length(joined)
  counts := map/histogram(words)
  keys := map/keys(counts)
  values := map/values(counts)
  formatted := number/format(size + sequence/length(values), 0u64)
  next := text/join(keys, formatted).
seed<string> |2> step
"#;

const RANGE_SELECTION_SOURCE: &str = r#"exclusive := start<u64>..end<u64>
inclusive := start..=end
stepped := start..increment<u64>..end
stepped-inclusive := start..increment..=end
text := input<string>
at := text[position<u64>]
gather := text[indices<[u64]>]
all := text[:]
converted<[f64]> := numbers<[u64]>
"#;

const LEXICAL_RANGE_SELECTION_SOURCE: &str = r#"lower := start<u64>
upper := end<u64>
stride := increment<u64>
offset := position<u64>
selectors := indices<[u64]>
values := numbers<[u64]>
step(s<string>) => (next<string>) :=
  exclusive := lower..upper
  inclusive := lower..=upper
  stepped := lower..stride..upper
  stepped-inclusive := lower..stride..=upper
  at := s[offset]
  gather := s[selectors]
  converted<[f64]> := values
  next := s[:].
seed<string> |2> step
"#;

#[test]
fn mixed_native_control_lowering_preserves_all_seven_library_families_and_scalar_work() {
    use mech_engine::native_artifact::{
        NativeIterationAction, NativeMatchAction, NativeScalarOp, lower_native_program_artifact,
    };
    let source = SOURCE.replace("(formatted, keys, values)\n", "more := size + 1u64\n");
    for artifact in artifacts(&source) {
        let plan =
            lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        plan.validate_current_visits().unwrap();
        assert_eq!(plan.program_revision(), artifact.revision());
        let mut names = Vec::new();
        let mut scalar_add = false;
        for block in plan.blocks() {
            match block.action() {
                NativeIterationAction::LibraryCall {
                    template,
                    operands,
                    output,
                } => {
                    assert_eq!(template.origin(), block.origin());
                    assert_eq!(template.program_revision(), artifact.revision());
                    assert!(template.root().is_some());
                    assert!(template.lexical().is_none());
                    assert_eq!(operands.len(), template.inputs().len());
                    assert_eq!(
                        plan.values()[output.get() as usize].definition(),
                        template.output()
                    );
                    names.push(template.primitive().name());
                }
                NativeIterationAction::Scalar(NativeMatchAction::Primitive {
                    op: NativeScalarOp::U64Add,
                    ..
                }) => scalar_add = true,
                _ => {}
            }
        }
        names.sort();
        assert_eq!(
            names,
            [
                "map/histogram",
                "map/keys",
                "map/values",
                "number/format",
                "sequence/length",
                "text/join",
                "text/lines"
            ]
        );
        assert!(scalar_add);
        assert!(plan.values().iter().any(|ty| ty.scalar().is_none()));
        assert!(
            mech_engine::native_artifact::lower_scalar_iteration_artifact(
                &artifact,
                &mech_stdlib::source_catalog()
            )
            .is_err()
        );
    }
}

#[test]
fn mixed_native_control_lowering_owns_managed_loop_carriers_and_lexical_leaf_calls() {
    use mech_engine::native_artifact::{
        NativeIterationAction, NativeIterationValueScope, lower_native_program_artifact,
    };
    let source = "+> sequence\n+> number\nstep(x<string>) => (next<string>) := next := (choose? | true => number/format(sequence/length(x), 0u64) | false => x).\nchoose := flag<bool>\nseed<string> |3> step\n";
    for artifact in artifacts(source) {
        let plan =
            lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        plan.validate_current_visits().unwrap();
        assert_eq!(plan.iterations().len(), 1);
        let iteration = plan.iterations()[0]
            .as_iteration()
            .expect("the declared literal loop, not a collection generator");
        assert_eq!(iteration.count(), 3);
        assert!(
            plan.values()[iteration.carrier().get() as usize]
                .scalar()
                .is_none()
        );
        assert_eq!(
            plan.value_scopes()[iteration.carrier().get() as usize],
            NativeIterationValueScope::Carrier { iteration: 0 }
        );
        let calls = plan
            .blocks()
            .iter()
            .filter_map(|block| {
                let NativeIterationAction::LibraryCall { template, .. } = block.action() else {
                    return None;
                };
                assert_eq!(block.iteration(), Some(0));
                assert_eq!(template.origin(), block.origin());
                let lexical = template.lexical().expect("actual lexical canonical site");
                assert_eq!(Some(lexical.block().0), block.origin().block());
                assert_eq!(Some(lexical.local()), block.origin().local());
                assert!(template.root().is_none());
                Some(template.primitive().name())
            })
            .collect::<Vec<_>>();
        assert_eq!(calls, ["sequence/length", "number/format"]);
    }
}

#[test]
fn mixed_native_control_lowering_never_uses_replacement_factories_as_authority() {
    use mech_engine::native_artifact::lower_native_program_artifact;
    let maintained = mech_stdlib::source_catalog();
    let original = maintained
        .resident_factory(&["sequence".into()], "length")
        .unwrap();
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_resident_factory_entry(original.clone().with_factory(replacement))
        .unwrap();
    let replaced = builder.build().unwrap();
    for artifact in artifacts("+> sequence\nsequence/length(input<string>)\n") {
        assert!(matches!(
            lower_native_program_artifact(&artifact, &replaced),
            Err(NativeArtifactLoweringError::MissingSemanticWitness { .. })
        ));
        lower_native_program_artifact(&artifact, &maintained)
            .unwrap()
            .validate_current_visits()
            .unwrap();
    }
    assert_eq!(BINDS.load(Ordering::SeqCst), 0);
}

#[test]
fn retained_native_cfg_library_templates_bind_only_the_current_selected_site() {
    use mech_engine::native_artifact::{NativeIterationAction, lower_native_program_artifact};
    let maintained = mech_stdlib::source_catalog();
    let cloned = FunctionCatalogBuilder::from_catalog(&maintained)
        .build()
        .unwrap();
    let original = maintained
        .resident_factory(&["sequence".into()], "length")
        .unwrap();
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_resident_factory_entry(original.clone().with_factory(replacement))
        .unwrap();
    let replaced = builder.build().unwrap();
    let limits = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    let root = SOURCE.replace("(formatted, keys, values)\n", "more := size + 1u64\n");
    for source in [
        root.as_str(),
        LEXICAL_ALL_SOURCE,
        RANGE_SELECTION_SOURCE,
        LEXICAL_RANGE_SELECTION_SOURCE,
    ] {
        for artifact in artifacts(source) {
            let plan = lower_native_program_artifact(&artifact, &maintained).unwrap();
            let mut families = std::collections::BTreeSet::new();
            let mut primitives = std::collections::BTreeSet::new();
            for block in plan.blocks() {
                let NativeIterationAction::LibraryCall { template, .. } = block.action() else {
                    continue;
                };
                families.insert(template.primitive().name());
                primitives.insert(format!("{:?}", template.primitive()));
                let parameters = template
                    .inputs()
                    .iter()
                    .map(|schema| vec![2; schema.dimension_parameters().len()])
                    .collect::<Vec<_>>();
                let parameters = parameters.iter().map(Vec::as_slice).collect::<Vec<_>>();
                let output = vec![0; template.output().dimension_parameters().len()];
                for catalog in [maintained.as_ref(), &cloned] {
                    let prepared =
                        prepare_native_program_library_call(&artifact, template, catalog).unwrap();
                    assert_eq!(prepared.template(), template);
                    assert!(prepared.binding_metadata_bytes() > 0);
                    let call = bind_native_program_library_call(
                        &artifact,
                        template,
                        catalog,
                        &parameters,
                        &output,
                        limits,
                    )
                    .unwrap();
                    assert_eq!(call.template(), template);
                    assert_eq!(call.template().origin(), block.origin());
                    assert_eq!(call.memory_plan().bound_call, *call.bound_call());
                    assert_eq!(
                        call.memory_plan().target.kind,
                        mech_core::MemoryTargetKind::NativeHost
                    );
                    for extent in [0, 1, 17, 127] {
                        let shapes = template
                            .inputs()
                            .iter()
                            .map(|schema| vec![extent; schema.dimension_parameters().len()])
                            .collect::<Vec<_>>();
                        let shapes = shapes.iter().map(Vec::as_slice).collect::<Vec<_>>();
                        let result = vec![extent; template.output().dimension_parameters().len()];
                        let live = bind_native_program_library_call(
                            &artifact, template, catalog, &shapes, &result, limits,
                        )
                        .unwrap();
                        let ports =
                            live.memory_plan().inputs.len() + live.memory_plan().outputs.len();
                        let entries = live.memory_plan().allocations.len()
                            + live.memory_plan().deferred_witnesses.len()
                            + live.memory_plan().transactions.len();
                        assert!(
                            entries <= ports * 3 + 16,
                            "sealed-family metadata bound: {} extent{extent}",
                            template.primitive().name()
                        );
                    }
                }
                assert!(
                    bind_native_program_library_call(
                        &artifact,
                        template,
                        &maintained,
                        &parameters,
                        &output,
                        mech_core::MemoryBudgetLimits::default()
                    )
                    .is_err()
                );
                if template.primitive() == NativeCollectionPrimitive::SequenceLength {
                    assert!(matches!(
                        prepare_native_program_library_call(&artifact, template, &replaced),
                        Err(NativeArtifactLoweringError::MissingSemanticWitness { .. })
                    ));
                    assert!(matches!(
                        bind_native_program_library_call(
                            &artifact,
                            template,
                            &replaced,
                            &parameters,
                            &output,
                            limits
                        ),
                        Err(NativeArtifactLoweringError::MissingSemanticWitness { .. })
                    ));
                }
                let key = template.selected_key();
                let original = maintained
                    .resident_factory(&key.module_path, &key.operation_name)
                    .unwrap();
                let mut replacement_catalog = FunctionCatalogBuilder::new();
                replacement_catalog
                    .insert_resident_factory_entry(original.clone().with_factory(replacement))
                    .unwrap();
                let replacement_catalog = replacement_catalog.build().unwrap();
                assert!(
                    matches!(
                        prepare_native_program_library_call(
                            &artifact,
                            template,
                            &replacement_catalog
                        ),
                        Err(NativeArtifactLoweringError::MissingSemanticWitness { .. })
                    ),
                    "replacement gained {} authority",
                    template.primitive().name()
                );
            }
            assert_eq!(families.len(), 7, "{source}");
            // Whole-String [:] is already an identity in the canonical
            // frontend (prepare_selection_schema), not an eighth call. Keep
            // that source case and require all seven actual selected families.
            assert_eq!(primitives.len(), 7, "{source}: {primitives:?}");
        }
    }
    assert_eq!(BINDS.load(Ordering::SeqCst), 0, "replacement must not bind");
}

#[test]
fn matrix_and_map_selection_keep_private_factory_provenance_and_shallow_plan_bounds() {
    use mech_engine::native_artifact::{NativeIterationAction, lower_native_program_artifact};
    let catalog = mech_stdlib::source_catalog();
    let source = "xs := values<[u64]>\nh := entries<{string:u64}>\nkey := query<string>\npos := position<u64>\nroot := xs[pos] + h{key}\nstep(x<u64>) => (out<u64>) := out := x + xs[pos] + h{key}.\nseed<u64> |2> step\n";
    let limits = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    for artifact in artifacts(source) {
        let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
        let mut selected = std::collections::BTreeSet::new();
        for block in plan.blocks() {
            let NativeIterationAction::LibraryCall { template, .. } = block.action() else {
                continue;
            };
            assert!(matches!(
                template.primitive(),
                NativeCollectionPrimitive::MatrixScalarAccess
                    | NativeCollectionPrimitive::MapKeyAccess
            ));
            selected.insert((
                template.root().is_some(),
                format!("{:?}", template.primitive()),
            ));
            let prepared =
                prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
            assert_eq!(prepared.template(), template);
            for extent in [0, 1, 17, 127] {
                let parameters = template
                    .inputs()
                    .iter()
                    .map(|s| vec![extent; s.dimension_parameters().len()])
                    .collect::<Vec<_>>();
                let parameters = parameters.iter().map(Vec::as_slice).collect::<Vec<_>>();
                let call = bind_native_program_library_call(
                    &artifact,
                    template,
                    &catalog,
                    &parameters,
                    &[],
                    limits,
                )
                .unwrap();
                let plan = call.memory_plan();
                let entries = plan.allocations.len()
                    + plan.deferred_witnesses.len()
                    + plan.transactions.len();
                assert!(entries <= (plan.inputs.len() + plan.outputs.len()) * 3 + 16);
            }
            let key = template.selected_key();
            let entry = catalog
                .resident_factory(&key.module_path, &key.operation_name)
                .unwrap();
            let mut replaced = FunctionCatalogBuilder::new();
            replaced
                .insert_resident_factory_entry(entry.clone().with_factory(replacement))
                .unwrap();
            assert!(matches!(
                prepare_native_program_library_call(
                    &artifact,
                    template,
                    &replaced.build().unwrap()
                ),
                Err(NativeArtifactLoweringError::MissingSemanticWitness { .. })
            ));
        }
        assert_eq!(
            selected,
            std::collections::BTreeSet::from([
                (true, "MatrixScalarAccess".into()),
                (false, "MatrixScalarAccess".into()),
                (true, "MapKeyAccess".into()),
                (false, "MapKeyAccess".into()),
            ])
        );
    }
}

#[test]
fn constructors_and_set_conversion_keep_selected_factories_and_bounded_plan_metadata() {
    use mech_engine::native_artifact::{NativeIterationAction, lower_native_program_artifact};
    let catalog = mech_stdlib::source_catalog();
    let limits = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    let mut sources = vec![
        "keys<{string}> := words<[string]>\nquery<string> ∈ keys\nquery ∉ keys\n".to_owned(),
        "(base<u64> + 1u64, label<string>, flag<bool>, base * 2u64)\n".to_owned(),
    ];
    for count in [1, 2, 8, 64] {
        let entries = (0..count)
            .map(|i| format!("\"key{i}\": base<u64> + {i}u64"))
            .collect::<Vec<_>>()
            .join(", ");
        sources.push(format!(
            "mapping := {{{entries}}}\nmapping{{query<string>}}\n"
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    for source in sources {
        for artifact in artifacts(&source) {
            let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
            for block in plan.blocks() {
                let NativeIterationAction::LibraryCall { template, .. } = block.action() else {
                    continue;
                };
                seen.insert(format!("{:?}", template.primitive()));
                let prepared =
                    prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
                assert_eq!(prepared.template(), template);
                for extent in [0, 1, 17, 127] {
                    let parameters = template
                        .inputs()
                        .iter()
                        .map(|s| vec![extent; s.dimension_parameters().len()])
                        .collect::<Vec<_>>();
                    let parameters = parameters.iter().map(Vec::as_slice).collect::<Vec<_>>();
                    let output = vec![extent; template.output().dimension_parameters().len()];
                    let call = bind_native_program_library_call(
                        &artifact,
                        template,
                        &catalog,
                        &parameters,
                        &output,
                        limits,
                    )
                    .unwrap();
                    let memory = call.memory_plan();
                    let entries = memory.allocations.len()
                        + memory.deferred_witnesses.len()
                        + memory.transactions.len();
                    assert!(
                        entries <= (memory.inputs.len() + memory.outputs.len()) * 3 + 16,
                        "{:?} extent {extent}",
                        template.primitive()
                    );
                }
                let key = template.selected_key();
                let entry = catalog
                    .resident_factory(&key.module_path, &key.operation_name)
                    .unwrap();
                let mut replaced = FunctionCatalogBuilder::new();
                replaced
                    .insert_resident_factory_entry(entry.clone().with_factory(replacement))
                    .unwrap();
                assert!(matches!(
                    prepare_native_program_library_call(
                        &artifact,
                        template,
                        &replaced.build().unwrap()
                    ),
                    Err(NativeArtifactLoweringError::MissingSemanticWitness { .. })
                ));
            }
        }
    }
    for expected in [
        "MatrixToSetConversion",
        "SetContains",
        "SetNotContains",
        "CompositePack(4)",
        "CompositePack(16)",
        "CompositePack(128)",
    ] {
        assert!(seen.contains(expected), "{expected}: {seen:?}");
    }
}

#[cfg(feature = "sequence_sort")]
#[test]
fn sort_and_reductions_keep_actual_private_factories_at_root_and_lexical_sites() {
    use mech_engine::native_artifact::{NativeIterationAction, lower_native_program_artifact};
    let catalog = mech_stdlib::source_catalog();
    let limits = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    let root = "+> sequence\n+> stats\nxs := values<[u64]>\nks := keys<[string]>\norder := sequence/sort-indices(xs, \"desc\", ks, \"asc\")\ncolumns := stats/sum/column(xs)\nrows := stats/sum/row(xs)\n";
    let lexical = "+> sequence\n+> stats\nxs := values<[u64]>\nks := keys<[string]>\nstep(seed<u64>) => (out<u64>) :=\n  order := sequence/sort-indices(xs, \"desc\", ks, \"asc\")\n  columns := stats/sum/column(xs)\n  rows := stats/sum/row(xs)\n  out := seed + sequence/length(order) + sequence/length(columns) + sequence/length(rows).\nseed<u64> |2> step\n";
    for source in [root, lexical] {
        for artifact in artifacts(source) {
            let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
            plan.validate_current_visits().unwrap();
            let mut seen = std::collections::BTreeSet::new();
            for block in plan.blocks() {
                let NativeIterationAction::LibraryCall { template, .. } = block.action() else {
                    continue;
                };
                if !matches!(
                    template.primitive(),
                    NativeCollectionPrimitive::SortIndices
                        | NativeCollectionPrimitive::SumColumns
                        | NativeCollectionPrimitive::SumRows
                ) {
                    continue;
                }
                seen.insert(template.primitive().name());
                assert_eq!(template.root().is_some(), source == root);
                assert_eq!(template.lexical().is_some(), source == lexical);
                prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
                for extent in [0, 1, 17, 127] {
                    let shapes = template
                        .inputs()
                        .iter()
                        .map(|s| vec![extent; s.dimension_parameters().len()])
                        .collect::<Vec<_>>();
                    let refs = shapes.iter().map(Vec::as_slice).collect::<Vec<_>>();
                    let result = vec![extent; template.output().dimension_parameters().len()];
                    let call = bind_native_program_library_call(
                        &artifact, template, &catalog, &refs, &result, limits,
                    )
                    .unwrap();
                    let memory = call.memory_plan();
                    assert!(
                        memory.allocations.len()
                            + memory.deferred_witnesses.len()
                            + memory.transactions.len()
                            <= (memory.inputs.len() + memory.outputs.len()) * 3 + 16
                    );
                }
                let key = template.selected_key();
                let entry = catalog
                    .resident_factory(&key.module_path, &key.operation_name)
                    .unwrap();
                for altered in [
                    entry.clone().with_factory(replacement),
                    entry.clone().with_implementation_memory(
                        mech_core::ImplementationMemoryClass::CanonicalFinalize,
                    ),
                ] {
                    let mut replaced = FunctionCatalogBuilder::new();
                    replaced.insert_resident_factory_entry(altered).unwrap();
                    assert!(matches!(
                        prepare_native_program_library_call(
                            &artifact,
                            template,
                            &replaced.build().unwrap()
                        ),
                        Err(NativeArtifactLoweringError::MissingSemanticWitness { .. })
                    ));
                }
            }
            assert_eq!(
                seen,
                std::collections::BTreeSet::from([
                    "sequence/sort-indices",
                    "stats/sum/column",
                    "stats/sum/row"
                ])
            );
        }
    }
}

#[cfg(feature = "sequence_sort")]
#[test]
fn prepared_sort_directions_and_numeric_sums_use_canonical_transport_and_recover() {
    use mech_core::{ValueData, ValueDataDraft, ValueDraft, snapshot::SequenceView};
    use mech_engine::native_artifact::NativeLibraryWorkBudget;
    let source = "+> sequence\n+> stats\norder := sequence/sort-indices(counts<[u64]>, first-order<string>, keys<[string]>, second-order<string>)\ncolumns := stats/sum/column(matrix<[u64]:2,3>)\nrows := stats/sum/row(matrix)\n";
    for artifact in artifacts(source) {
        let memory = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
            .unwrap()
            .limits;
        limits.max_compute_work = Some(128 * 1024 * 1024);
        limits.max_temporary_bytes = Some(128 * 1024 * 1024);
        let matrix =
            |name: &str, ordinal: usize, rows: u64, columns: u64, data: Vec<ValueDataDraft>| {
                let schema = input_schema(&artifact, name, ordinal);
                let shape = mech_core::shape_for_resolved_extents(
                    artifact.schemas().get(schema).unwrap(),
                    &[rows, columns],
                )
                .unwrap();
                admit_value(
                    ValueDraft {
                        schema,
                        shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
                        data: ValueDataDraft::Matrix(data.into_boxed_slice()),
                    }
                    .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
                        artifact.schemas(),
                    ))
                    .unwrap(),
                    &memory,
                )
            };
        let order = |ordinal, text: &str| {
            admit_value(
                scalar_value(
                    &artifact,
                    input_schema(&artifact, "sequence/sort-indices", ordinal),
                    ValueDataDraft::String(text.into()),
                ),
                &memory,
            )
        };
        let mut output = None;
        for count in [5, 0, 5] {
            let primary = [
                9_007_199_254_740_993,
                9_007_199_254_740_992,
                9_007_199_254_740_993,
                9_007_199_254_740_993,
                9_007_199_254_740_992,
            ];
            let keys = ["β", "b", "a", "a", "a"];
            let args = [
                Some(matrix(
                    "sequence/sort-indices",
                    0,
                    1,
                    count,
                    primary[..count as usize]
                        .iter()
                        .copied()
                        .map(ValueDataDraft::U64)
                        .collect(),
                )),
                Some(order(1, "desc")),
                Some(matrix(
                    "sequence/sort-indices",
                    2,
                    1,
                    count,
                    keys[..count as usize]
                        .iter()
                        .map(|s| ValueDataDraft::String((*s).into()))
                        .collect(),
                )),
                Some(order(3, "asc")),
            ];
            let call = call_for_values(&artifact, "sequence/sort-indices", &args, &output, limits);
            let prepared = prepare_native_program_library_call(
                &artifact,
                call.template(),
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            prepared
                .invoke_current(&args, &mut output, limits, &work)
                .unwrap();
            let ValueData::Matrix(value) = output.as_ref().unwrap().data() else {
                panic!("matrix")
            };
            let SequenceView::Index(indices) = value.elements() else {
                panic!("Index result")
            };
            assert_eq!(
                indices,
                if count == 0 {
                    &[][..]
                } else {
                    &[3, 4, 1, 5, 2][..]
                }
            );
            assert!(
                !prepared
                    .invoke_current(&args, &mut output, limits, &work)
                    .unwrap()
            );
            let previous = output.clone().unwrap();
            let mut invalid = args.clone();
            invalid[1] = Some(order(1, "descending"));
            assert!(
                prepared
                    .invoke_current(&invalid, &mut output, limits, &work)
                    .is_err()
            );
            assert!(output.as_ref().unwrap().shares_frozen_storage(&previous));
            assert!(
                !prepared
                    .invoke_current(&args, &mut output, limits, &work)
                    .unwrap()
            );
        }
        for (name, expected) in [
            ("stats/sum/column", &[6, 15][..]),
            ("stats/sum/row", &[5, 7, 9][..]),
        ] {
            let args = [Some(matrix(
                name,
                0,
                2,
                3,
                (1..=6).map(ValueDataDraft::U64).collect(),
            ))];
            let mut output = None;
            let call = call_for_values(&artifact, name, &args, &output, limits);
            let prepared = prepare_native_program_library_call(
                &artifact,
                call.template(),
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            prepared
                .invoke_current(&args, &mut output, limits, &work)
                .unwrap();
            let ValueData::Matrix(value) = output.as_ref().unwrap().data() else {
                panic!("matrix")
            };
            let SequenceView::U64(sums) = value.elements() else {
                panic!("U64 sums")
            };
            assert_eq!(sums, expected);
            assert!(
                !prepared
                    .invoke_current(&args, &mut output, limits, &work)
                    .unwrap()
            );
        }
    }
}

#[test]
fn text_and_matrix_range_leaves_keep_actual_private_factories_in_roots_and_loops() {
    use mech_engine::native_artifact::{NativeIterationAction, lower_native_program_artifact};
    let catalog = mech_stdlib::source_catalog();
    let limits = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    let root = "left := lhs<string>\nright := rhs<string>\nsame := left == right\ndifferent := left != right\njoined := left + right\nxs := values<[string]>\nselected := xs[selectors<[u64]>]\n";
    let lexical = "+> text\ntail := suffix<string>\nxs := values<[string]>\nindices := selectors<[u64]>\nstep(s<string>) => (out<string>) :=\n  same := s == tail\n  different := s != tail\n  selected := xs[indices]\n  out := text/join(selected, s + tail).\nseed<string> |2> step\n";
    for source in [root, lexical] {
        for artifact in artifacts(source) {
            let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
            plan.validate_current_visits().unwrap();
            let mut seen = std::collections::BTreeSet::new();
            for block in plan.blocks() {
                let NativeIterationAction::LibraryCall { template, .. } = block.action() else {
                    continue;
                };
                if !matches!(
                    template.primitive(),
                    NativeCollectionPrimitive::StringEqual
                        | NativeCollectionPrimitive::StringNotEqual
                        | NativeCollectionPrimitive::StringConcat
                        | NativeCollectionPrimitive::MatrixRangeAccess
                ) {
                    continue;
                }
                seen.insert(format!("{:?}", template.primitive()));
                assert_eq!(template.root().is_some(), source == root);
                assert_eq!(template.lexical().is_some(), source == lexical);
                prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
                for extent in [0, 1, 17, 127] {
                    let parameters = template
                        .inputs()
                        .iter()
                        .map(|s| vec![extent; s.dimension_parameters().len()])
                        .collect::<Vec<_>>();
                    let refs = parameters.iter().map(Vec::as_slice).collect::<Vec<_>>();
                    let result = vec![extent; template.output().dimension_parameters().len()];
                    let call = bind_native_program_library_call(
                        &artifact, template, &catalog, &refs, &result, limits,
                    )
                    .unwrap();
                    let memory = call.memory_plan();
                    assert!(
                        memory.allocations.len()
                            + memory.deferred_witnesses.len()
                            + memory.transactions.len()
                            <= (memory.inputs.len() + memory.outputs.len()) * 3 + 16
                    );
                }
                let key = template.selected_key();
                let entry = catalog
                    .resident_factory(&key.module_path, &key.operation_name)
                    .unwrap();
                for altered in [
                    entry.clone().with_factory(replacement),
                    entry
                        .clone()
                        .with_implementation_memory(entry.implementation_memory()),
                ] {
                    let mut replaced = FunctionCatalogBuilder::new();
                    replaced.insert_resident_factory_entry(altered).unwrap();
                    assert!(matches!(
                        prepare_native_program_library_call(
                            &artifact,
                            template,
                            &replaced.build().unwrap()
                        ),
                        Err(NativeArtifactLoweringError::MissingSemanticWitness { .. })
                    ));
                }
            }
            assert_eq!(
                seen,
                std::collections::BTreeSet::from([
                    "StringEqual".to_owned(),
                    "StringNotEqual".to_owned(),
                    "StringConcat".to_owned(),
                    "MatrixRangeAccess".to_owned()
                ])
            );
        }
    }
}

#[test]
fn string_entry_witnesses_do_not_replace_finished_scalar_comparison_seals() {
    use mech_engine::native_artifact::{
        NativeIterationAction, NativeMatchAction, NativeScalarOp, lower_native_program_artifact,
    };
    for (source, expected) in [
        ("x<u64> == y<u64>\n", NativeScalarOp::U64Eq),
        ("x<i64> != y<i64>\n", NativeScalarOp::I64Ne),
        ("x<f64> == y<f64>\n", NativeScalarOp::F64Eq),
        ("x<bool> != y<bool>\n", NativeScalarOp::BoolNe),
    ] {
        for artifact in artifacts(source) {
            let plan =
                lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert!(plan.blocks().iter().any(
                |block| matches!(block.action(), NativeIterationAction::Scalar(
                    NativeMatchAction::Primitive { op, .. }) if *op == expected)
            ));
            assert!(
                !plan.blocks().iter().any(|block| matches!(
                    block.action(),
                    NativeIterationAction::LibraryCall { .. }
                ))
            );
        }
    }
}

fn artifacts(source: &str) -> [ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x434f4c4c), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let compiled = CanonicalSourceFrontend
        .compile_document_with_catalog(&document, mech_stdlib::source_catalog())
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    let original = compiled.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    [original, decoded]
}

fn operations(artifact: &ProgramArtifact) -> Vec<(NodeId, String)> {
    artifact
        .nodes()
        .iter()
        .filter_map(|node| {
            let operation = node.as_operation()?;
            Some((
                node.node,
                format!(
                    "{}/{}",
                    operation.operation.module_path.join("/"),
                    operation.operation.operation_name
                ),
            ))
        })
        .collect()
}

#[test]
fn original_and_decoded_collection_templates_retain_actual_schemas_and_selected_implementations() {
    let catalog = mech_stdlib::source_catalog();
    let cloned = FunctionCatalogBuilder::from_catalog(&catalog)
        .build()
        .unwrap();
    let expected = [
        NativeCollectionPrimitive::TextLines,
        NativeCollectionPrimitive::TextJoin,
        NativeCollectionPrimitive::SequenceLength,
        NativeCollectionPrimitive::NumberFormat,
        NativeCollectionPrimitive::StringHistogram,
        NativeCollectionPrimitive::StringMapKeys,
        NativeCollectionPrimitive::StringMapValues,
    ];
    for artifact in artifacts(SOURCE) {
        let operations = operations(&artifact);
        for primitive in expected {
            let (node, _) = operations
                .iter()
                .find(|(_, name)| name == primitive.name())
                .unwrap();
            for catalog in [catalog.as_ref(), &cloned] {
                let template =
                    native_collection_operation_template(&artifact, *node, catalog).unwrap();
                assert_eq!(template.program_revision(), artifact.revision());
                assert_eq!(template.node(), *node);
                assert_eq!(template.primitive(), primitive);
                assert_eq!(template.sources().len(), primitive.arity());
                assert_eq!(template.inputs().len(), primitive.arity());
                assert_eq!(
                    template.selected_key().operation_name,
                    primitive.name().rsplit('/').next().unwrap()
                );
                assert_eq!(
                    template.output().key(),
                    artifact
                        .schemas()
                        .get(artifact.slots()[template.target().get() as usize].schema)
                        .unwrap()
                        .key()
                );
                assert_eq!(
                    template.contract(),
                    artifact
                        .contracts()
                        .get(
                            artifact.nodes()[node.get() as usize]
                                .as_operation()
                                .unwrap()
                                .contract
                        )
                        .unwrap()
                );
            }
        }
        let invalid = NodeId::new(u32::MAX);
        assert!(
            matches!(native_collection_operation_template(&artifact, invalid, &catalog), Err(NativeArtifactLoweringError::InvalidBinding { node: Some(node) }) if node == invalid)
        );
        // A local library proof cannot silently cover the enclosing aggregate
        // constructor or claim the complete program is executable natively.
        let (other, _) = operations
            .iter()
            .find(|(_, name)| !expected.iter().any(|primitive| primitive.name() == name))
            .unwrap();
        assert!(native_collection_operation_template(&artifact, *other, &catalog).is_err());
    }
}

#[test]
fn actual_collection_calls_bind_native_host_handles_and_explicit_finite_policy() {
    let catalog = mech_stdlib::source_catalog();
    let limits = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    for artifact in artifacts(SOURCE) {
        let mut bound = 0;
        for (node, _) in operations(&artifact) {
            let Ok(template) = native_collection_operation_template(&artifact, node, &catalog)
            else {
                continue;
            };
            let parameters = template
                .inputs()
                .iter()
                .map(|schema| vec![2; schema.dimension_parameters().len()])
                .collect::<Vec<_>>();
            let refs = parameters.iter().map(Vec::as_slice).collect::<Vec<_>>();
            let output_parameters = vec![2; template.output().dimension_parameters().len()];
            let call = bind_native_collection_operation(
                &artifact,
                node,
                &catalog,
                &refs,
                &output_parameters,
                limits,
            )
            .unwrap_or_else(|error| panic!("{}: {error:?}", template.primitive().name()));
            bound += 1;
            assert_eq!(call.template().origin().node(), node);
            assert_eq!(call.memory_plan().bound_call, *call.bound_call());
            assert_eq!(
                call.memory_plan().target.kind,
                mech_core::MemoryTargetKind::NativeHost
            );
            assert_eq!(call.memory_plan().target.limits, limits);
            assert_eq!(call.inputs().len(), template.inputs().len());
            assert_eq!(call.output().schema_key, template.output().key());
            for port in call
                .memory_plan()
                .inputs
                .iter()
                .chain(&call.memory_plan().outputs)
            {
                assert_eq!(
                    port.value.storage.planned_slot(),
                    mech_core::PlannedSlotKind::CanonicalValueHandle
                );
                assert_eq!(
                    port.value.slot.bytes,
                    core::mem::size_of::<Option<mech_core::Value>>() as u64
                );
            }
            assert!(
                call.memory_plan()
                    .allocations
                    .iter()
                    .all(|allocation| allocation.space == mech_core::MemorySpace::Host)
            );
            assert!(!call.memory_plan().deferred_witnesses.is_empty());
            assert!(
                bind_native_collection_operation(
                    &artifact,
                    node,
                    &catalog,
                    &refs,
                    &output_parameters,
                    mech_core::MemoryBudgetLimits::default(),
                )
                .is_err()
            );
            let mut wrong_output = output_parameters.clone();
            wrong_output.push(1);
            assert!(
                bind_native_collection_operation(
                    &artifact,
                    node,
                    &catalog,
                    &refs,
                    &wrong_output,
                    limits,
                )
                .is_err()
            );
        }
        assert_eq!(bound, 7, "all seven actual maintained leaves must bind");
    }
}

fn input_schema(artifact: &ProgramArtifact, name: &str, ordinal: usize) -> mech_core::SchemaId {
    let root = operations(artifact)
        .into_iter()
        .find(|(_, actual)| actual == name);
    let Some((node, _)) = root else {
        let site = lexical_sites(artifact)
            .into_iter()
            .find(|site| site.name == name)
            .unwrap();
        let template = native_collection_lexical_template(
            artifact,
            site.owner,
            site.block,
            site.local,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let mech_core::ResolvedOperationContract::Declared(contract) = template.contract() else {
            panic!("declared lexical call")
        };
        return contract.inputs[ordinal].schema;
    };
    let operation = artifact.nodes()[node.get() as usize]
        .as_operation()
        .unwrap();
    let mech_core::ResolvedOperationContract::Declared(contract) =
        artifact.contracts().get(operation.contract).unwrap()
    else {
        panic!("declared call")
    };
    contract.inputs[ordinal].schema
}

fn scalar_value(
    artifact: &ProgramArtifact,
    schema: mech_core::SchemaId,
    data: mech_core::ValueDataDraft,
) -> mech_core::Value {
    mech_core::ValueDraft {
        schema,
        shape_values: Box::new([]),
        data,
    }
    .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
        artifact.schemas(),
    ))
    .unwrap()
}

fn admit_value(
    value: mech_core::Value,
    memory: &mech_core::ManagedMemoryBudget,
) -> mech_core::Value {
    let mut reservation = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    value.into_memory_budget(&mut reservation).unwrap()
}

fn call_for_values(
    artifact: &ProgramArtifact,
    name: &str,
    inputs: &[Option<mech_core::Value>],
    output: &Option<mech_core::Value>,
    limits: mech_core::MemoryBudgetLimits,
) -> mech_engine::native_artifact::NativeCollectionCall {
    let root = operations(artifact)
        .into_iter()
        .find(|(_, actual)| actual == name);
    let catalog = mech_stdlib::source_catalog();
    let parameters = inputs
        .iter()
        .map(|value| value.as_ref().unwrap().shape().parameter_values())
        .collect::<Vec<_>>();
    let dimensions = if let Some((node, _)) = &root {
        native_collection_operation_template(artifact, *node, &catalog)
            .unwrap()
            .output()
            .dimension_parameters()
            .len()
    } else {
        let site = lexical_sites(artifact)
            .into_iter()
            .find(|site| site.name == name)
            .unwrap();
        native_collection_lexical_template(artifact, site.owner, site.block, site.local, &catalog)
            .unwrap()
            .output()
            .dimension_parameters()
            .len()
    };
    let output_parameters = output
        .as_ref()
        .map(|value| value.shape().parameter_values().to_vec())
        .unwrap_or_else(|| vec![0; dimensions]);
    if let Some((node, _)) = root {
        bind_native_collection_operation(
            artifact,
            node,
            &catalog,
            &parameters,
            &output_parameters,
            limits,
        )
        .unwrap()
    } else {
        let site = lexical_sites(artifact)
            .into_iter()
            .find(|site| site.name == name)
            .unwrap();
        bind_native_collection_lexical(
            artifact,
            site.owner,
            site.block,
            site.local,
            &catalog,
            &parameters,
            &output_parameters,
            limits,
        )
        .unwrap()
    }
}

#[test]
fn native_leaf_invocation_runs_all_seven_real_factories_and_transfers_same_account_results() {
    use mech_core::{ValueData, ValueDataDraft, snapshot::SequenceView};
    use mech_engine::native_artifact::NativeLibraryWorkBudget;
    let limits = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    for artifact in [SOURCE, LEXICAL_ALL_SOURCE].into_iter().flat_map(artifacts) {
        for prepared_mode in [false, true] {
            let memory = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
            {
                let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
                let text_id = input_schema(&artifact, "text/lines", 0);
                let text = admit_value(
                    scalar_value(
                        &artifact,
                        text_id,
                        ValueDataDraft::String("one\ntwo\none".into()),
                    ),
                    &memory,
                );
                let separator = admit_value(
                    scalar_value(
                        &artifact,
                        input_schema(&artifact, "text/join", 1),
                        ValueDataDraft::String(",".into()),
                    ),
                    &memory,
                );
                let precision = admit_value(
                    scalar_value(
                        &artifact,
                        input_schema(&artifact, "number/format", 1),
                        ValueDataDraft::U64(0),
                    ),
                    &memory,
                );
                let invoke = |name: &str, inputs: &[Option<mech_core::Value>]| {
                    let mut output = None;
                    let call = call_for_values(&artifact, name, inputs, &output, limits);
                    let result = if prepared_mode {
                        let prepared = prepare_native_program_library_call(
                            &artifact,
                            call.template(),
                            &mech_stdlib::source_catalog(),
                        )
                        .unwrap();
                        let mut native_limits = limits;
                        // Explicit test policy for admitted live binding, not a
                        // change to resident/native defaults or an unbounded cap.
                        native_limits.max_compute_work = Some(128 * 1024 * 1024);
                        native_limits.max_temporary_bytes = Some(128 * 1024 * 1024);
                        prepared.invoke_current(inputs, &mut output, native_limits, &work)
                    } else {
                        call.invoke(inputs, &mut output, &work)
                    };
                    result.unwrap_or_else(|error| panic!("{name}: {error:?}"));
                    assert!(
                        output
                            .as_ref()
                            .unwrap()
                            .memory_budget_retained_bytes(&memory)
                            .is_some()
                    );
                    output
                };
                let mut words = invoke("text/lines", &[Some(text.clone())]);
                let mut joined = invoke("text/join", &[words.clone(), Some(separator.clone())]);
                assert!(
                    matches!(joined.as_ref().unwrap().data(), ValueData::String(value) if value.as_ref() == "one,two,one")
                );
                // Unchanged joins still finish the candidate's ownership
                // scope. Reusing its actual A-owned wrapper must not search
                // through registrations later imported into other accounts.
                let join_args = [words.clone(), Some(separator.clone())];
                let join_call =
                    call_for_values(&artifact, "text/join", &join_args, &joined, limits);
                let prepared_join = prepared_mode.then(|| {
                    prepare_native_program_library_call(
                        &artifact,
                        join_call.template(),
                        &mech_stdlib::source_catalog(),
                    )
                    .unwrap()
                });
                let repeat_join = |output: &mut Option<mech_core::Value>| {
                    if let Some(prepared) = &prepared_join {
                        let mut policy = limits;
                        policy.max_compute_work = Some(128 * 1024 * 1024);
                        policy.max_temporary_bytes = Some(128 * 1024 * 1024);
                        prepared
                            .invoke_current(&join_args, output, policy, &work)
                            .unwrap()
                    } else {
                        join_call.invoke(&join_args, output, &work).unwrap()
                    }
                };
                let before = work.used();
                assert!(!repeat_join(&mut joined));
                let baseline = (work.used().0 - before.0, work.used().1 - before.1);
                let retained = memory.used_bytes();
                let foreign = (0..16)
                    .map(|_| {
                        let account = mech_core::ManagedMemoryBudget::new(1_000_000);
                        let value = admit_value(joined.as_ref().unwrap().clone(), &account);
                        (account, value)
                    })
                    .collect::<Vec<_>>();
                for _ in 0..2 {
                    let before = work.used();
                    assert!(!repeat_join(&mut joined));
                    assert_eq!(
                        (work.used().0 - before.0, work.used().1 - before.1),
                        baseline
                    );
                    assert_eq!(memory.used_bytes(), retained);
                    assert!(
                        joined
                            .as_ref()
                            .unwrap()
                            .memory_budget_retained_bytes(&memory)
                            .is_some()
                    );
                }
                drop(foreign);
                let length = invoke("sequence/length", &[joined.clone()]);
                assert!(matches!(
                    length.as_ref().unwrap().data(),
                    ValueData::U64(11)
                ));
                let formatted = invoke("number/format", &[length.clone(), Some(precision.clone())]);
                assert!(
                    matches!(formatted.as_ref().unwrap().data(), ValueData::String(value) if value.as_ref() == "11")
                );
                let counts = invoke("map/histogram", &[words.clone()]);
                let keys = invoke("map/keys", &[counts.clone()]);
                let values = invoke("map/values", &[counts.clone()]);
                let ValueData::Matrix(matrix) = keys.as_ref().unwrap().data() else {
                    panic!("keys matrix")
                };
                let SequenceView::String(keys) = matrix.elements() else {
                    panic!("String keys")
                };
                assert_eq!(
                    keys.iter().map(|value| value.as_ref()).collect::<Vec<_>>(),
                    ["one", "two"]
                );
                let ValueData::Matrix(matrix) = values.as_ref().unwrap().data() else {
                    panic!("count matrix")
                };
                let SequenceView::U64(values) = matrix.elements() else {
                    panic!("exact counts")
                };
                assert_eq!(values, [2, 1]);

                // Reuse a live binding across a changed data-dependent shape.
                let args = [Some(text.clone())];
                let lines = call_for_values(&artifact, "text/lines", &args, &words, limits);
                let shorter = admit_value(
                    scalar_value(&artifact, text_id, ValueDataDraft::String("x\ny".into())),
                    &memory,
                );
                let before = work.used();
                lines.invoke(&[Some(shorter)], &mut words, &work).unwrap();
                let ValueData::Matrix(matrix) = words.as_ref().unwrap().data() else {
                    panic!("words")
                };
                assert_eq!(matrix.elements().len(), 2);
                assert!(work.used().0 > before.0);
                if prepared_mode {
                    let prepared = prepare_native_program_library_call(
                        &artifact,
                        lines.template(),
                        &mech_stdlib::source_catalog(),
                    )
                    .unwrap();
                    let mut native_limits = limits;
                    native_limits.max_compute_work = Some(128 * 1024 * 1024);
                    native_limits.max_temporary_bytes = Some(128 * 1024 * 1024);
                    let one_line = admit_value(
                        scalar_value(&artifact, text_id, ValueDataDraft::String("single".into())),
                        &memory,
                    );
                    prepared
                        .invoke_current(&[Some(one_line)], &mut words, native_limits, &work)
                        .unwrap();
                    let ValueData::Matrix(matrix) = words.as_ref().unwrap().data() else {
                        panic!("live words")
                    };
                    assert_eq!(matrix.elements().len(), 1);
                }
            }
            assert_eq!(
                memory.used_bytes(),
                0,
                "all payload and invocation owners release"
            );
        }
    }
}

#[test]
fn prepared_native_binding_is_pre_admitted_and_cannot_refill_the_leaf_allowance() {
    use mech_core::{MemoryBudgetDimension, MemoryRuntimeError, ValueDataDraft};
    use mech_engine::native_artifact::{
        NativeCollectionInvocationError as Error, NativeIterationAction, NativeLibraryWorkBudget,
        lower_native_program_artifact,
    };
    for artifact in artifacts("+> sequence\nsequence/length(input<string>)\n") {
        let catalog = mech_stdlib::source_catalog();
        let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
        let template = plan
            .blocks()
            .iter()
            .find_map(|block| match block.action() {
                NativeIterationAction::LibraryCall { template, .. } => Some(template),
                _ => None,
            })
            .unwrap();
        let prepared = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
        let memory = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let input = admit_value(
            scalar_value(
                &artifact,
                input_schema(&artifact, "sequence/length", 0),
                ValueDataDraft::String("é🙂".into()),
            ),
            &memory,
        );
        let args = [Some(input)];
        let mut output = None;
        let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
            .unwrap()
            .limits;
        limits.max_temporary_bytes = Some(128 * 1024 * 1024);
        // Setup consumes this entire relative allowance. A fresh inner scope
        // would incorrectly permit the otherwise tiny actual length call.
        limits.max_compute_work = Some(prepared.binding_work());
        let before_memory = memory.used_bytes();
        assert!(matches!(
            prepared.invoke_current(&args, &mut output, limits, &work),
            Err(Error::Kernel(
                mech_core::ResidentKernelError::ResourceLimit {
                    dimension: MemoryBudgetDimension::ComputeWork,
                    ..
                }
            ))
        ));
        assert!(output.is_none());
        assert_eq!(memory.used_bytes(), before_memory);
        assert_eq!(work.used().0, prepared.binding_work());
        limits.max_compute_work = Some(128 * 1024 * 1024);
        prepared
            .invoke_current(&args, &mut output, limits, &work)
            .unwrap();
        assert!(matches!(
            output.as_ref().unwrap().data(),
            mech_core::ValueData::U64(2)
        ));
        let prior = output.as_ref().unwrap().clone();
        let before_memory = memory.used_bytes();
        let blocker = memory
            .reserve_capacity(memory.limit_bytes() - before_memory)
            .unwrap();
        assert!(matches!(
            prepared.invoke_current(&args, &mut output, limits, &work),
            Err(Error::Memory(MemoryRuntimeError::BudgetExceeded { .. }))
        ));
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        drop(blocker);
        assert_eq!(memory.used_bytes(), before_memory);
        assert!(
            !prepared
                .invoke_current(&args, &mut output, limits, &work)
                .unwrap()
        );
        let prior = output.as_ref().unwrap().clone();
        // Enough memory for binding alone, but none left for invocation
        // metadata. The binding reservation must remain alive across invoke.
        let before_memory = memory.used_bytes();
        let blocker = memory
            .reserve_capacity(
                memory.limit_bytes() - before_memory - prepared.binding_metadata_bytes(),
            )
            .unwrap();
        assert!(matches!(
            prepared.invoke_current(&args, &mut output, limits, &work),
            Err(Error::Memory(MemoryRuntimeError::BudgetExceeded { .. }))
        ));
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        drop(blocker);
        assert_eq!(memory.used_bytes(), before_memory);
        // Likewise, each part fits this temporary cap separately, but their
        // coexistence does not. Omitting the subtraction would publish here.
        limits.max_temporary_bytes = Some(prepared.binding_metadata_bytes() + 1_024);
        assert!(
            matches!(prepared.invoke_current(&args, &mut output, limits, &work), Err(Error::Plan(mech_core::MemoryPlanError::TargetLimitExceeded { violation })) if violation.dimension == MemoryBudgetDimension::TemporaryBytes)
        );
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        assert_eq!(memory.used_bytes(), before_memory);
        limits.max_temporary_bytes = Some(128 * 1024 * 1024);
        let input_schema = input_schema(&artifact, "sequence/length", 0);
        let unadmitted = [Some(scalar_value(
            &artifact,
            input_schema,
            ValueDataDraft::String("x".into()),
        ))];
        assert_eq!(
            prepared.invoke_current(&unadmitted, &mut output, limits, &work),
            Err(Error::UnadmittedValue)
        );
        let foreign = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
        let foreign_args = [Some(admit_value(
            scalar_value(&artifact, input_schema, ValueDataDraft::String("x".into())),
            &foreign,
        ))];
        assert_eq!(
            prepared.invoke_current(&foreign_args, &mut output, limits, &work),
            Err(Error::UnadmittedValue)
        );
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        let independent = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        assert_eq!(
            work.with_scope(|| prepared.invoke_current(&args, &mut output, limits, &independent))
                .unwrap(),
            Err(Error::Kernel(mech_core::ResidentKernelError::InvalidInput))
        );
        assert_eq!(independent.used(), (0, 0));
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        drop(independent);
        let before_memory = memory.used_bytes();
        limits.max_temporary_bytes = Some(prepared.binding_metadata_bytes() - 1);
        let blocker = memory
            .reserve_capacity(memory.limit_bytes() - before_memory)
            .unwrap();
        assert!(
            matches!(prepared.invoke_current(&args, &mut output, limits, &work), Err(Error::Plan(mech_core::MemoryPlanError::TargetLimitExceeded { violation })) if violation.dimension == MemoryBudgetDimension::TemporaryBytes)
        );
        assert!(output.is_some());
        drop(blocker);
        assert_eq!(memory.used_bytes(), before_memory);
        let mut unlimited = limits;
        unlimited.max_compute_work = None;
        assert!(matches!(
            prepared.invoke_current(&args, &mut output, unlimited, &work),
            Err(Error::Binding(
                NativeArtifactLoweringError::UnsupportedNode { .. }
            ))
        ));
    }
}

#[test]
fn invocation_owned_fixed_binding_cache_keeps_live_values_caps_and_ledger_identity() {
    use mech_core::{ValueData, ValueDataDraft};
    use mech_engine::native_artifact::{
        NativeCollectionInvocationError as Error, NativeIterationAction, NativeLibraryWorkBudget,
        NativePreparedLibraryCallCache, lower_native_program_artifact,
    };
    for artifact in artifacts("+> sequence\nsequence/length(input<string>)\n") {
        let catalog = mech_stdlib::source_catalog();
        let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
        let template = plan
            .blocks()
            .iter()
            .find_map(|block| match block.action() {
                NativeIterationAction::LibraryCall { template, .. } => Some(template),
                _ => None,
            })
            .unwrap();
        let prepared = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
        let other_site_owner =
            prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
        let memory = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let independent = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let foreign = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
        let foreign_work =
            NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &foreign).unwrap();
        let schema = input_schema(&artifact, "sequence/length", 0);
        let args = ["é🙂", "abc"].map(|text| {
            [Some(admit_value(
                scalar_value(&artifact, schema, ValueDataDraft::String(text.into())),
                &memory,
            ))]
        });
        let mech_core::ResolvedOperationContract::Declared(contract) = template.contract() else {
            panic!("declared maintained call")
        };
        let wrong = [Some(admit_value(
            scalar_value(
                &artifact,
                contract.outputs[0].schema,
                ValueDataDraft::U64(99),
            ),
            &memory,
        ))];
        let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
            .unwrap()
            .limits;
        limits.max_compute_work = Some(128 * 1024 * 1024);
        limits.max_temporary_bytes = Some(128 * 1024 * 1024);
        let baseline = memory.used_bytes();
        let mut cache = NativePreparedLibraryCallCache::default();
        let mut output = None;
        let mut denied = limits;
        denied.max_temporary_bytes = Some(prepared.binding_metadata_bytes() - 1);
        assert!(matches!(
            prepared.invoke_current_cached(&mut cache, &args[0], &mut output, denied, &work),
            Err(Error::Plan(mech_core::MemoryPlanError::TargetLimitExceeded { violation }))
                if violation.dimension == mech_core::MemoryBudgetDimension::TemporaryBytes
        ));
        assert!(output.is_none());
        assert_eq!(memory.used_bytes(), baseline);
        // Failed first admission leaves this same slot empty and usable.
        let start = work.used().0;
        prepared
            .invoke_current_cached(&mut cache, &args[0], &mut output, limits, &work)
            .unwrap();
        let first = work.used().0 - start;
        assert!(matches!(output.as_ref().unwrap().data(), ValueData::U64(2)));
        let start = work.used().0;
        prepared
            .invoke_current_cached(&mut cache, &args[1], &mut output, limits, &work)
            .unwrap();
        let repeated = work.used().0 - start;
        assert!(matches!(output.as_ref().unwrap().data(), ValueData::U64(3)));
        assert!(
            first > repeated + prepared.binding_work() / 2,
            "first={first}, repeated={repeated}, binding={}",
            prepared.binding_work()
        );
        let prior = output.as_ref().unwrap().clone();
        let retained = memory.used_bytes();
        assert_eq!(
            prepared.invoke_current_cached(&mut cache, &wrong, &mut output, limits, &work),
            Err(Error::InvalidInput)
        );
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        assert_eq!(memory.used_bytes(), retained);
        drop(prior);
        // Dropping the result does not release the still-live bound metadata.
        drop(output.take());
        let retained = memory.used_bytes();
        assert!(retained >= baseline + prepared.binding_metadata_bytes());
        let mut changed = limits;
        changed.max_output_elements = Some(limits.max_output_elements.unwrap() + 1);
        for (owner, policy, ledger) in [
            (&other_site_owner, limits, &work),
            (&prepared, changed, &work),
            (&prepared, limits, &independent),
            (&prepared, limits, &foreign_work),
        ] {
            assert_eq!(
                owner.invoke_current_cached(&mut cache, &args[0], &mut output, policy, ledger),
                Err(Error::InvalidInput)
            );
            assert!(output.is_none());
            assert_eq!(memory.used_bytes(), retained);
        }
        // The exact original ledger can still invoke the same maintained leaf.
        prepared
            .invoke_current_cached(&mut cache, &args[0], &mut output, limits, &work)
            .unwrap();
        assert!(matches!(output.as_ref().unwrap().data(), ValueData::U64(2)));
        drop(output.take());
        drop(cache);
        assert_eq!(memory.used_bytes(), baseline);
        drop((args, wrong, work, independent, foreign_work));
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(foreign.used_bytes(), 0);
    }
}

#[test]
fn live_binding_cache_keys_absent_and_published_output_geometry_without_caching_values() {
    use mech_core::{Value, ValueDataDraft as Data};
    use mech_engine::native_artifact::{
        NativeIterationAction, NativeLibraryWorkBudget, NativePreparedLibraryCallCache,
        lower_native_program_artifact,
    };
    for artifact in artifacts("start<u64>..end<u64>\n") {
        let catalog = mech_stdlib::source_catalog();
        let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
        let template = plan
            .blocks()
            .iter()
            .find_map(|block| match block.action() {
                NativeIterationAction::LibraryCall { template, .. } => Some(template),
                _ => None,
            })
            .unwrap();
        assert!(!template.output().dimension_parameters().is_empty());
        let mech_core::ResolvedOperationContract::Declared(contract) = template.contract() else {
            panic!("declared range")
        };
        let prepared = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
        let memory = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let args = [(1u64, 4u64), (3, 8), (5, 7)].map(|(start, end)| {
            [start, end].map(|value| {
                Some(admit_value(
                    scalar_value(&artifact, contract.inputs[0].schema, Data::U64(value)),
                    &memory,
                ))
            })
        });
        let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
            .unwrap()
            .limits;
        limits.max_compute_work = Some(128 * 1024 * 1024);
        limits.max_temporary_bytes = Some(128 * 1024 * 1024);
        let baseline = memory.used_bytes();
        let mut cache = NativePreparedLibraryCallCache::default();
        let mut output: Option<Value> = None;
        let mut invoke = |index: usize, output: &mut Option<Value>| {
            let start = work.used().0;
            prepared
                .invoke_current_cached(&mut cache, &args[index], output, limits, &work)
                .unwrap();
            work.used().0 - start
        };
        let check = |output: &Option<Value>, start: u64, end: u64| {
            assert_eq!(
                output.as_ref().unwrap().canonical_data_draft().unwrap(),
                Data::Matrix((start..end).map(Data::U64).collect())
            )
        };
        let first = invoke(0, &mut output);
        check(&output, 1, 4);
        drop(output.take());
        let same_absent_signature = invoke(1, &mut output);
        check(&output, 3, 8); // Different endpoints and length, same binding key.
        assert!(first > same_absent_signature + prepared.binding_work() / 2);
        let published_signature = invoke(2, &mut output);
        check(&output, 5, 7); // Some prior length5 must not hit bound zero tape.
        assert!(published_signature > same_absent_signature + prepared.binding_work() / 2);
        drop(output.take());
        let absent_again = invoke(0, &mut output);
        check(&output, 1, 4); // None must not hit the prior length5 signature.
        drop(output.take());
        let hit_again = invoke(1, &mut output);
        check(&output, 3, 8);
        assert!(absent_again > hit_again + prepared.binding_work() / 2);
        drop(invoke);
        drop(output.take());
        drop(cache);
        assert_eq!(memory.used_bytes(), baseline);
        drop((args, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn prepared_matrix_format_derives_current_input_shape_without_or_after_publication() {
    use mech_core::{SchemaBody, ValueData, ValueDataDraft, ValueDraft};
    use mech_engine::native_artifact::{
        NativeIterationAction, NativeLibraryWorkBudget, lower_native_program_artifact,
    };
    for source in [
        "+> number\nnumber/format(values<[i64]>, 0u64)\n",
        "+> number\nresult := (choose<bool>? | true => number/format(values<[i64]>, 0u64) | false => number/format(values, 0u64))\nresult\n",
    ] {
        for artifact in artifacts(source) {
            let catalog = mech_stdlib::source_catalog();
            let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
            let template = plan
                .blocks()
                .iter()
                .find_map(|block| match block.action() {
                    NativeIterationAction::LibraryCall { template, .. } => Some(template),
                    _ => None,
                })
                .unwrap();
            let prepared =
                prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
            let memory = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
            let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
            let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
                .unwrap()
                .limits;
            limits.max_compute_work = Some(128 * 1024 * 1024);
            limits.max_temporary_bytes = Some(128 * 1024 * 1024);
            let precision = admit_value(
                scalar_value(
                    &artifact,
                    input_schema(&artifact, "number/format", 1),
                    ValueDataDraft::U64(0),
                ),
                &memory,
            );
            let schema_id = input_schema(&artifact, "number/format", 0);
            let schema = artifact.schemas().get(schema_id).unwrap();
            let mut output = None;
            // A cache slot must not freeze parameterized matrix geometry.
            let mut cache = mech_engine::native_artifact::NativePreparedLibraryCallCache::default();
            let mut costs = Vec::new();
            for (visit, columns) in [3, 3, 0, 1, 5, 3, 3].into_iter().enumerate() {
                let offset = (visit as i64) * 10;
                let shape = mech_core::shape_for_resolved_extents(schema, &[1, columns]).unwrap();
                let value = ValueDraft {
                    schema: schema_id,
                    shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
                    data: ValueDataDraft::Matrix(
                        (0..columns)
                            .map(|i| ValueDataDraft::I64(i as i64 + 1 + offset))
                            .collect::<Vec<_>>()
                            .into_boxed_slice(),
                    ),
                }
                .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
                    artifact.schemas(),
                ))
                .unwrap();
                let input = admit_value(value, &memory);
                let start = work.used().0;
                prepared
                    .invoke_current_cached(
                        &mut cache,
                        &[Some(input), Some(precision.clone())],
                        &mut output,
                        limits,
                        &work,
                    )
                    .unwrap();
                costs.push(work.used().0 - start);
                let value = output.as_ref().unwrap();
                assert!(value.memory_budget_retained_bytes(&memory).is_some());
                let SchemaBody::Matrix { dimensions, .. } = template.output().body() else {
                    panic!("matrix output")
                };
                assert_eq!(value.shape().resolve_dimension(&dimensions[0]).unwrap(), 1);
                assert_eq!(
                    value.shape().resolve_dimension(&dimensions[1]).unwrap(),
                    columns
                );
                let ValueData::Matrix(matrix) = value.data() else {
                    panic!("matrix value")
                };
                assert_eq!(matrix.elements().len(), columns as usize);
                let mech_core::snapshot::SequenceView::String(values) = matrix.elements() else {
                    panic!("packed String output")
                };
                assert_eq!(
                    values
                        .iter()
                        .map(|value| value.as_ref())
                        .collect::<Vec<_>>(),
                    (1..=columns)
                        .map(|value| (value as i64 + offset).to_string())
                        .collect::<Vec<_>>()
                );
            }
            assert!(costs[0] > costs[1] + prepared.binding_work() / 2);
            assert!(costs[5] > costs[6] + prepared.binding_work() / 2);
            // A concrete shape miss over a fixed cap may fail at re-binding
            // or fresh turn admission, but cannot retain two bound plans or
            // overwrite the previous publication. The same site/ledger/caps
            // must recover when the earlier valid shape returns.
            let mut bounded = limits;
            bounded.max_output_elements = Some(3);
            let make = |columns: u64| {
                let shape = mech_core::shape_for_resolved_extents(schema, &[1, columns]).unwrap();
                admit_value(
                    ValueDraft {
                        schema: schema_id,
                        shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
                        data: ValueDataDraft::Matrix(
                            (0..columns)
                                .map(|i| ValueDataDraft::I64(i as i64 + 1))
                                .collect(),
                        ),
                    }
                    .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
                        artifact.schemas(),
                    ))
                    .unwrap(),
                    &memory,
                )
            };
            let good = [Some(make(2)), Some(precision.clone())];
            let bad = [Some(make(4)), Some(precision.clone())];
            let mut bounded_cache =
                mech_engine::native_artifact::NativePreparedLibraryCallCache::default();
            let mut bounded_output = None;
            prepared
                .invoke_current_cached(
                    &mut bounded_cache,
                    &good,
                    &mut bounded_output,
                    bounded,
                    &work,
                )
                .unwrap();
            let prior = bounded_output.as_ref().unwrap().clone();
            let before = memory.used_bytes();
            assert!(
                prepared
                    .invoke_current_cached(
                        &mut bounded_cache,
                        &bad,
                        &mut bounded_output,
                        bounded,
                        &work,
                    )
                    .is_err()
            );
            assert!(
                bounded_output
                    .as_ref()
                    .unwrap()
                    .shares_frozen_storage(&prior)
            );
            assert!(memory.used_bytes() <= before);
            let mut changed = bounded;
            changed.max_output_elements = Some(4);
            assert_eq!(
                prepared.invoke_current_cached(
                    &mut bounded_cache,
                    &good,
                    &mut bounded_output,
                    changed,
                    &work,
                ),
                Err(mech_engine::native_artifact::NativeCollectionInvocationError::InvalidInput)
            );
            // Release the comparison alias before a fresh equal publication;
            // the maintained formatter may replace an equal immutable root.
            drop(prior);
            prepared
                .invoke_current_cached(
                    &mut bounded_cache,
                    &good,
                    &mut bounded_output,
                    bounded,
                    &work,
                )
                .unwrap();
            assert_eq!(
                bounded_output
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                ValueDataDraft::Matrix(
                    ["1", "2"]
                        .into_iter()
                        .map(|text| ValueDataDraft::String(text.into()))
                        .collect()
                )
            );
            drop((
                cache,
                output,
                bounded_cache,
                bounded_output,
                good,
                bad,
                precision,
                work,
            ));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn native_leaf_errors_and_memory_denials_preserve_output_and_recover_on_the_same_account() {
    use mech_core::{MemoryRuntimeError, ValueDataDraft};
    use mech_engine::native_artifact::{
        NativeCollectionInvocationError as Error, NativeLibraryWorkBudget,
    };
    let limits = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    for artifact in [SOURCE, LEXICAL_ALL_SOURCE].into_iter().flat_map(artifacts) {
        let memory = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let integer = input_schema(&artifact, "number/format", 1);
        let good = [
            Some(admit_value(
                scalar_value(&artifact, integer, ValueDataDraft::U64(42)),
                &memory,
            )),
            Some(admit_value(
                scalar_value(&artifact, integer, ValueDataDraft::U64(0)),
                &memory,
            )),
        ];
        let mut output = None;
        let call = call_for_values(&artifact, "number/format", &good, &output, limits);
        call.invoke(&good, &mut output, &work).unwrap();
        let prior = output.clone().unwrap();
        let bad_precision = [
            good[0].clone(),
            Some(admit_value(
                scalar_value(&artifact, integer, ValueDataDraft::U64(13)),
                &memory,
            )),
        ];
        assert!(call.invoke(&bad_precision, &mut output, &work).is_err());
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        let before = memory.used_bytes();
        let blocker = memory
            .reserve_capacity(memory.limit_bytes() - before)
            .unwrap();
        assert!(matches!(
            call.invoke(&good, &mut output, &work),
            Err(Error::Memory(MemoryRuntimeError::BudgetExceeded { .. }))
        ));
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        drop(blocker);
        assert_eq!(memory.used_bytes(), before);
        assert!(!call.invoke(&good, &mut output, &work).unwrap());
        // Successful unchanged execution follows the maintained leaf's normal
        // publication behavior: equality is semantic, not root-pointer identity.
        assert!(
            output
                .as_ref()
                .unwrap()
                .snapshot_eq(artifact.schemas(), &prior, artifact.schemas())
                .unwrap()
        );
        let prior = output.as_ref().unwrap().clone();
        let unadmitted = [
            Some(scalar_value(&artifact, integer, ValueDataDraft::U64(42))),
            good[1].clone(),
        ];
        assert_eq!(
            call.invoke(&unadmitted, &mut output, &work),
            Err(Error::UnadmittedValue)
        );
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        let foreign = mech_core::ManagedMemoryBudget::new(1_000_000);
        let wrong_account = [
            Some(admit_value(
                scalar_value(&artifact, integer, ValueDataDraft::U64(42)),
                &foreign,
            )),
            good[1].clone(),
        ];
        assert_eq!(
            call.invoke(&wrong_account, &mut output, &work),
            Err(Error::UnadmittedValue)
        );
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        assert!(!call.invoke(&good, &mut output, &work).unwrap());
    }
}

#[test]
fn native_leaf_call_policy_cannot_be_lifted_by_a_larger_invocation_account() {
    use mech_core::{MemoryBudgetDimension, ValueDataDraft};
    use mech_engine::native_artifact::{
        NativeCollectionInvocationError as Error, NativeLibraryWorkBudget,
    };
    for artifact in [SOURCE, LEXICAL_ALL_SOURCE].into_iter().flat_map(artifacts) {
        let memory = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let text = admit_value(
            scalar_value(
                &artifact,
                input_schema(&artifact, "text/lines", 0),
                ValueDataDraft::String("one\ntwo".into()),
            ),
            &memory,
        );
        let args = [Some(text)];
        let mut output = None;
        let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
            .unwrap()
            .limits;
        limits.max_compute_work = Some(1);
        let call = call_for_values(&artifact, "text/lines", &args, &output, limits);
        let before = memory.used_bytes();
        assert!(matches!(
            call.invoke(&args, &mut output, &work),
            Err(Error::Kernel(
                mech_core::ResidentKernelError::ResourceLimit {
                    dimension: MemoryBudgetDimension::ComputeWork,
                    ..
                }
            ))
        ));
        assert!(output.is_none());
        assert_eq!(memory.used_bytes(), before);
        assert_eq!(work.used(), (0, 0));
        limits.max_compute_work = Some(16_777_216);
        for temporary in [1_024, 65_536] {
            limits.max_temporary_bytes = Some(temporary);
            let call = call_for_values(&artifact, "text/lines", &args, &output, limits);
            // The per-call policy must reject before any private owner is
            // allocated, even when its larger invocation account is full.
            let blocker = memory
                .reserve_capacity(memory.limit_bytes() - before)
                .unwrap();
            assert!(
                matches!(call.invoke(&args, &mut output, &work), Err(Error::Plan(mech_core::MemoryPlanError::TargetLimitExceeded { violation })) if violation.dimension == MemoryBudgetDimension::TemporaryBytes)
            );
            assert!(output.is_none());
            drop(blocker);
            assert_eq!(memory.used_bytes(), before);
        }
    }
}

#[test]
fn native_leaf_invocation_requires_the_actual_current_shape_parameters() {
    use mech_core::ValueDataDraft;
    use mech_engine::native_artifact::{
        NativeCollectionInvocationError as Error, NativeLibraryWorkBudget,
    };
    let limits = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    for artifact in artifacts(SOURCE) {
        let memory = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let text = admit_value(
            scalar_value(
                &artifact,
                input_schema(&artifact, "text/lines", 0),
                ValueDataDraft::String("one\ntwo\none".into()),
            ),
            &memory,
        );
        let separator = admit_value(
            scalar_value(
                &artifact,
                input_schema(&artifact, "text/join", 1),
                ValueDataDraft::String(",".into()),
            ),
            &memory,
        );
        let args = [Some(text)];
        let mut words = None;
        call_for_values(&artifact, "text/lines", &args, &words, limits)
            .invoke(&args, &mut words, &work)
            .unwrap();
        let mut stale = words.as_ref().unwrap().shape().parameter_values().to_vec();
        assert!(!stale.is_empty());
        stale[0] += 1;
        let catalog = mech_stdlib::source_catalog();
        let node = |name: &str| {
            operations(&artifact)
                .into_iter()
                .find(|(_, actual)| actual == name)
                .unwrap()
                .0
        };
        let joined_args = [words.clone(), Some(separator)];
        let wrong_input = bind_native_collection_operation(
            &artifact,
            node("text/join"),
            &catalog,
            &[&stale, &[]],
            &[],
            limits,
        )
        .unwrap();
        let mut joined = None;
        let before = memory.used_bytes();
        assert_eq!(
            wrong_input.invoke(&joined_args, &mut joined, &work),
            Err(Error::InvalidInput)
        );
        assert!(joined.is_none());
        assert_eq!(memory.used_bytes(), before);
        call_for_values(&artifact, "text/join", &joined_args, &joined, limits)
            .invoke(&joined_args, &mut joined, &work)
            .unwrap();

        let wrong_publication = bind_native_collection_operation(
            &artifact,
            node("text/lines"),
            &catalog,
            &[&[]],
            &stale,
            limits,
        )
        .unwrap();
        let prior = words.as_ref().unwrap().clone();
        let before = memory.used_bytes();
        assert_eq!(
            wrong_publication.invoke(&args, &mut words, &work),
            Err(Error::InvalidInput)
        );
        assert!(words.as_ref().unwrap().shares_frozen_storage(&prior));
        assert_eq!(memory.used_bytes(), before);
        call_for_values(&artifact, "text/lines", &args, &words, limits)
            .invoke(&args, &mut words, &work)
            .unwrap();
    }
}

static BINDS: AtomicUsize = AtomicUsize::new(0);
fn replacement(
    _: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    BINDS.fetch_add(1, Ordering::SeqCst);
    Err(ResidentKernelBindError::UnsupportedLayout)
}

#[test]
fn factory_replacements_names_costs_and_public_opcodes_do_not_grant_collection_authority() {
    #[derive(Debug)]
    struct ForgedOpcode(NativeCollectionPrimitive);
    let maintained = mech_stdlib::source_catalog();
    let original = maintained
        .resident_factory(&["sequence".into()], "length")
        .unwrap();
    let forged = Arc::new(ForgedOpcode(NativeCollectionPrimitive::SequenceLength));
    assert_eq!(forged.0, NativeCollectionPrimitive::SequenceLength);
    let variants = [
        original.clone().with_factory(replacement),
        original.clone().with_factory(original.factory()),
        original.clone().with_key(original.key().clone()),
        original
            .clone()
            .with_implementation_memory(original.implementation_memory()),
        original.clone().with_lowering_witness(forged),
    ];
    for artifact in artifacts(SOURCE) {
        let node = operations(&artifact)
            .into_iter()
            .find(|(_, name)| name == "sequence/length")
            .unwrap()
            .0;
        for changed in &variants {
            let mut builder = FunctionCatalogBuilder::new();
            builder
                .insert_resident_factory_entry(changed.clone())
                .unwrap();
            let catalog: FunctionCatalog = builder.build().unwrap();
            assert!(
                matches!(native_collection_operation_template(&artifact, node, &catalog), Err(NativeArtifactLoweringError::MissingSemanticWitness { node: actual }) if node == actual)
            );
            assert!(matches!(
                bind_native_collection_operation(
                    &artifact,
                    node,
                    &catalog,
                    &[&[]],
                    &[],
                    mech_core::TargetMemoryProfile::current_resident_cpu().unwrap().limits,
                ),
                Err(NativeArtifactLoweringError::MissingSemanticWitness { node: actual }) if node == actual
            ));
        }
    }
    assert_eq!(
        BINDS.load(Ordering::SeqCst),
        0,
        "metadata lowering must not bind or execute replacement code"
    );
}

struct Site {
    owner: NodeId,
    block: mech_engine::ControlBlockId,
    local: u32,
    name: String,
    sources: Box<[NativeCollectionLexicalSource]>,
}
fn lexical_sites(artifact: &ProgramArtifact) -> Vec<Site> {
    use mech_engine::{ControlOperationBody as Body, ExecutableNodeBody as Root};
    fn operation_name(operation: &mech_engine::OperationReference) -> String {
        format!(
            "{}/{}",
            operation.module_path.join("/"),
            operation.operation_name
        )
    }
    fn body(owner: NodeId, declaration: &Body, sites: &mut Vec<Site>) {
        match declaration {
            Body::Match(declaration) => matched(owner, declaration, sites),
            Body::Iteration(declaration) => block(owner, &declaration.body, sites),
            Body::Comprehension(declaration) => comprehension(owner, declaration, sites),
            _ => {}
        }
    }
    fn block(owner: NodeId, declaration: &mech_engine::ControlBlock, sites: &mut Vec<Site>) {
        for operation in &declaration.operations {
            if let Body::Operation {
                operation: reference,
                ..
            } = &operation.body
            {
                sites.push(Site {
                    owner,
                    block: declaration.id,
                    local: operation.node,
                    name: operation_name(reference),
                    sources: operation
                        .inputs
                        .iter()
                        .copied()
                        .map(NativeCollectionLexicalSource::Control)
                        .collect(),
                });
            }
            body(owner, &operation.body, sites);
        }
    }
    fn matched(owner: NodeId, declaration: &mech_engine::MatchDeclaration, sites: &mut Vec<Site>) {
        for arm in &declaration.arms {
            if let Some(guard) = &arm.guard {
                block(owner, guard, sites);
            }
            block(owner, &arm.body, sites);
        }
    }
    fn comprehension(
        owner: NodeId,
        declaration: &mech_engine::ComprehensionDeclaration,
        sites: &mut Vec<Site>,
    ) {
        for step in &declaration.steps {
            if let mech_engine::ComprehensionStep::Operation(operation) = step {
                if let Body::Operation {
                    operation: reference,
                    ..
                } = &operation.body
                {
                    sites.push(Site {
                        owner,
                        block: declaration.id,
                        local: operation.local,
                        name: operation_name(reference),
                        sources: operation
                            .inputs
                            .iter()
                            .copied()
                            .map(NativeCollectionLexicalSource::Comprehension)
                            .collect(),
                    });
                }
                body(owner, &operation.body, sites);
            }
        }
    }
    let mut sites = Vec::new();
    for node in artifact.nodes() {
        match &node.body {
            Root::Match(declaration) | Root::Activation(declaration) => {
                matched(node.node, declaration, &mut sites)
            }
            Root::Iteration(declaration) => block(node.node, &declaration.body, &mut sites),
            Root::Comprehension(declaration) => comprehension(node.node, declaration, &mut sites),
            _ => {}
        }
    }
    sites
}

const LEXICAL_CASES: &[&str] = &[
    "+> sequence\n(signal<string>? | value => sequence/length(value))\n",
    "+> sequence\n+> text\n[sequence/length(word) | word <- text/lines(signal<string>)]\n",
    "+> sequence\n+> text\n(flag<bool>? | true => [sequence/length(word) | word <- text/lines(signal<string>)] | false => [sequence/length(word) | word <- text/lines(signal<string>)])\n",
    "+> sequence\n+> number\nstep(x<string>) => (next<string>) := next := number/format(sequence/length(x), 0u64).\nseed<string> |2> step\n",
];

#[test]
fn collection_templates_resolve_genuine_match_comprehension_nested_and_iteration_sites() {
    let catalog = mech_stdlib::source_catalog();
    let mut control_operands = false;
    let mut comprehension_operands = false;
    for &source in LEXICAL_CASES {
        for artifact in artifacts(source) {
            let sites = lexical_sites(&artifact);
            let lengths = sites
                .iter()
                .filter(|site| site.name == "sequence/length")
                .collect::<Vec<_>>();
            assert!(
                !lengths.is_empty(),
                "{source}: expected an actual lexical library site"
            );
            for site in lengths {
                let template = native_collection_lexical_template(
                    &artifact, site.owner, site.block, site.local, &catalog,
                )
                .unwrap_or_else(|error| panic!("{source}: {error:?}"));
                assert_eq!(template.program_revision(), artifact.revision());
                assert_eq!(template.owner(), site.owner);
                assert_eq!(template.block(), site.block);
                assert_eq!(template.local(), site.local);
                assert_eq!(
                    template.primitive(),
                    NativeCollectionPrimitive::SequenceLength
                );
                assert_eq!(template.sources(), site.sources.as_ref());
                assert_eq!(template.inputs()[0].body(), &mech_core::SchemaBody::String);
                assert_eq!(
                    template.output().body(),
                    &mech_core::SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64)
                );
                for source in template.sources() {
                    control_operands |= matches!(source, NativeCollectionLexicalSource::Control(_));
                    comprehension_operands |=
                        matches!(source, NativeCollectionLexicalSource::Comprehension(_));
                }
                assert!(
                    native_collection_lexical_template(
                        &artifact,
                        site.owner,
                        mech_engine::ControlBlockId(u32::MAX),
                        site.local,
                        &catalog
                    )
                    .is_err()
                );
                assert!(
                    native_collection_lexical_template(
                        &artifact,
                        site.owner,
                        site.block,
                        u32::MAX,
                        &catalog
                    )
                    .is_err()
                );
                assert!(
                    native_collection_lexical_template(
                        &artifact,
                        NodeId::new(u32::MAX),
                        site.block,
                        site.local,
                        &catalog
                    )
                    .is_err()
                );
            }
        }
    }
    assert!(control_operands && comprehension_operands);
}

#[test]
fn lexical_collection_templates_also_reject_replaced_factories_without_binding_them() {
    let maintained = mech_stdlib::source_catalog();
    let original = maintained
        .resident_factory(&["sequence".into()], "length")
        .unwrap();
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_resident_factory_entry(original.clone().with_factory(replacement))
        .unwrap();
    let catalog = builder.build().unwrap();
    for artifact in artifacts(LEXICAL_CASES[2]) {
        let sites = lexical_sites(&artifact);
        let site = sites
            .iter()
            .find(|site| site.name == "sequence/length")
            .unwrap();
        assert!(
            matches!(native_collection_lexical_template(&artifact, site.owner, site.block, site.local, &catalog),
            Err(NativeArtifactLoweringError::MissingSemanticWitness { node }) if node == site.owner)
        );
    }
    assert_eq!(BINDS.load(Ordering::SeqCst), 0);
}
