use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mech_core::{
    CommittedValueStateDelta, LegacyValue, MechMap, MechRecord, MechSet, Ref, ToMatrix,
    ValueStateJournal,
};
use std::hint::black_box;

const HASHED_COLLECTION_SIZE: usize = 64;
const PHASE_SCALARS: usize = 64;
const VALUE_MATRIX_SIDE: usize = 32;

fn scalar_roots(count: usize) -> Vec<LegacyValue> {
    (0..count)
        .map(|index| LegacyValue::F64(Ref::new(index as f64)))
        .collect()
}

fn capture_roots(roots: &[LegacyValue]) -> ValueStateJournal {
    let mut journal = ValueStateJournal::new();
    for root in roots {
        journal.capture_value(root).unwrap();
    }
    journal
}

fn mutate_scalars(roots: &[LegacyValue]) {
    for (index, root) in roots.iter().enumerate() {
        match root {
            LegacyValue::F64(value) => *value.borrow_mut() = 10_000.0 + index as f64,
            _ => unreachable!("scalar benchmark roots are f64 values"),
        }
    }
}

fn captured_mutated_scalars(count: usize) -> ValueStateJournal {
    let roots = scalar_roots(count);
    let journal = capture_roots(&roots);
    mutate_scalars(&roots);
    journal
}

fn scalar_delta(count: usize) -> CommittedValueStateDelta {
    let mut journal = captured_mutated_scalars(count);
    journal.record_after().unwrap();
    journal.into_delta().unwrap()
}

fn scalar_set_root(count: usize) -> (LegacyValue, Vec<Ref<f64>>) {
    let cells = (0..count)
        .map(|index| Ref::new(index as f64))
        .collect::<Vec<_>>();
    let members = cells
        .iter()
        .map(|cell| LegacyValue::F64(cell.clone()))
        .collect();
    (
        LegacyValue::Set(Ref::new(MechSet::from_vec(members))),
        cells,
    )
}

fn scalar_map_root(count: usize) -> (LegacyValue, Vec<Ref<f64>>) {
    let cells = (0..count)
        .map(|index| Ref::new(index as f64))
        .collect::<Vec<_>>();
    let entries = cells
        .iter()
        .enumerate()
        .map(|(index, cell)| {
            (
                LegacyValue::F64(cell.clone()),
                LegacyValue::Id(index as u64),
            )
        })
        .collect();
    (
        LegacyValue::Map(Ref::new(MechMap::from_vec(entries))),
        cells,
    )
}

fn hashed_set_journal() -> ValueStateJournal {
    let (root, cells) = scalar_set_root(HASHED_COLLECTION_SIZE);
    let journal = capture_roots(&[root]);
    *cells[HASHED_COLLECTION_SIZE / 2].borrow_mut() = 10_000.0;
    journal
}

fn hashed_set_delta() -> CommittedValueStateDelta {
    let mut journal = hashed_set_journal();
    journal.record_after().unwrap();
    journal.into_delta().unwrap()
}

fn hashed_map_journal() -> ValueStateJournal {
    let (root, cells) = scalar_map_root(HASHED_COLLECTION_SIZE);
    let journal = capture_roots(&[root]);
    *cells[HASHED_COLLECTION_SIZE / 2].borrow_mut() = 10_000.0;
    journal
}

fn hashed_map_delta() -> CommittedValueStateDelta {
    let mut journal = hashed_map_journal();
    journal.record_after().unwrap();
    journal.into_delta().unwrap()
}

fn nested_shared_record() -> LegacyValue {
    let shared = Ref::new(1.0);
    let inner = LegacyValue::Record(Ref::new(MechRecord::new(vec![
        ("left", LegacyValue::F64(shared.clone())),
        ("right", LegacyValue::F64(shared.clone())),
    ])));
    LegacyValue::Record(Ref::new(MechRecord::new(vec![
        ("direct", LegacyValue::F64(shared)),
        ("nested", inner),
    ])))
}

fn dynamic_f64_matrix() -> LegacyValue {
    LegacyValue::MatrixF64(<f64 as ToMatrix>::to_matrixd(
        vec![1.0; 100 * 100],
        100,
        100,
    ))
}

