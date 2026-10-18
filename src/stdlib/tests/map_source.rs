#![cfg(all(feature = "full_compiler", feature = "map_ops"))]

use mech_core::{
    DimensionExpr, ExecutionTarget, FunctionExposure, FunctionInvocation,
    FunctionValueRepresentation, IntegerWidth, OperationId, ReactiveInstanceId, ResidentValueKind,
    ResidentValueRef, ResolvedOperationDescriptor, RuntimeBindingSelector, RuntimeFunctionId,
    SchemaBody, SpecializedFunction, Value, ValueCell, ValueData,
    snapshot::{
        MapEntryDraft, SequenceView, SnapshotValidationContext, ValueDataDraft, ValueDraft,
    },
};
use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn compile(source: &str) -> (ProgramArtifact, usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4d61), Revision(0), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let product = CanonicalSourceFrontend
        .compile_document_with_catalog(&document, mech_stdlib::source_catalog())
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    let output = product
        .document_outputs()
        .iter()
        .rev()
        .find(|output| output.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    (product.compile_artifact().unwrap(), output)
}
fn assert_projection(value: &Value, keys: &[&str], counts: &[u64]) {
    let ValueData::Tuple(values) = value.data() else {
        panic!("{value:?}");
    };
    let [
        ValueData::Matrix(key_matrix),
        ValueData::Matrix(count_matrix),
    ] = values.as_ref()
    else {
        panic!("{value:?}");
    };
    let (SequenceView::String(actual_keys), SequenceView::U64(actual_counts)) =
        (key_matrix.elements(), count_matrix.elements())
    else {
        panic!("{value:?}");
    };
    assert_eq!(
        actual_keys
            .iter()
            .map(|key| key.as_ref())
            .collect::<Vec<_>>(),
        keys
    );
    assert_eq!(actual_counts, counts);
    let schemas = value.schemas().unwrap();
    let SchemaBody::Tuple(parts) = schemas
        .get(value.schema())
        .unwrap()
        .closed_body(value.shape())
        .unwrap()
    else {
        panic!();
    };
    for part in parts {
        let SchemaBody::Matrix { dimensions, .. } = part else {
            panic!();
        };
        assert_eq!(
            dimensions.as_ref(),
            [
                DimensionExpr::Constant(1),
                DimensionExpr::Constant(keys.len() as u64)
            ]
        );
    }
}

#[test]
fn map_family_has_ordinary_exports_maintained_contracts_and_native_linkage() {
    let catalog = mech_stdlib::source_catalog();
    for name in ["histogram", "keys", "values"] {
        let canonical = format!("map/{name}");
        let operation = OperationId::from_name(&canonical);
        let export = catalog.module_export("map", name).unwrap();
        assert_eq!(export.operation, operation);
        assert_eq!(export.exposure, FunctionExposure::ModuleOnly);
        assert!(catalog.specializer(operation).is_some());
        assert!(
            catalog
                .resident_factory(&["map".to_owned()], name)
                .is_some()
        );
        let entries = catalog
            .runtime_entries_for_binding(
                RuntimeBindingSelector::Operation(operation),
                ExecutionTarget::DirectRuntime,
            )
            .collect::<Vec<_>>();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, canonical);
        assert_eq!(entries[0].id, RuntimeFunctionId::from_name(&canonical));
        assert_eq!(
            entries[0].signature(),
            mech_core::RuntimeFunctionSignature::unary(
                FunctionValueRepresentation::AnyValue,
                FunctionValueRepresentation::AnyValue,
            )
        );
        assert!(
            matches!(entries[0].operation_binding(), mech_core::RuntimeOperationBinding::Fixed(ids)
            if ids.as_ref() == [operation])
        );
        assert_eq!(
            entries[0].implementation_memory_class(),
            if name == "histogram" {
                mech_core::ImplementationMemoryClass::CanonicalSortUnique
            } else {
                mech_core::ImplementationMemoryClass::CanonicalFinalize
            }
        );
        let linkage = entries[0].native_linkage.as_ref().unwrap();
        assert_eq!(linkage.package, "mech-engine");
        assert_eq!(linkage.crate_name, "mech_engine");
        assert_eq!(
            linkage.installer_path,
            format!("mech_engine::__mech_native::install_map_{name}")
        );
        assert_eq!(
            linkage.cargo_features,
            ["map_ops", "native-link", "runtime"]
        );
        let specializer = catalog.specializer(operation).unwrap();
        assert_eq!(specializer.operation.id, operation);
        assert_eq!(specializer.operation.canonical_name.as_ref(), canonical);
        assert_eq!(
            entries[0].operation_contract(operation),
            Some(
                &mech_core::maintained_operation_contract(&canonical, 1, name != "histogram")
                    .unwrap()
            )
        );
    }
    capture_qualified_catalog_inventory(&catalog);
}

