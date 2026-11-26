//! Differential checks against ordinary ordered numeric_sum and admission.
use super::*;

fn f64_cells(cells: impl IntoIterator<Item = u64>) -> Vec<ValueDataDraft> {
    cells
        .into_iter()
        .map(|bits| ValueDataDraft::F64(F64Bits::from_bits(bits)))
        .collect()
}

#[test]
fn direct_float_sum_preserves_ieee_bits_order_axes_tails_and_empty_shapes() {
    let element = SchemaBody::FloatingPoint(F::W64);
    let corpus = [
        0.0f64.to_bits(),
        (-0.0f64).to_bits(),
        1e16f64.to_bits(),
        1.0f64.to_bits(),
        (-1e16f64).to_bits(),
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
        0x7ff8_0000_0000_0042,
        0xfff8_0000_0000_0027,
        0x7ff0_0000_0000_0001,
        1,
        0x8000_0000_0000_0001,
        f64::MAX.to_bits(),
        (-f64::MAX).to_bits(),
    ];
    let finite = [
        0.0f64.to_bits(),
        (-0.0f64).to_bits(),
        1e16f64.to_bits(),
        1.0f64.to_bits(),
        (-1e16f64).to_bits(),
        1,
        0x8000_0000_0000_0001,
        f64::MAX.to_bits(),
        (-f64::MAX).to_bits(),
    ];
    let infinite = [
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
        0.0f64.to_bits(),
        (-0.0f64).to_bits(),
        1.0f64.to_bits(),
        (-2.0f64).to_bits(),
    ];
    for corpus in [&corpus[..], &finite[..], &infinite[..]] {
        for (rows, columns) in [
            (0, 0),
            (0, 7),
            (7, 0),
            (1, 1),
            (1, 9),
            (9, 1),
            (2, 3),
            (3, 5),
            (4, 4),
            (7, 11),
            (8, 12),
            (9, 13),
        ] {
            for column in [false, true] {
                let out = if column { (rows, 1) } else { (1, columns) };
                let (schemas, ids) = table([
                    matrix(&element, rows, columns),
                    matrix(&element, out.0, out.1),
                ]);
                let bound = kernel(sum_snapshot, &schemas, ids[1], vec![u64::from(column), 1]);
                let mut fast = [None];
                let mut old = [None];
                for offset in 0..corpus.len() {
                    let input = [Some(value(
                        &schemas,
                        ids[0],
                        f64_cells(
                            (0..rows * columns).map(|at| corpus[(at + offset) % corpus.len()]),
                        ),
                    ))];
                    let before = data(&input);
                    let args = Inputs(&[ResidentValueRef::Snapshot(&input)]);
                    // The second call also checks unchanged-publication reporting.
                    for _ in 0..2 {
                        direct_float_sum::constructions();
                        let a = bound.execute(&args, ResidentValueMut::Snapshot(&mut fast));
                        let selected = direct_float_sum::constructions();
                        let b = direct_float_sum::without(|| {
                            bound.execute(&args, ResidentValueMut::Snapshot(&mut old))
                        });
                        assert_eq!(a, b, "{rows}x{columns} column={column} offset={offset}");
                        assert!(a.is_ok());
                        let nan_input = (0..rows * columns).any(|at| {
                            f64::from_bits(corpus[(at + offset) % corpus.len()]).is_nan()
                        });
                        assert_eq!(
                            selected,
                            usize::from(!nan_input),
                            "actual guarded selection"
                        );
                        assert_eq!(
                            data(&fast),
                            data(&old),
                            "exact IEEE payloads and order: {rows}x{columns} column={column} offset={offset}"
                        );
                    }
                    assert_eq!(data(&input), before);
                }
            }
        }
    }
}

