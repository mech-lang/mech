//! Metadata-only differential tests; the canonical suites cover real binding,
//! admitted owners, allocation, publication, denials and recovery.
use super::*;
use crate::resident::budget::ResidentBudgetMeter;
#[cfg(feature = "native-artifact")]
use crate::resident::budget::{
    NativeLibraryWorkBudget, detached_turn_plan, with_resident_turn_plan,
};
use mech_core::{DimensionExpr, SchemaDraft, SchemaTable, SchemaTableBuilder, Value};

struct Fixture {
    source: Value,
    schemas: Arc<SchemaTable>,
    index_schema: SchemaId,
    index_shape: ShapeInstance,
}

fn fixture() -> Fixture {
    let mut builder = SchemaTableBuilder::new();
    let mut insert = |body| {
        builder
            .insert(
                SchemaDraft {
                    body,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap()
    };
    let source_handle = insert(SchemaBody::Matrix {
        element: Box::new(SchemaBody::String),
        dimensions: vec![DimensionExpr::Constant(3), DimensionExpr::Constant(4)].into_boxed_slice(),
    });
    let index_handle = insert(SchemaBody::Index);
    let build = builder.finish().unwrap();
    let source_schema = build.resolve(source_handle).unwrap();
    let index_schema = build.resolve(index_handle).unwrap();
    let schemas = Arc::new(build.table);
    let texts = [
        "",
        "é",
        "a\0b",
        "x\r\ny",
        "🧑‍🚀",
        "z",
        "abc",
        "e\u{301}",
        "é",
        "",
        "\0",
        "tail",
    ];
    let source = ValueDraft {
        schema: source_schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::Matrix(
            texts
                .into_iter()
                .map(|s| ValueDataDraft::String(s.into()))
                .collect(),
        ),
    }
    .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
    .unwrap();
    let index_shape = schemas
        .get(index_schema)
        .unwrap()
        .instantiate_shape(Box::new([]))
        .unwrap();
    Fixture {
        source,
        schemas,
        index_schema,
        index_shape,
    }
}

enum Selector {
    Index(Vec<u64>),
    Bool(Vec<u8>),
    Float(Vec<f64>),
}
impl Selector {
    fn view(&self) -> ResidentValueRef<'_> {
        match self {
            Self::Index(v) => ResidentValueRef::Index(v),
            Self::Bool(v) => ResidentValueRef::Bool(v),
            Self::Float(v) => ResidentValueRef::F64(v),
        }
    }
}
struct Inputs<'a>(&'a [ResidentValueRef<'a>]);
impl ResidentKernelInputs for Inputs<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn get(&self, i: usize) -> Option<ResidentValueRef<'_>> {
        self.0.get(i).copied()
    }
}

// This private cost fixture grants no allocation/publication permit. The
// selector layouts are unused by this function apart from their cardinality.
fn plan(f: &Fixture, mode: ResolvedSelectionMode, selectors: usize) -> SnapshotAccessPlan {
    SnapshotAccessPlan {
        dense_source: None,
        matrix_constructor: None,
        selectors: (0..selectors)
            .map(|_| SnapshotAccessSelectorLayout {
                schema: f.index_schema,
                shape: f.index_shape.clone(),
                resident_shape: ResidentShape::SCALAR,
            })
            .collect(),
        matrix_mode: Some(mode),
        source_dimensions: Some((3, 4)),
        output_dimensions: Some((1, 1)),
        output_dimensions_dynamic: true,
        linear_gather_row: false,
        output_geometry: ResolvedAccessGeometry {
            logical_output_rows: 1,
            logical_output_columns: 1,
        },
        aggregate_ordinal: None,
        output_schema: f.source.schema(),
    }
}

