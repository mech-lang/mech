use super::super::tests::{artifacts, catalog, limits, value};
use super::*;
use crate::{
    ProgramArtifact,
    resident::general::native_collection_calls::{
        NativePreparedLibraryCallCache, prepare_native_program_library_call,
    },
};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data, snapshot::MapEntryDraft};
fn source() -> &'static str {
    r#"
+> map
+> sequence
seed := dictionary<{string:u64}>
keys := map/keys(seed)
count := sequence/length(keys)
key := keys[position<u64>]
next := map/insert(seed,key,id<u64>)
(keys,count,next)
"#
}
fn prepared(a: &ProgramArtifact) -> Vec<NativePreparedLibraryCall> {
    let c = catalog();
    let p = crate::native_artifact::lower_native_program_artifact(a, &c).unwrap();
    p.blocks()
        .iter()
        .filter_map(|b| {
            let crate::native_artifact::NativeIterationAction::LibraryCall { template, .. } =
                b.action()
            else {
                return None;
            };
            Some(prepare_native_program_library_call(a, template, &c).unwrap())
        })
        .collect()
}
fn seed(
    a: &ProgramArtifact,
    call: &NativePreparedLibraryCall,
    texts: &[String],
    m: &ManagedMemoryBudget,
) -> Value {
    let mech_core::ResolvedOperationContract::Declared(c) = call.template.contract() else {
        unreachable!()
    };
    value(
        a,
        c.inputs[0].schema,
        Box::new([]),
        Data::Map(
            texts
                .iter()
                .enumerate()
                .map(|(i, s)| MapEntryDraft {
                    items: vec![Data::String(s.clone()), Data::U64(19 + i as u64)]
                        .into_boxed_slice(),
                })
                .collect(),
        ),
        m,
    )
}
fn ordinary(
    call: &NativePreparedLibraryCall,
    source: &Value,
    l: MemoryBudgetLimits,
    w: &NativeLibraryWorkBudget,
) -> Result<Value, InvokeError> {
    let mut output = None;
    call.invoke_current_cached(
        &mut NativePreparedLibraryCallCache::default(),
        &[Some(source.clone())],
        &mut output,
        l,
        w,
    )?;
    Ok(output.unwrap())
}
#[test]
fn borrowed_map_keys_pin_generic_snapshot_and_match_real_matrix_cells_and_logical_caps() {
    for a in artifacts(source()) {
        let calls = prepared(&a);
        let keys = calls
            .iter()
            .find(|c| c.template.primitive() == Primitive::StringMapKeys)
            .unwrap();
        let element = calls
            .iter()
            .find(|c| c.supports_string_matrix_key_loan())
            .unwrap();
        let insert = calls
            .iter()
            .find(|c| c.template.primitive() == Primitive::MapInsert)
            .unwrap();
        let m = ManagedMemoryBudget::new(64 << 20);
        let w = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &m).unwrap();
        let mut texts = vec![
            "".into(),
            "\0embedded\0".into(),
            "é🧑‍🚀".into(),
            "e\u{301}\r\n".into(),
            "long".repeat(1000),
        ];
        texts.extend((0..65).map(|i| format!("key-{i:03}-{}", "界".repeat(i))));
        texts.sort();
        let source = seed(&a, keys, &texts, &m);
        let canonical = ordinary(keys, &source, limits(), &w).unwrap();
        let mut matrix = element
            .try_string_matrix_key_loan(&canonical, limits(), &w, ResourceDemand::default())
            .unwrap()
            .unwrap();
        let mut view = keys
            .try_borrowed_map_keys(element, &source, limits(), &w, ResourceDemand::default())
            .unwrap()
            .unwrap();
        let mut meter = ResidentBudgetMeter::for_closed_analysis(u64::MAX, u64::MAX);
        assert_eq!(
            view.logical_footprint(),
            budget::measure_borrowed_canonical_value_footprint(&mut meter, &canonical, a.schemas())
                .unwrap()
        );
        assert_eq!(view.loan.cells.len(), matrix.cells.len());
        for (x, y) in view.loan.cells.iter().zip(matrix.cells.iter()) {
            assert_eq!(x.bytes, y.bytes);
            assert_eq!(x.logical_output_bytes, y.logical_output_bytes);
            assert_eq!(x.logical_cloned_bytes, y.logical_cloned_bytes);
            assert_eq!(x.selection_compute_work, y.selection_compute_work);
            assert_eq!(x.selection_comparison_work, y.selection_comparison_work);
            assert_eq!(x.logical_temporary_bytes, y.logical_temporary_bytes);
            assert_eq!(x.logical_extra_nodes, y.logical_extra_nodes);
        }
        // Original source arena ownership can retire: only this immutable pin is
        // required. A new dictionary with equal keys and different values is not
        // a replacement identity for that pin.
        drop(source);
        for (i, text) in texts.iter().enumerate() {
            let ordinal = view
                .admit_element(i as u64 + 1, limits(), &w, ResourceDemand::default())
                .unwrap();
            assert_eq!(
                view.with_element(insert, &ordinal, &w, str::to_owned)
                    .unwrap(),
                *text
            );
            assert_eq!(
                view.with_element_index(insert, &ordinal, &w, |index, text| (
                    index,
                    text.to_owned()
                ))
                .unwrap(),
                (i, text.clone())
            );
            matrix
                .admit_element(
                    &canonical,
                    i as u64 + 1,
                    limits(),
                    &w,
                    ResourceDemand::default(),
                )
                .unwrap();
        }
        for (dimension, exact) in [
            ("output", view.logical.retained_bytes),
            ("clone", texts.iter().map(|s| s.len() as u64).sum()),
        ] {
            for lower in [false, true] {
                let mut l = limits();
                let cap = exact - u64::from(lower);
                if dimension == "output" {
                    l.max_output_bytes = Some(cap)
                } else {
                    l.max_cloned_bytes = Some(cap)
                };
                let old = ordinary(keys, view.snapshot(), l, &w);
                let new = keys.try_borrowed_map_keys(
                    element,
                    view.snapshot(),
                    l,
                    &w,
                    ResourceDemand::default(),
                );
                assert_eq!(
                    old.is_ok(),
                    new.is_ok(),
                    "{dimension} lower={lower}: {old:?} {new:?}"
                );
                if lower {
                    let e = if dimension == "output" {
                        "OutputBytes"
                    } else {
                        "ClonedBytes"
                    };
                    assert!(format!("{old:?}").contains(e));
                    assert!(format!("{new:?}").contains(e));
                }
            }
        }
        let foreign = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &m).unwrap();
        assert!(view.native_view(&foreign).is_err());
        drop((foreign, matrix, view, canonical, w));
        assert_eq!((m.used_bytes(), m.retained_snapshot_nodes()), (0, 0));
    }
}
#[test]
fn borrowed_map_keys_frame_admission_accepts_sealed_shape_and_preserves_output_limit() {
    for a in artifacts(source()) {
        let calls = prepared(&a);
        let keys = calls
            .iter()
            .find(|c| c.template.primitive() == Primitive::StringMapKeys)
            .unwrap();
        let element = calls
            .iter()
            .find(|c| c.supports_string_matrix_key_loan())
            .unwrap();
        let pack = calls
            .iter()
            .find(|c| c.template.primitive() == Primitive::CompositePack(3))
            .unwrap();
        let m = ManagedMemoryBudget::new(64 << 20);
        let w = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &m).unwrap();
        let source = seed(&a, keys, &["one".into(), "é".into()], &m);
        let canonical = ordinary(keys, &source, limits(), &w).unwrap();
        let view = keys
            .try_borrowed_map_keys(element, &source, limits(), &w, ResourceDemand::default())
            .unwrap()
            .unwrap();
        let words = [0, 2, 0];
        let mut meter = ResidentBudgetMeter::for_closed_analysis(u64::MAX, u64::MAX);
        let scalar = budget::measure_native_scalar_frame_footprint(
            &mut meter,
            &SchemaBody::UnsignedInteger(IntegerWidth::W64),
            NativeScalarValue::U64(2),
        )
        .unwrap();
        let map =
            budget::measure_borrowed_canonical_value_footprint(&mut meter, &source, a.schemas())
                .unwrap();
        let exact = view.logical.retained_bytes + scalar.retained_bytes + map.retained_bytes;
        for cap in [exact, exact - 1] {
            let mut l = limits();
            l.max_output_bytes = Some(cap);
            let old = pack.admit_buffer_frame_words(
                &[Some(canonical.clone()), None, Some(source.clone())],
                &words,
                l,
                &w,
                ResourceDemand::default(),
            );
            let new = pack.admit_buffer_frame_borrowed_map_keys(
                &[None, None, Some(source.clone())],
                &words,
                0,
                &view,
                l,
                &w,
                ResourceDemand::default(),
            );
            assert_eq!(old.is_ok(), new.is_ok(), "{cap}: {old:?} {new:?}");
            assert_eq!(new.is_ok(), cap == exact);
        }
        assert!(
            pack.admit_buffer_frame_borrowed_map_keys(
                &[None, None, Some(source.clone())],
                &words,
                1,
                &view,
                limits(),
                &w,
                ResourceDemand::default()
            )
            .is_err()
        );
        drop((view, canonical, source, w));
        assert_eq!((m.used_bytes(), m.retained_snapshot_nodes()), (0, 0));
    }
}