fn nested_value_matrix() -> LegacyValue {
    let elements = (0..VALUE_MATRIX_SIDE * VALUE_MATRIX_SIDE)
        .map(|index| LegacyValue::F64(Ref::new(index as f64)))
        .collect();
    LegacyValue::MatrixValue(<LegacyValue as ToMatrix>::to_matrixd(
        elements,
        VALUE_MATRIX_SIDE,
        VALUE_MATRIX_SIDE,
    ))
}

fn topology_journal() -> ValueStateJournal {
    let removed = Ref::new(1.0);
    let retained = Ref::new(2.0);
    let added = Ref::new(3.0);
    let record = Ref::new(MechRecord::new(vec![
        ("removed", LegacyValue::F64(removed)),
        ("retained", LegacyValue::F64(retained.clone())),
    ]));
    let mut journal = ValueStateJournal::new();
    journal
        .capture_value(&LegacyValue::Record(record.clone()))
        .unwrap();

    *record.borrow_mut() = MechRecord::new(vec![
        ("retained", LegacyValue::F64(retained.clone())),
        ("added", LegacyValue::F64(added)),
    ]);
    *retained.borrow_mut() = 20.0;
    journal
}

fn topology_delta() -> CommittedValueStateDelta {
    let mut journal = topology_journal();
    journal.record_after().unwrap();
    journal.into_delta().unwrap()
}

fn direct_scalar_mutation(c: &mut Criterion) {
    let cell = Ref::new(0.0);
    c.bench_function("value_state_journal/direct_scalar/mutation_baseline", |b| {
        b.iter(|| {
            *cell.borrow_mut() = black_box(1.0);
            black_box(*cell.borrow())
        })
    });
}

fn capture_scalars(c: &mut Criterion) {
    let mut group = c.benchmark_group("value_state_journal/capture_scalars");
    for count in [1usize, 64, 1_024] {
        let roots = scalar_roots(count);
        assert_eq!(capture_roots(&roots).cell_count(), count);
        group.throughput(Throughput::Elements(count as u64));
        let batch_size = if count == 1_024 {
            BatchSize::LargeInput
        } else {
            BatchSize::SmallInput
        };
        group.bench_with_input(BenchmarkId::new("capture", count), &count, |b, _| {
            b.iter_batched(
                ValueStateJournal::new,
                |mut journal| {
                    for root in black_box(&roots) {
                        journal.capture_value(root).unwrap();
                    }
                    black_box(journal)
                },
                batch_size,
            )
        });
    }
    group.finish();
}