fn cost(
    f: &Fixture,
    mode: ResolvedSelectionMode,
    selectors: &[Selector],
    meter: &mut ResidentBudgetMeter,
    legacy: bool,
) -> Result<SnapshotAccessOutputCost, ResidentKernelError> {
    let measured = measure_source(meter, &f.source, &f.schemas)?;
    let source_input = Some(f.source.clone());
    let mut inputs = vec![ResidentValueRef::Snapshot(core::slice::from_ref(
        &source_input,
    ))];
    inputs.extend(selectors.iter().map(Selector::view));
    snapshot_access_output_cost(
        &f.source,
        f.schemas.get(f.source.schema()).unwrap().body(),
        &plan(f, mode, selectors.len()),
        &Inputs(&inputs),
        meter,
        if legacy { None } else { Some(&measured) },
    )
}

#[test]
fn selected_string_scans_match_old_cost_and_paid_prefix_for_all_selection_modes() {
    use ResolvedSelectionMode::*;
    use Selector::*;
    let f = fixture();
    for (mode, selectors) in [
        (Rows, vec![Index(vec![1, 3])]),
        (Rows, vec![Index(vec![3, 1])]),
        (Rows, vec![Index(vec![2, 2])]),
        (Rows, vec![Index(vec![1, 3, 2])]),
        (Rows, vec![Bool(vec![1, 0, 1])]),
        (Rows, vec![Index(vec![])]),
        (Columns, vec![Index(vec![4, 1])]),
        (Columns, vec![Float(vec![1.0, 4.0])]),
        (Rectangle, vec![Index(vec![3, 1]), Index(vec![4, 2])]),
        (Rectangle, vec![Index(vec![1, 1]), Index(vec![2, 4])]),
        (LinearGather, vec![]),
        (LinearGather, vec![Index(vec![1, 4, 7, 12])]),
        (LinearGather, vec![Index(vec![12, 7, 4, 1])]),
        (LinearGather, vec![Index(vec![1, 8, 4, 12])]),
        (LinearGather, vec![Index(vec![1, 12, 12])]),
        (
            LinearGather,
            vec![Bool(vec![1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0])],
        ),
        (LinearGather, vec![Index(vec![])]),
        (LinearGather, vec![Index(vec![0, 2])]),
        (Rows, vec![Index(vec![1, 4])]),
        (Columns, vec![Float(vec![f64::NAN, 1.0])]),
        (Columns, vec![Bool(vec![1, 2, 0, 1])]),
    ] {
        let mut baseline = ResidentBudgetMeter::for_closed_analysis(1_000_000, 0);
        let _ = cost(&f, mode, &selectors, &mut baseline, true);
        let work = baseline.estimate().compute_work();
        for limit in [
            0,
            1,
            63,
            64,
            65,
            127,
            128,
            255,
            1023,
            work.saturating_sub(1),
            work,
            work + 1,
            1_000_000,
        ] {
            let mut old = ResidentBudgetMeter::for_closed_analysis(limit, 0);
            let mut new = old;
            assert_eq!(
                cost(&f, mode, &selectors, &mut new, false),
                cost(&f, mode, &selectors, &mut old, true)
            );
            assert_eq!(new, old, "mode={mode:?}, limit={limit}");
        }
    }
}

#[cfg(feature = "native-artifact")]
#[test]
fn selected_string_scan_real_turn_denials_match_old_account_and_node_prefixes() {
    let f = fixture();
    let selectors = [Selector::Index(vec![3, 1])];
    let execute = |legacy, compute, nodes, account, call| {
        let memory = mech_core::ManagedMemoryBudget::new(4096);
        let budget = NativeLibraryWorkBudget::new(account, 0, &memory).unwrap();
        budget.charge(11, 0).unwrap();
        let mut turn = detached_turn_plan(mech_core::ResourceDemand::default()).unwrap();
        turn.budget_limits.max_compute_work = Some(compute);
        turn.budget_limits.max_retained_nodes = Some(nodes);
        with_resident_turn_plan(turn, || {
            budget
                .with_call_limits(call, 0, || {
                    let mut meter = ResidentBudgetMeter::default();
                    let result = cost(
                        &f,
                        ResolvedSelectionMode::Rows,
                        &selectors,
                        &mut meter,
                        legacy,
                    );
                    (result, meter.estimate(), budget.used())
                })
                .unwrap()
        })
    };
    let baseline = execute(true, 1_000_000, 1_000, 1_000_000, 1_000_000);
    assert!(baseline.0.is_ok());
    let work = baseline.1.compute_work();
    let retained = baseline.1.retained_nodes();
    for compute in [63, 127, 1023, work - 1, work, work + 1, 1_000_000] {
        for nodes in [0, 1, 13, 14, 15, retained - 1, retained, 1_000] {
            for account in [11, 64, 11 + work - 1, 11 + work, 1_000_000] {
                for call in [63, work - 1, work, 1_000_000] {
                    assert_eq!(
                        execute(false, compute, nodes, account, call),
                        execute(true, compute, nodes, account, call),
                        "compute={compute}, nodes={nodes}, account={account}, call={call}"
                    );
                }
            }
        }
    }
}