#[test]
fn borrowed_map_keys_decline_headroom_preserve_empty_bounds_and_reject_replayed_selection() {
    for a in artifacts(source()) {
        let calls = prepared(&a);
        let keys = calls
            .iter()
            .find(|c| c.template.primitive() == Primitive::StringMapKeys)
            .unwrap();
        let element = calls
            .iter()
            .find(|c| c.supports_string_matrix_key_loan())
            .unwrap();
        let length = calls
            .iter()
            .find(|c| c.template.primitive() == Primitive::SequenceLength)
            .unwrap();
        let insert = calls
            .iter()
            .find(|c| c.template.primitive() == Primitive::MapInsert)
            .unwrap();
        let m = ManagedMemoryBudget::new(64 << 20);
        let w = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &m).unwrap();
        for texts in [vec![], vec![String::from("one"), String::from("two")]] {
            let source = seed(&a, keys, &texts, &m);
            let prior = (m.used_bytes(), m.retained_snapshot_nodes());
            let mut l = limits();
            l.max_temporary_bytes = Some(0);
            assert!(
                keys.try_borrowed_map_keys(element, &source, l, &w, ResourceDemand::default())
                    .unwrap()
                    .is_none()
            );
            assert_eq!((m.used_bytes(), m.retained_snapshot_nodes()), prior);
            let mut l = limits();
            l.max_compute_work = Some(0);
            assert!(
                keys.try_borrowed_map_keys(element, &source, l, &w, ResourceDemand::default())
                    .is_err()
            );
            assert_eq!((m.used_bytes(), m.retained_snapshot_nodes()), prior);
            let mut view = keys
                .try_borrowed_map_keys(element, &source, limits(), &w, ResourceDemand::default())
                .unwrap()
                .unwrap();
            assert_eq!(
                length
                    .borrowed_map_keys_length(&view, limits(), &w, ResourceDemand::default())
                    .unwrap(),
                texts.len() as u64
            );
            for (dimension, cap) in [("elements", 0), ("bytes", 7)] {
                let mut l = limits();
                if dimension == "elements" {
                    l.max_output_elements = Some(cap)
                } else {
                    l.max_output_bytes = Some(cap)
                };
                assert!(
                    length
                        .borrowed_map_keys_length(&view, l, &w, ResourceDemand::default())
                        .is_err()
                );
            }
            for index in [0, texts.len() as u64 + 1, u64::MAX] {
                assert!(
                    view.admit_element(index, limits(), &w, ResourceDemand::default())
                        .is_err()
                );
            }
            if !texts.is_empty() {
                let old = view
                    .admit_element(1, limits(), &w, ResourceDemand::default())
                    .unwrap();
                let current = view
                    .admit_element(2, limits(), &w, ResourceDemand::default())
                    .unwrap();
                assert!(view.with_element(insert, &old, &w, str::to_owned).is_err());
                assert_eq!(
                    view.with_element(insert, &current, &w, str::to_owned)
                        .unwrap(),
                    "two"
                );
            }
            drop(view);
            assert_eq!((m.used_bytes(), m.retained_snapshot_nodes()), prior);
            drop(source);
        }
        drop(w);
        assert_eq!((m.used_bytes(), m.retained_snapshot_nodes()), (0, 0));
    }
}