// This opt-in test artifact is produced only after the exact additive family
// assertions above pass. It captures real catalogs without replacing any
// historical fixture, factory ID, or digest expectation.
fn capture_qualified_catalog_inventory(source: &mech_core::FunctionCatalog) {
    let Some(path) = std::env::var_os("MECH_MAP_INVENTORY_CAPTURE") else {
        return;
    };
    let path = std::path::PathBuf::from(path);
    assert!(
        path.is_absolute(),
        "catalog inventory capture must use an explicit absolute path"
    );
    fn runtime_entries(catalog: &mech_core::FunctionCatalog) -> Vec<serde_json::Value> {
        let mut entries = catalog
            .runtime_entries()
            .map(|entry| (entry.name.clone(), format!("{:016x}", entry.id.raw())))
            .collect::<Vec<_>>();
        entries.sort();
        entries
            .into_iter()
            .map(|(name, id_hex)| serde_json::json!({"name": name, "id_hex": id_hex}))
            .collect()
    }
    let runtime = mech_stdlib::runtime_catalog();
    let mut specializers = source
        .all_specializers()
        .map(|entry| {
            (
                entry.operation.canonical_name.to_string(),
                format!("{:016x}", entry.operation.id.raw()),
            )
        })
        .collect::<Vec<_>>();
    specializers.sort();
    let full_source_specializers = specializers
        .iter()
        .map(|(name, id_hex)| serde_json::json!({"name": name, "id_hex": id_hex}))
        .collect::<Vec<_>>();
    let prelude_source_specializers = specializers
        .iter()
        .filter(|(name, _)| {
            source
                .exports_for_operation(OperationId::from_name(name))
                .iter()
                .any(|export| export.exposure == FunctionExposure::Prelude)
        })
        .map(|(name, id_hex)| serde_json::json!({"name": name, "id_hex": id_hex}))
        .collect::<Vec<_>>();
    let mut modules = source
        .all_exports()
        .filter_map(|export| match (&export.module, &export.item) {
            (Some(module), Some(item)) => Some((
                module.clone(),
                item.clone(),
                export.canonical_name.clone(),
                format!("{:016x}", export.operation.raw()),
            )),
            (None, None) => None,
            _ => panic!("validated export has a partial module/item pair"),
        })
        .collect::<Vec<_>>();
    modules.sort();
    let module_exports = modules.into_iter().map(|(module, item, canonical_name, id_hex)|
        serde_json::json!({"module": module, "item": item, "canonical_name": canonical_name, "id_hex": id_hex})
    ).collect::<Vec<_>>();
    let additions = ["map/histogram", "map/keys", "map/values"].into_iter().map(|name| {
        let entry = source.runtime_entry(RuntimeFunctionId::from_name(name)).unwrap();
        let linkage = entry.native_linkage.as_ref().unwrap();
        serde_json::json!({
            "name": entry.name,
            "id_hex": format!("{:016x}", entry.id.raw()),
            "operation_id_hex": format!("{:016x}", OperationId::from_name(name).raw()),
            "memory_class": format!("{:?}", entry.implementation_memory_class()),
            "native_linkage": {"package": linkage.package, "crate_name": linkage.crate_name,
                "installer_path": linkage.installer_path, "cargo_features": linkage.cargo_features},
        })
    }).collect::<Vec<_>>();
    let inventory = serde_json::json!({
        "schema": 1,
        "profile": "mech-stdlib/full_compiler,map_ops",
        "feature_closure": {
            "full_runtime": cfg!(feature = "full_runtime"),
            "full_source": cfg!(feature = "full_source"),
            "full_compiler": cfg!(feature = "full_compiler"),
            "native-sequence": cfg!(feature = "native-sequence"),
            "native-plan": cfg!(feature = "native-plan"),
            "matrix1": cfg!(feature = "matrix1"), "matrix2": cfg!(feature = "matrix2"),
            "matrix3": cfg!(feature = "matrix3"), "matrix4": cfg!(feature = "matrix4"),
            "matrix2x3": cfg!(feature = "matrix2x3"), "matrix3x2": cfg!(feature = "matrix3x2"),
            "row_vector2": cfg!(feature = "row_vector2"), "row_vector3": cfg!(feature = "row_vector3"),
            "row_vector4": cfg!(feature = "row_vector4"), "vector2": cfg!(feature = "vector2"),
            "vector3": cfg!(feature = "vector3"), "vector4": cfg!(feature = "vector4"),
        },
        "runtime_factory_count": runtime.runtime_factory_count(),
        "source_runtime_factory_count": source.runtime_factory_count(),
        "runtime_factories": runtime_entries(&runtime),
        "source_runtime_factories": runtime_entries(source),
        "full_source_specializers": full_source_specializers,
        "prelude_source_specializers": prelude_source_specializers,
        "module_exports": module_exports,
        "map_additions": additions,
    });
    std::fs::write(path, serde_json::to_vec_pretty(&inventory).unwrap()).unwrap();
}

