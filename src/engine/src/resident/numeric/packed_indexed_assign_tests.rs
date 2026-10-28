use super::*;
use crate::resident::budget::{
    KernelCostEstimate, ResidentBudgetMeter, measure_canonical_value_footprint,
};

fn matrix(element: SchemaBody, rows: u64, columns: u64) -> SchemaBody {
    SchemaBody::Matrix {
        element: Box::new(element),
        dimensions: Box::new([
            mech_core::DimensionExpr::Constant(rows),
            mech_core::DimensionExpr::Constant(columns),
        ]),
    }
}

fn value(body: SchemaBody, data: ValueDataDraft) -> (mech_core::Value, mech_core::SchemaTable) {
    let mut builder = mech_core::SchemaTableBuilder::new();
    let handle = builder
        .insert(
            mech_core::SchemaDraft {
                body,
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let build = builder.finish().unwrap();
    let schema = build.resolve(handle).unwrap();
    let value = mech_core::ValueDraft {
        schema,
        shape_values: Box::new([]),
        data,
    }
    .finalize(&SnapshotValidationContext::new(&build.table))
    .unwrap();
    (value, build.table)
}

fn projection(
    meter: &mut ResidentBudgetMeter,
    current: &mech_core::Value,
    schemas: &mech_core::SchemaTable,
    source: &mech_core::Value,
    source_schemas: &mech_core::SchemaTable,
    fast: bool,
    traversal: u64,
    selected: usize,
) -> (Result<(ValueFootprint, u64), ResidentKernelError>, usize) {
    let footprint = measure_canonical_value_footprint(
        &mut ResidentBudgetMeter::for_closed_analysis(1_000_000, 1_000_000),
        current,
        schemas,
    )
    .unwrap();
    let mut visits = 0;
    let result = super::super::projected_snapshot_matrix_output(
        meter,
        schemas.get(current.schema()).unwrap().body(),
        current.data(),
        footprint,
        source_schemas.get(source.schema()).unwrap().body(),
        source.data(),
        6,
        fast.then_some(6 * traversal),
        |destination, meter| {
            visits += 1;
            meter.charge_compute_work(traversal)?;
            Ok((destination == selected).then_some(0))
        },
    );
    (result, visits)
}

#[test]
fn single_projection_matches_incremental_cost_and_every_finite_prefix() {
    for (width, word) in [
        (IntegerWidth::W8, ValueDataDraft::U8(255)),
        (IntegerWidth::W16, ValueDataDraft::U16(u16::MAX)),
        (IntegerWidth::W32, ValueDataDraft::U32(u32::MAX)),
        (IntegerWidth::W64, ValueDataDraft::U64(u64::MAX)),
        (IntegerWidth::W128, ValueDataDraft::U128(u128::MAX)),
    ] {
        let element = SchemaBody::UnsignedInteger(width);
        let (current, schemas) = value(
            matrix(element.clone(), 2, 3),
            ValueDataDraft::Matrix(vec![word.clone(); 6].into()),
        );
        for packed_source in [false, true] {
            let (source, source_schemas) = if packed_source {
                value(
                    matrix(element.clone(), 1, 1),
                    ValueDataDraft::Matrix(vec![word.clone()].into()),
                )
            } else {
                value(element.clone(), word.clone())
            };
            for (traversal, selected) in [(1, 0), (1, 4), (6, 5)] {
                let mut complete = ResidentBudgetMeter::for_closed_analysis(1_000_000, 0);
                assert!(
                    projection(
                        &mut complete,
                        &current,
                        &schemas,
                        &source,
                        &source_schemas,
                        false,
                        traversal,
                        selected
                    )
                    .0
                    .is_ok()
                );
                let exact = complete.estimate().compute_work();
                for limit in 0..=exact + 1 {
                    let mut old = ResidentBudgetMeter::for_closed_analysis(limit, 0);
                    let mut new = old;
                    let old_result = projection(
                        &mut old,
                        &current,
                        &schemas,
                        &source,
                        &source_schemas,
                        false,
                        traversal,
                        selected,
                    );
                    let new_result = projection(
                        &mut new,
                        &current,
                        &schemas,
                        &source,
                        &source_schemas,
                        true,
                        traversal,
                        selected,
                    );
                    assert_eq!(
                        old_result.0, new_result.0,
                        "{width:?}, packed={packed_source}, limit={limit}"
                    );
                    assert_eq!(old.estimate(), new.estimate());
                    if limit >= exact {
                        assert_eq!(old_result.1, 6);
                        assert_eq!(
                            new_result.1, 0,
                            "the accepted plan must avoid the destination walk"
                        );
                    } else {
                        assert_eq!(old_result.1, new_result.1);
                    }
                }
            }
        }
    }
}

#[cfg(feature = "native-artifact")]
#[test]
fn single_projection_keeps_live_work_denials_and_spent_prefix() {
    use crate::resident::budget::{
        NativeLibraryWorkBudget, detached_turn_plan, with_resident_turn_plan,
    };
    let element = SchemaBody::UnsignedInteger(IntegerWidth::W32);
    let (current, schemas) = value(
        matrix(element.clone(), 2, 3),
        ValueDataDraft::Matrix(vec![ValueDataDraft::U32(0); 6].into()),
    );
    let (source, source_schemas) = value(element, ValueDataDraft::U32(7));
    let mut complete = ResidentBudgetMeter::for_closed_analysis(1_000_000, 0);
    projection(
        &mut complete,
        &current,
        &schemas,
        &source,
        &source_schemas,
        false,
        1,
        4,
    )
    .0
    .unwrap();
    for limit in 0..=complete.estimate().compute_work() + 1 {
        let run = |fast| {
            let memory = mech_core::ManagedMemoryBudget::new(4096);
            let budget = NativeLibraryWorkBudget::new(limit, 0, &memory).unwrap();
            let mut plan = detached_turn_plan(mech_core::ResourceDemand::default()).unwrap();
            plan.budget_limits.max_compute_work = Some(limit);
            let mut meter = ResidentBudgetMeter::default();
            let result = with_resident_turn_plan(plan, || {
                budget
                    .with_scope(|| {
                        projection(
                            &mut meter,
                            &current,
                            &schemas,
                            &source,
                            &source_schemas,
                            fast,
                            1,
                            4,
                        )
                        .0
                    })
                    .unwrap()
            });
            (result, meter.estimate(), budget.used())
        };
        assert_eq!(run(false), run(true), "live limit={limit}");
    }
}

#[test]
fn single_projection_declines_wrong_kinds_and_overflow_without_spending() {
    let current = [0u32; 6];
    let element = SchemaBody::UnsignedInteger(IntegerWidth::W32);
    for (schema, source, scan) in [
        (
            SchemaBody::UnsignedInteger(IntegerWidth::W16),
            ValueData::U16(7),
            6,
        ),
        (element.clone(), ValueData::U32(7), u64::MAX),
        (SchemaBody::Bool, ValueData::Bool(true), 6),
    ] {
        let mut meter = ResidentBudgetMeter::for_closed_analysis(1_000_000, 0);
        assert_eq!(
            try_project_single_source(
                &mut meter,
                &element,
                SequenceView::U32(&current),
                &schema,
                &source,
                scan
            ),
            Ok(false)
        );
        assert_eq!(meter.estimate(), KernelCostEstimate::default());
    }
}