fn scalar_phases(c: &mut Criterion) {
    assert_eq!(
        capture_roots(&scalar_roots(PHASE_SCALARS)).cell_count(),
        PHASE_SCALARS
    );
    assert_eq!(scalar_delta(PHASE_SCALARS).cell_count(), PHASE_SCALARS);

    let mut group = c.benchmark_group("value_state_journal/phases_64_scalars");
    group.throughput(Throughput::Elements(PHASE_SCALARS as u64));

    group.bench_function("capture_and_restore", |b| {
        b.iter_batched(
            || scalar_roots(PHASE_SCALARS),
            |roots| {
                let journal = capture_roots(&roots);
                mutate_scalars(&roots);
                journal.restore_before().unwrap();
                black_box((journal, roots))
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("restore_before", |b| {
        b.iter_batched(
            || captured_mutated_scalars(PHASE_SCALARS),
            |journal| {
                journal.restore_before().unwrap();
                black_box(journal)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("record_after", |b| {
        b.iter_batched(
            || captured_mutated_scalars(PHASE_SCALARS),
            |mut journal| {
                journal.record_after().unwrap();
                black_box(journal)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("rewind", |b| {
        b.iter_batched(
            || scalar_delta(PHASE_SCALARS),
            |delta| {
                delta.rewind().unwrap();
                black_box(delta)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("replay", |b| {
        b.iter_batched(
            || {
                let delta = scalar_delta(PHASE_SCALARS);
                delta.rewind().unwrap();
                delta
            },
            |delta| {
                delta.replay().unwrap();
                black_box(delta)
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn hashed_collection_group(
    c: &mut Criterion,
    name: &str,
    root: LegacyValue,
    journal_factory: fn() -> ValueStateJournal,
    delta_factory: fn() -> CommittedValueStateDelta,
) {
    let expected_cells = 1 + HASHED_COLLECTION_SIZE;
    assert_eq!(capture_roots(&[root.clone()]).cell_count(), expected_cells);
    assert_eq!(journal_factory().cell_count(), expected_cells);
    assert_eq!(delta_factory().cell_count(), expected_cells);

    let mut group = c.benchmark_group(name);
    group.throughput(Throughput::Elements(HASHED_COLLECTION_SIZE as u64));

    group.bench_function("capture", |b| {
        b.iter_batched(
            ValueStateJournal::new,
            |mut journal| {
                journal.capture_value(black_box(&root)).unwrap();
                black_box(journal)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("record_after", |b| {
        b.iter_batched(
            journal_factory,
            |mut journal| {
                journal.record_after().unwrap();
                black_box(journal)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("rewind", |b| {
        b.iter_batched(
            delta_factory,
            |delta| {
                delta.rewind().unwrap();
                black_box(delta)
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("replay", |b| {
        b.iter_batched(
            || {
                let delta = delta_factory();
                delta.rewind().unwrap();
                delta
            },
            |delta| {
                delta.replay().unwrap();
                black_box(delta)
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn hashed_collection_phases(c: &mut Criterion) {
    let (set_root, _) = scalar_set_root(HASHED_COLLECTION_SIZE);
    hashed_collection_group(
        c,
        "value_state_journal/hashed_set_64_scalars",
        set_root,
        hashed_set_journal,
        hashed_set_delta,
    );

    let (map_root, _) = scalar_map_root(HASHED_COLLECTION_SIZE);
    hashed_collection_group(
        c,
        "value_state_journal/hashed_map_64_scalar_keys",
        map_root,
        hashed_map_journal,
        hashed_map_delta,
    );
}

fn capture_graphs(c: &mut Criterion) {
    let nested_record = nested_shared_record();
    assert_eq!(capture_roots(&[nested_record.clone()]).cell_count(), 3);

    let matrix_f64 = dynamic_f64_matrix();
    assert_eq!(capture_roots(&[matrix_f64.clone()]).cell_count(), 1);

    let matrix_value = nested_value_matrix();
    assert_eq!(
        capture_roots(&[matrix_value.clone()]).cell_count(),
        1 + VALUE_MATRIX_SIDE * VALUE_MATRIX_SIDE
    );

    let mut group = c.benchmark_group("value_state_journal/capture_graphs");
    group.bench_function("nested_record_shared", |b| {
        b.iter_batched(
            ValueStateJournal::new,
            |mut journal| {
                journal.capture_value(black_box(&nested_record)).unwrap();
                black_box(journal)
            },
            BatchSize::SmallInput,
        )
    });

    group.throughput(Throughput::Elements(100 * 100));
    group.bench_function("dynamic_f64_matrix_100x100", |b| {
        b.iter_batched(
            ValueStateJournal::new,
            |mut journal| {
                journal.capture_value(black_box(&matrix_f64)).unwrap();
                black_box(journal)
            },
            BatchSize::LargeInput,
        )
    });

    group.throughput(Throughput::Elements(
        (VALUE_MATRIX_SIDE * VALUE_MATRIX_SIDE) as u64,
    ));
    group.bench_function("matrix_value_32x32", |b| {
        b.iter_batched(
            ValueStateJournal::new,
            |mut journal| {
                journal.capture_value(black_box(&matrix_value)).unwrap();
                black_box(journal)
            },
            BatchSize::LargeInput,
        )
    });
    group.finish();
}

fn topology_phases(c: &mut Criterion) {
    assert_eq!(topology_journal().cell_count(), 3);
    assert_eq!(topology_delta().cell_count(), 4);

    let mut group = c.benchmark_group("value_state_journal/topology_new_removed");
    group.bench_function("record_after", |b| {
        b.iter_batched(
            topology_journal,
            |mut journal| {
                journal.record_after().unwrap();
                black_box(journal)
            },
            BatchSize::SmallInput,
        )
    });
    group.bench_function("rewind", |b| {
        b.iter_batched(
            topology_delta,
            |delta| {
                delta.rewind().unwrap();
                black_box(delta)
            },
            BatchSize::SmallInput,
        )
    });
    group.bench_function("replay", |b| {
        b.iter_batched(
            || {
                let delta = topology_delta();
                delta.rewind().unwrap();
                delta
            },
            |delta| {
                delta.replay().unwrap();
                black_box(delta)
            },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

criterion_group!(
    benches,
    direct_scalar_mutation,
    capture_scalars,
    scalar_phases,
    hashed_collection_phases,
    capture_graphs,
    topology_phases
);
criterion_main!(benches);