#[cfg(feature = "native-sequence")]
#[test]
fn packed_sequence_catalog_remains_separate_from_the_ordinary_string_map_family() {
    use mech_engine::native_sequence::{OutputKind, SequenceKind};
    let packed = mech_stdlib::native_sequence_catalog();
    for (name, id, input, output) in [
        (
            "map/histogram",
            5,
            SequenceKind::Windows,
            SequenceKind::Histogram,
        ),
        (
            "map/keys",
            13,
            SequenceKind::Histogram,
            SequenceKind::Integers,
        ),
        (
            "map/values",
            14,
            SequenceKind::Histogram,
            SequenceKind::Integers,
        ),
    ] {
        let declaration = packed
            .iter()
            .find(|declaration| declaration.name == name)
            .unwrap();
        assert_eq!(declaration.op, id);
        assert_eq!(declaration.inputs, &[&[input][..]]);
        assert!(matches!(declaration.output, OutputKind::Fixed(kind) if kind == output));
        let ordinary = mech_stdlib::source_catalog();
        assert_eq!(
            ordinary
                .runtime_entries_for_binding(
                    RuntimeBindingSelector::Operation(OperationId::from_name(name)),
                    ExecutionTarget::DirectRuntime
                )
                .count(),
            1
        );
    }
}

#[test]
fn ordinary_histogram_counts_unicode_strings_and_projects_aligned_canonical_rows() {
    let source = "+> map\nh := map/histogram([\"z\" \"é\" \"a\"; \"é\" \"z\" \"a\"])\n(map/keys(h), map/values(h))\n";
    let (original, output) = compile(source);
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    for artifact in [&original, &decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x4d61, 0),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        for _ in 0..2 {
            instance.turn(&[]).unwrap();
            assert_projection(
                &instance.copied_output(output).unwrap(),
                &["a", "é", "z", "é"],
                &[2, 1, 2, 1],
            );
        }
    }
}

#[test]
fn projections_preserve_large_u64_counts_and_resize_to_one_by_zero() {
    let (artifact, output) =
        compile("+> map\n(map/keys(counts<{string:u64}>), map/values(counts))\n");
    let mut instance = activate(
        ReactiveInstanceId::new(0x4d62, 0),
        &artifact,
        &mech_stdlib::source_catalog(),
        &ActivationFacts::default(),
    )
    .unwrap();
    let slot = instance.plan.inputs[0].slot;
    for entries in [
        vec![("z", u64::MAX), ("a", 9_007_199_254_740_993)],
        vec![],
        vec![("x", 7)],
    ] {
        let data = entries
            .iter()
            .map(|(key, count)| MapEntryDraft {
                items: vec![
                    ValueDataDraft::String((*key).into()),
                    ValueDataDraft::U64(*count),
                ]
                .into_boxed_slice(),
            })
            .collect::<Vec<_>>();
        let input = [Some(
            ValueDraft {
                schema: artifact.inputs()[0].schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::Map(data.into_boxed_slice()),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap(),
        )];
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::Snapshot(&input),
            }])
            .unwrap();
        let mut expected = entries;
        expected.sort_by_key(|(key, _)| *key);
        assert_projection(
            &instance.copied_output(output).unwrap(),
            &expected.iter().map(|(key, _)| *key).collect::<Vec<_>>(),
            &expected.iter().map(|(_, count)| *count).collect::<Vec<_>>(),
        );
    }
}