#[test]
fn borrowed_map_keys_deferred_materialization_preserves_caps_and_paid_prefix() {
    for a in artifacts(source()) {
        let calls = prepared(&a);
        let keys = calls
            .iter()
            .find(|c| c.template.primitive() == Primitive::StringMapKeys)
            .unwrap();
        let element = calls
            .iter()
            .find(|c| c.supports_string_matrix_key_loan())
            .unwrap();
        let mut texts = vec!["".into(), "a\0b".into(), "é🧑‍🚀".into(), "long".repeat(400)];
        texts.sort();
        // Fresh accounts and owners make each boundary independent. Only the
        // original source call's work counts toward its relative cap; another
        // call's 37 units remain in the shared cumulative account instead.
        let run = |cap: u64, total: u64, other: u64| {
            let m = ManagedMemoryBudget::new(64 << 20);
            let w = NativeLibraryWorkBudget::new(total, u64::MAX, &m).unwrap();
            let mut input = Some(seed(&a, keys, &texts, &m));
            let mut l = limits();
            l.max_compute_work = Some(cap);
            let choice = keys.borrowed_map_keys_or_canonical(
                element,
                input.as_ref().unwrap(),
                l,
                &w,
                ResourceDemand::default(),
            );
            let (result, prefix, chosen) = match choice {
                Ok(NativeBorrowedMapKeysChoice::View(view)) => {
                    let prefix = view.source_spent.0;
                    // The original arena owner is really gone before fallback.
                    drop(input.take());
                    let result = w
                        .charge(other, 0)
                        .map_err(InvokeError::Kernel)
                        .and_then(|()| keys.materialize_borrowed_map_keys(view, &w));
                    (result, prefix, true)
                }
                Ok(NativeBorrowedMapKeysChoice::Canonical(value)) => (Ok(value), 0, false),
                Err(error) => (Err(error), 0, false),
            };
            drop(input);
            let used = w.used().0;
            let ok = result.is_ok();
            if let Ok(value) = &result {
                let ValueData::Matrix(matrix) = value.data() else {
                    panic!("Matrix")
                };
                let SequenceView::String(values) = matrix.elements() else {
                    panic!("Strings")
                };
                assert_eq!(
                    values.iter().map(|s| s.as_ref()).collect::<Vec<_>>(),
                    texts.iter().map(String::as_str).collect::<Vec<_>>()
                );
            }
            let error = format!("{result:?}");
            drop((result, w));
            assert_eq!((m.used_bytes(), m.retained_snapshot_nodes()), (0, 0));
            (ok, used, prefix, chosen, error)
        };
        let full = run(u64::MAX, u64::MAX, 0);
        assert!(full.0 && full.3, "{full:?}");
        let mut low = 0;
        let mut high = full.1 * 2;
        while low < high {
            let mid = low + (high - low) / 2;
            if run(mid, u64::MAX, 0).0 {
                high = mid;
            } else {
                low = mid + 1;
            }
        }
        let exact = run(low, u64::MAX, 0);
        let denied = run(low - 1, u64::MAX, 0);
        assert!(exact.0 && exact.3);
        assert!(!denied.0 && denied.3, "{denied:?}");
        assert!(denied.4.contains("ComputeWork"), "{denied:?}");
        assert!(denied.1 <= low - 1);
        assert!(exact.2 > 0 && exact.1 > exact.2);
        let with_other = run(low, u64::MAX, 37);
        assert!(with_other.0 && with_other.3, "{with_other:?}");
        assert_eq!(with_other.1, exact.1 + 37);
        let global_denied = run(u64::MAX, full.1 + 37 - 1, 37);
        assert!(!global_denied.0 && global_denied.3, "{global_denied:?}");
        assert!(global_denied.4.contains("ComputeWork"));
    }
}