#[test]
fn direct_float_sum_keeps_finite_denial_atomic_publication_aliases_and_retry() {
    let element = SchemaBody::FloatingPoint(F::W64);
    let (rows, columns) = (7, 9);
    for column in [false, true] {
        let out = if column { (rows, 1) } else { (1, columns) };
        let (schemas, ids) = table([
            matrix(&element, rows, columns),
            matrix(&element, out.0, out.1),
        ]);
        let input = [Some(value(
            &schemas,
            ids[0],
            f64_cells((0..rows * columns).map(|at| ((at as f64 - 17.0) / 11.0).to_bits())),
        ))];
        let alias = input.clone();
        let args = Inputs(&[ResidentValueRef::Snapshot(&input)]);
        let bound = kernel(sum_snapshot, &schemas, ids[1], vec![u64::from(column), 1]);
        let mut prior = [None];
        bound
            .execute(&args, ResidentValueMut::Snapshot(&mut prior))
            .unwrap();
        let retained = prior.clone();
        for dimension in 0..8 {
            for cap in [0, 1, 16, 128, 4096, 65535, 1_000_000] {
                let mut plan = crate::resident::budget::detached_turn_plan(
                    mech_core::ResourceDemand::default(),
                )
                .unwrap();
                match dimension {
                    0 => plan.budget_limits.max_output_elements = Some(cap),
                    1 => plan.budget_limits.max_output_bytes = Some(cap),
                    2 => plan.budget_limits.max_temporary_bytes = Some(cap),
                    3 => plan.budget_limits.max_cloned_bytes = Some(cap),
                    4 => plan.budget_limits.max_retained_nodes = Some(cap),
                    5 => plan.budget_limits.max_compute_work = Some(cap),
                    6 => plan.budget_limits.max_comparison_work = Some(cap),
                    _ => plan.budget_limits.max_storage_buffer_bytes = Some(cap),
                }
                let mut fast = prior.clone();
                let mut old = prior.clone();
                let a = crate::resident::budget::with_resident_turn_plan(plan.clone(), || {
                    bound.execute(&args, ResidentValueMut::Snapshot(&mut fast))
                });
                let b = direct_float_sum::without(|| {
                    crate::resident::budget::with_resident_turn_plan(plan, || {
                        bound.execute(&args, ResidentValueMut::Snapshot(&mut old))
                    })
                });
                assert_eq!(a, b, "column={column} dimension={dimension} cap={cap}");
                assert_eq!(data(&fast), data(&old));
                if a.is_err() {
                    assert_eq!(data(&fast), data(&prior));
                }
                assert_eq!(data(&prior), data(&retained));
            }
        }
        let mut recovered = [None];
        bound
            .execute(&args, ResidentValueMut::Snapshot(&mut recovered))
            .unwrap();
        assert_eq!(data(&recovered), data(&prior));
        assert_eq!(data(&input), data(&alias));
    }
}

#[test]
fn direct_float_sum_unmatched_kind_and_storage_keep_the_ordinary_reduction() {
    for (element, cells) in cases()
        .into_iter()
        .filter(|(e, _)| !matches!(e, SchemaBody::Bool | SchemaBody::Index))
    {
        let (schemas, ids) = table([matrix(&element, 2, 3), matrix(&element, 1, 3)]);
        let input = [Some(value(&schemas, ids[0], cells))];
        let bound = kernel(sum_snapshot, &schemas, ids[1], vec![0, 1]);
        direct_float_sum::constructions();
        let mut a = [None];
        let mut b = [None];
        let args = Inputs(&[ResidentValueRef::Snapshot(&input)]);
        // Remove only typed borrowing; this preserves the packed publisher
        // and exercises the ordinary current-value read/reduction path.
        let x =
            borrowed_numeric::without(|| bound.execute(&args, ResidentValueMut::Snapshot(&mut a)));
        assert_eq!(direct_float_sum::constructions(), 0);
        let y =
            direct_float_sum::without(|| bound.execute(&args, ResidentValueMut::Snapshot(&mut b)));
        assert_eq!(x, y);
        assert_eq!(data(&a), data(&b));
    }
}