#[test]
fn empty_string_matrix_histogram_produces_empty_map_and_one_by_zero_projections() {
    let (artifact, output) =
        compile("+> map\nh := map/histogram(words<[string]:0,3>)\n(map/keys(h), map/values(h))\n");
    let mut instance = activate(
        ReactiveInstanceId::new(0x4d63, 0),
        &artifact,
        &mech_stdlib::source_catalog(),
        &ActivationFacts::default(),
    )
    .unwrap();
    let input = [Some(
        ValueDraft {
            schema: artifact.inputs()[0].schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Matrix(Box::new([])),
        }
        .finalize(&SnapshotValidationContext::new(artifact.schemas()))
        .unwrap(),
    )];
    let strings: Vec<String> = Vec::new();
    let value = if instance.plan.inputs[0].region.kind == ResidentValueKind::Snapshot {
        ResidentValueRef::Snapshot(&input)
    } else {
        ResidentValueRef::String(&strings)
    };
    instance
        .turn(&[CapturedSignalInput {
            slot: instance.plan.inputs[0].slot,
            value,
        }])
        .unwrap();
    assert_projection(&instance.copied_output(output).unwrap(), &[], &[]);
}

#[test]
fn map_family_rejects_unsupported_kinds_in_its_public_type_declarations() {
    let catalog = mech_stdlib::source_catalog();
    for source in [
        "+> map\nmap/histogram([1<u64> 2<u64>])\n",
        "+> map\nmap/keys({1<u64>: 2<u64>})\n",
        "+> map\nmap/values({\"x\": 2.0})\n",
    ] {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x4d64), Revision(0), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(
            parsed.is_strictly_clean(),
            "{source}: {:?}",
            parsed.diagnostics
        );
        let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
        assert!(
            CanonicalSourceFrontend
                .compile_document_with_catalog(&document, catalog.clone())
                .is_err(),
            "{source}"
        );
    }
}