#[test]
fn borrowed_map_keys_nested_and_immediate_refusal_keep_original_canonical_scope() {
    for a in artifacts(source()) {
        let calls = prepared(&a);
        let keys = calls
            .iter()
            .find(|c| c.template.primitive() == Primitive::StringMapKeys)
            .unwrap();
        let element = calls
            .iter()
            .find(|c| c.supports_string_matrix_key_loan())
            .unwrap();
        let length = calls
            .iter()
            .find(|c| c.template.primitive() == Primitive::SequenceLength)
            .unwrap();
        let m = ManagedMemoryBudget::new(64 << 20);
        let w = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &m).unwrap();
        let input = seed(&a, keys, &["one".into(), "two".into()], &m);
        let before = w.used();
        // A wrong optional element specialization must use the original call,
        // not publish a fake matrix or restart a second independent allowance.
        let canonical = keys
            .borrowed_map_keys_or_canonical(length, &input, limits(), &w, ResourceDemand::default())
            .unwrap();
        assert!(matches!(
            canonical,
            NativeBorrowedMapKeysChoice::Canonical(_)
        ));
        let ordinary = w.used().0 - before.0;
        drop(canonical);
        let nested = w
            .with_call_limits(u64::MAX, u64::MAX, || {
                keys.borrowed_map_keys_or_canonical(
                    element,
                    &input,
                    limits(),
                    &w,
                    ResourceDemand::default(),
                )
            })
            .unwrap()
            .unwrap();
        assert!(matches!(nested, NativeBorrowedMapKeysChoice::Canonical(_)));
        drop(nested);
        let denied = w
            .with_call_limits(ordinary - 1, u64::MAX, || {
                keys.borrowed_map_keys_or_canonical(
                    element,
                    &input,
                    limits(),
                    &w,
                    ResourceDemand::default(),
                )
            })
            .unwrap();
        assert!(format!("{denied:?}").contains("ComputeWork"), "{denied:?}");
        drop(denied);
        drop((input, w));
        assert_eq!((m.used_bytes(), m.retained_snapshot_nodes()), (0, 0));
    }
}