#[test]
fn selector_summary_proves_only_monotone_distinct_cells_and_pins_the_exact_view() {
    for (indices, unique) in [
        (vec![], true),
        (vec![1], true),
        (vec![1, 3, 5], true),
        (vec![5, 3, 1], true),
        (vec![1, 1], false),
        (vec![1, 5, 3], false),
    ] {
        let summary = summarize(ResidentValueRef::Index(&indices), 5).unwrap();
        assert_eq!(summary.count, indices.len());
        assert_eq!(summary.unique, unique);
        let ResidentValueRef::Index(pinned) = summary.value() else {
            panic!("same representation")
        };
        assert!(core::ptr::eq(pinned.as_ptr(), indices.as_ptr()));
    }
    assert!(
        summarize(ResidentValueRef::Bool(&[1, 0, 1]), 3)
            .unwrap()
            .unique
    );
    for view in [
        ResidentValueRef::Index(&[0, 1]),
        ResidentValueRef::Index(&[6]),
        ResidentValueRef::Bool(&[1, 2, 0]),
        ResidentValueRef::Bool(&[1]),
        ResidentValueRef::F64(&[f64::NAN]),
        ResidentValueRef::F64(&[-1.0]),
    ] {
        assert_eq!(
            summarize(view, 5).map(|s| s.count),
            selector_access_count(view, 5)
        );
    }
}

#[test]
fn selected_string_preview_requires_same_source_and_exact_proven_cardinality_without_spending() {
    let f = fixture();
    let mut meter = ResidentBudgetMeter::for_closed_analysis(1_000_000, 0);
    let measured = measure_source(&mut meter, &f.source, &f.schemas).unwrap();
    let before = meter;
    let all = plan(&f, ResolvedSelectionMode::LinearGather, 0);
    let clone = f.source.clone();
    for (source, witness, count, dimensions) in [
        (&f.source, None, 12, (3, 4)),
        (&clone, Some(&measured), 12, (3, 4)),
        (&f.source, Some(&measured), 2, (3, 4)),
        (&f.source, Some(&measured), usize::MAX, (3, 4)),
        (&f.source, Some(&measured), 12, (2, 4)),
    ] {
        assert!(
            !try_begin(
                &mut meter,
                source,
                witness,
                count,
                &all,
                dimensions,
                &[None, None]
            )
            .unwrap()
        );
        assert_eq!(meter, before);
    }
    let rows = plan(&f, ResolvedSelectionMode::Rows, 1);
    assert!(
        !try_begin(
            &mut meter,
            &f.source,
            Some(&measured),
            8,
            &rows,
            (3, 4),
            &[None, None]
        )
        .unwrap()
    );
    assert_eq!(meter, before);
    assert!(
        try_begin(
            &mut meter,
            &f.source,
            Some(&measured),
            12,
            &all,
            (3, 4),
            &[None, None]
        )
        .unwrap()
    );
    assert_eq!(
        meter.estimate().compute_work() - before.estimate().compute_work(),
        12 * 64
    );
    assert_eq!(
        meter.estimate().retained_nodes() - before.estimate().retained_nodes(),
        12
    );
}