#[test]
fn managed_histogram_and_projections_use_the_same_canonical_result_and_shape_contract() {
    let input = ValueCell::from_schema_data(
        SchemaBody::Matrix {
            element: Box::new(SchemaBody::String),
            dimensions: Box::new([DimensionExpr::Constant(1), DimensionExpr::Constant(4)]),
        },
        ValueDataDraft::Matrix(
            ["b", "a", "b", "a"]
                .into_iter()
                .map(|value| ValueDataDraft::String(value.into()))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
    )
    .unwrap();
    let map = ValueCell::from_schema_data(
        SchemaBody::Map {
            key: Box::new(SchemaBody::String),
            value: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
            cardinality: mech_core::CardinalitySpec::Dynamic { upper_bound: None },
        },
        ValueDataDraft::Map(Box::new([])),
    )
    .unwrap();
    let histogram = managed("map/histogram", &map, &input);
    histogram.instance().solve_result().unwrap();
    let snapshot = map.snapshot().unwrap();
    let ValueData::Map(entries) = snapshot.data() else {
        panic!();
    };
    assert_eq!(entries.entries().len(), 2);
    for (entry, key) in entries.entries().iter().zip(["a", "b"]) {
        assert!(matches!(entry.key().data(), ValueData::String(actual) if actual.as_ref() == key));
        assert!(matches!(entry.value(), ValueData::U64(2)));
    }
    for (operation, element) in [
        ("map/keys", SchemaBody::String),
        ("map/values", SchemaBody::UnsignedInteger(IntegerWidth::W64)),
    ] {
        let template = ValueCell::dynamic_matrix(element, Box::new([1, 0]), Box::new([])).unwrap();
        let output = ValueCell::allocate_for_descriptor(
            &template.resolved_descriptor().unwrap(),
            FunctionValueRepresentation::AnyValue,
        )
        .unwrap();
        assert!(output.requires_canonical_output_builder().unwrap());
        assert_eq!(
            output.representation(),
            FunctionValueRepresentation::AnyValue
        );
        let domain = output.memory_domain().unwrap();
        let input_map = map.import_owned_in(&domain).unwrap();
        let function = managed(operation, &output, &input_map);
        // Exact shape authority is refreshed from the current Map metadata,
        // including both growth and shrinkage without changing cell/schema ID.
        for entries in [
            vec![],
            vec![
                ("z".to_string(), u64::MAX),
                ("a".to_string(), 9_007_199_254_740_993),
            ],
            vec![],
            vec![("x".to_string(), 7)],
        ] {
            replace_managed_count_map(&input_map, &entries);
            function.instance().solve_result().unwrap();
            assert_managed_projection(operation, &output, &entries);
        }
        // A larger changed candidate forces fresh admitted storage. Both
        // denial paths must retain the old publication, including its shape.
        for (attempt, failure) in [
            mech_core::MemoryFailurePoint::Admission,
            mech_core::MemoryFailurePoint::HostAllocation,
        ]
        .into_iter()
        .enumerate()
        {
            let entries = (0..32 * (attempt + 1))
                .map(|index| (format!("key-{index:04}"), index as u64 + 1))
                .collect::<Vec<_>>();
            replace_managed_count_map(&input_map, &entries);
            let before = output.snapshot().unwrap();
            let version = output.published_version();
            let ledger = domain.ledger();
            domain.inject_failure_after(failure, 0).unwrap();
            assert!(
                function.instance().solve_result().is_err(),
                "{operation}: {failure:?}"
            );
            assert_eq!(output.published_version(), version);
            assert_eq!(domain.ledger(), ledger);
            let after = output.snapshot().unwrap();
            assert!(
                before
                    .snapshot_eq(
                        &before.schemas().unwrap(),
                        &after,
                        &after.schemas().unwrap(),
                    )
                    .unwrap()
            );
            function.instance().solve_result().unwrap();
            assert_managed_projection(operation, &output, &entries);
            replace_managed_count_map(&input_map, &[]);
            function.instance().solve_result().unwrap();
            assert_managed_projection(operation, &output, &[]);
        }
    }
}
fn replace_managed_count_map(map: &ValueCell, entries: &[(String, u64)]) {
    let data = entries
        .iter()
        .map(|(key, count)| MapEntryDraft {
            items: vec![
                ValueDataDraft::String(key.clone()),
                ValueDataDraft::U64(*count),
            ]
            .into_boxed_slice(),
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let candidate = map.rebuild_data_draft(ValueDataDraft::Map(data)).unwrap();
    map.replace(&candidate).unwrap();
    assert_eq!(map.current_map_entry_count().unwrap(), entries.len() as u64);
}

fn assert_managed_projection(operation: &str, output: &ValueCell, entries: &[(String, u64)]) {
    let mut expected = entries.to_vec();
    expected.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));
    let value = output.snapshot().unwrap();
    let ValueData::Matrix(matrix) = value.data() else {
        panic!("projection must be a matrix");
    };
    match (operation, matrix.elements()) {
        ("map/keys", SequenceView::String(actual)) => assert_eq!(
            actual.iter().map(|text| text.as_ref()).collect::<Vec<_>>(),
            expected
                .iter()
                .map(|(text, _)| text.as_str())
                .collect::<Vec<_>>(),
        ),
        ("map/values", SequenceView::U64(actual)) => assert_eq!(
            actual,
            expected
                .iter()
                .map(|(_, count)| *count)
                .collect::<Vec<_>>()
                .as_slice(),
        ),
        _ => panic!("projection has wrong element kind"),
    }
    assert_eq!(
        output.current_top_level_extents().unwrap().as_ref(),
        &[1, entries.len() as u64]
    );
    assert!(
        matches!(value.schemas().unwrap().get(value.schema()).unwrap().closed_body(value.shape()).unwrap(),
            SchemaBody::Matrix { dimensions, .. }
            if dimensions.as_ref() == [DimensionExpr::Constant(1), DimensionExpr::Constant(entries.len() as u64)])
    );
}

fn managed(operation: &str, output: &ValueCell, input: &ValueCell) -> SpecializedFunction {
    let catalog = mech_stdlib::source_catalog();
    let id = OperationId::from_name(operation);
    let entry = catalog
        .runtime_entry(RuntimeFunctionId::from_name(operation))
        .unwrap();
    let parts = entry
        .bind_resolved_invocation(
            id,
            ExecutionTarget::DirectRuntime,
            FunctionInvocation::unary(output.clone(), input.clone()),
        )
        .unwrap();
    SpecializedFunction::syntax_directed(
        parts,
        ResolvedOperationDescriptor::from_name(
            operation,
            entry.operation_contract(id).unwrap().clone(),
        )
        .unwrap(),
        entry.id,
        ExecutionTarget::DirectRuntime,
        entry.implementation_memory_class(),
    )
    .unwrap()
}

#[test]
fn admitted_map_failures_preserve_published_output_and_recover() {
    // Two live Strings fit the input/retention bounds, but the histogram's
    // conservative merge-sort comparison demand exceeds the maintained call
    // limit. This must fail before the first index-buffer allocation.
    let (artifact, output) = compile("+> map\nmap/histogram(words<[string]:1,2>)\n");
    let mut instance = activate(
        ReactiveInstanceId::new(0x4d65, 0),
        &artifact,
        &mech_stdlib::source_catalog(),
        &ActivationFacts::default(),
    )
    .unwrap();
    let slot = instance.plan.inputs[0].slot;
    assert_eq!(
        instance.plan.inputs[0].region.kind,
        ResidentValueKind::String
    );
    let valid = ["a".to_owned(), "a".to_owned()];
    instance
        .turn(&[CapturedSignalInput {
            slot,
            value: ResidentValueRef::String(&valid),
        }])
        .unwrap();
    let before = instance.copied_output(output).unwrap();
    let width = usize::try_from(mech_core::RESIDENT_MAX_COMPARISON_WORK / 14 + 1).unwrap();
    let denied = ["x".repeat(width), "x".repeat(width)];
    assert!(width * 2 < mech_core::RESIDENT_MAX_BYTES as usize);
    assert!((width * 2 + 2) * 7 > mech_core::RESIDENT_MAX_COMPARISON_WORK as usize);
    assert!(
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::String(&denied)
            }])
            .is_err()
    );
    let after = instance.copied_output(output).unwrap();
    assert!(
        before
            .snapshot_eq(artifact.schemas(), &after, artifact.schemas())
            .unwrap()
    );
    instance
        .turn(&[CapturedSignalInput {
            slot,
            value: ResidentValueRef::String(&valid),
        }])
        .unwrap();
    assert!(
        before
            .snapshot_eq(
                artifact.schemas(),
                &instance.copied_output(output).unwrap(),
                artifact.schemas()
            )
            .unwrap()
    );

    // The managed runtime has separate admission and fallible allocation
    // authority. Exercise both with the established public failure injector;
    // failed construction must preserve version, contents and owned ledger.
    let input = ValueCell::from_schema_data(
        SchemaBody::Matrix {
            element: Box::new(SchemaBody::String),
            dimensions: Box::new([DimensionExpr::Constant(1), DimensionExpr::Constant(2)]),
        },
        ValueDataDraft::Matrix(
            vec![
                ValueDataDraft::String("b".into()),
                ValueDataDraft::String("a".into()),
            ]
            .into_boxed_slice(),
        ),
    )
    .unwrap();
    let map = ValueCell::from_schema_data(
        SchemaBody::Map {
            key: Box::new(SchemaBody::String),
            value: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
            cardinality: mech_core::CardinalitySpec::Dynamic { upper_bound: None },
        },
        ValueDataDraft::Map(Box::new([])),
    )
    .unwrap();
    let domain = mech_core::MemoryDomain::new().unwrap();
    let input = ValueCell::from_snapshot_in(&domain, input.snapshot().unwrap()).unwrap();
    let map = ValueCell::from_snapshot_in(&domain, map.snapshot().unwrap()).unwrap();
    let histogram = managed("map/histogram", &map, &input);
    histogram.instance().solve_result().unwrap();
    for (attempt, point) in [
        mech_core::MemoryFailurePoint::Admission,
        mech_core::MemoryFailurePoint::HostAllocation,
    ]
    .into_iter()
    .enumerate()
    {
        let candidate = ValueCell::from_schema_data(
            input.closed_schema_body().unwrap(),
            ValueDataDraft::Matrix(
                vec![
                    ValueDataDraft::String("candidate".repeat(64 * (attempt + 1))),
                    ValueDataDraft::String("other".repeat(64 * (attempt + 1))),
                ]
                .into_boxed_slice(),
            ),
        )
        .unwrap();
        input.replace(&candidate.snapshot().unwrap()).unwrap();
        let before = map.snapshot().unwrap();
        let version = map.published_version();
        let ledger = domain.ledger();
        domain.inject_failure_after(point, 0).unwrap();
        assert!(histogram.instance().solve_result().is_err(), "{point:?}");
        assert_eq!(map.published_version(), version);
        assert_eq!(domain.ledger(), ledger);
        let after = map.snapshot().unwrap();
        assert!(
            before
                .snapshot_eq(
                    &before.schemas().unwrap(),
                    &after,
                    &after.schemas().unwrap()
                )
                .unwrap()
        );
        histogram.instance().solve_result().unwrap();
    }
}