#[test]
fn borrowed_map_keys_publication_rejects_foreign_factory_and_account_and_keeps_later_reads() {
    for a in artifacts(source()) {
        let calls = prepared(&a);
        let other_calls = prepared(&a);
        let keys = calls
            .iter()
            .find(|c| c.template.primitive() == Primitive::StringMapKeys)
            .unwrap();
        let other_keys = other_calls
            .iter()
            .find(|c| c.template.primitive() == Primitive::StringMapKeys)
            .unwrap();
        let element = calls
            .iter()
            .find(|c| c.supports_string_matrix_key_loan())
            .unwrap();
        let length = calls
            .iter()
            .find(|c| c.template.primitive() == Primitive::SequenceLength)
            .unwrap();
        let m = ManagedMemoryBudget::new(64 << 20);
        let w = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &m).unwrap();
        let foreign = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &m).unwrap();
        let input = seed(&a, keys, &["one".into(), "two".into()], &m);
        let make = || {
            keys.try_borrowed_map_keys(element, &input, limits(), &w, ResourceDemand::default())
                .unwrap()
                .unwrap()
        };
        assert!(
            other_keys
                .materialize_borrowed_map_keys(make(), &w)
                .is_err()
        );
        assert!(
            keys.materialize_borrowed_map_keys(make(), &foreign)
                .is_err()
        );
        let view = make();
        drop(input);
        // Insertion-owner retirement does not own this independent snapshot.
        // A later length remains valid until the source itself materializes.
        assert_eq!(
            length
                .borrowed_map_keys_length(&view, limits(), &w, ResourceDemand::default())
                .unwrap(),
            2
        );
        let value = keys.materialize_borrowed_map_keys(view, &w).unwrap();
        assert!(matches!(value.data(), ValueData::Matrix(_)));
        drop((value, foreign, w));
        assert_eq!((m.used_bytes(), m.retained_snapshot_nodes()), (0, 0));
    }
}
