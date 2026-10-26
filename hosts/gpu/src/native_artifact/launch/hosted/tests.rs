use crate::native_artifact::launch::hosted::*;
#[test]
fn exact_acquisition_limit_is_not_truncation() {
    let mut input = std::io::Cursor::new(b"abcd");
    assert!(bounded(&mut input, 3).is_err());
    assert_eq!(input.position(), 4);
    assert_eq!(bounded(std::io::Cursor::new(b"abc"), 3).unwrap(), b"abc");
    assert_eq!(bounded(std::io::Cursor::new(b""), 0).unwrap(), b"");
    assert!(bounded(std::io::Cursor::new(b"x"), 0).is_err());
    assert!(bounded(std::io::Cursor::new(b""), u64::MAX).is_err());
}

#[test]
fn payload_aggregate_cap_preserves_unicode_and_empty_arguments() {
    let memory = ManagedMemoryBudget::new(4096);
    let arguments = || vec![OsString::from("é"), OsString::new(), OsString::from("abc")];
    let exact = bounded_arguments(arguments().into_iter(), 5, &memory).unwrap();
    assert_eq!(exact.arguments, arguments());
    assert_eq!(exact._reservation.budget(), memory);
    drop(exact);
    assert_eq!(memory.used_bytes(), 0);
    let error = bounded_arguments(arguments().into_iter(), 4, &memory)
        .err()
        .unwrap();
    assert!(error.contains("finite aggregate acquisition cap"));
    assert_eq!(memory.used_bytes(), 0);
    let empty = bounded_arguments(Vec::new().into_iter(), 0, &memory).unwrap();
    assert!(empty.arguments.is_empty());
    drop(empty);
    let empty_argument = bounded_arguments(vec![OsString::new()].into_iter(), 0, &memory).unwrap();
    assert_eq!(empty_argument.arguments, [OsString::new()]);
    drop(empty_argument);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn result_carriers_reject_spare_capacity_and_drop_before_live_claim() {
    struct Probe<'a> {
        memory: &'a ManagedMemoryBudget,
        dropped: &'a std::cell::Cell<usize>,
    }
    impl Drop for Probe<'_> {
        fn drop(&mut self) {
            assert_eq!(self.memory.used_bytes(), 128);
            self.dropped.set(self.dropped.get() + 1);
        }
    }
    let memory = ManagedMemoryBudget::new(256);
    let claim = memory.reserve_capacity(128).unwrap();
    let dropped = std::cell::Cell::new(0);
    let mut values = Vec::with_capacity(3);
    values.push(Probe {
        memory: &memory,
        dropped: &dropped,
    });
    // A deterministic spare-capacity response, independent of allocator
    // rounding behavior, passes through the SAME production owned guard.
    let error = exact_capacity(values, 2, "carrier capacity").err().unwrap();
    assert_eq!(error, "carrier capacity");
    assert_eq!(dropped.get(), 1);
    assert_eq!(memory.used_bytes(), 128);
    drop(claim);
    assert_eq!(memory.used_bytes(), 0);
    let values = Vec::<u64>::with_capacity(4);
    let actual = values.capacity();
    assert_eq!(
        exact_capacity(values, actual, "carrier capacity")
            .unwrap()
            .capacity(),
        actual
    );
    assert!(exact_capacity(Vec::<u64>::new(), 1, "carrier capacity").is_err());
    assert!(
        exact_capacity(Vec::<u64>::new(), 0, "carrier capacity")
            .unwrap()
            .is_empty()
    );
}

#[test]
fn payload_header_denial_precedes_iteration_and_limits_many_empty_arguments() {
    let count = 32;
    let header_bytes = argument_carrier_bytes(count).unwrap();
    let memory = ManagedMemoryBudget::new(header_bytes - 1);
    let reads = std::cell::Cell::new(0);
    let arguments = vec![OsString::new(); count]
        .into_iter()
        .inspect(|_| reads.set(reads.get() + 1));
    let error = bounded_arguments(arguments, 0, &memory).err().unwrap();
    assert!(error.contains("payload argument snapshot memory: BudgetExceeded"));
    assert_eq!(reads.get(), 0);
    assert_eq!(memory.used_bytes(), 0);
    assert!(argument_carrier_bytes(usize::MAX).is_err());
    // Deterministic allocator-rounding witness through the same guard:
    // already spare header capacity must not be silently retained.
    let mut spare = Vec::with_capacity(count + 1);
    let error = reserve_argument_headers(&mut spare, count).unwrap_err();
    assert_eq!(
        error,
        "payload argument header allocation exceeds admitted capacity"
    );
}

#[test]
fn payload_aggregate_denial_stops_before_unused_later_arguments() {
    let memory = ManagedMemoryBudget::new(4096);
    let reads = std::cell::Cell::new(0);
    let arguments = ["a", "unused large argument", "never acquired"]
        .map(OsString::from)
        .into_iter()
        .inspect(|_| reads.set(reads.get() + 1));
    let error = bounded_arguments(arguments, 1, &memory).err().unwrap();
    assert!(error.contains("finite aggregate acquisition cap"));
    assert_eq!(reads.get(), 2);
    assert_eq!(memory.used_bytes(), 0);
}

fn arguments_with_spare_capacity() -> Vec<OsString> {
    let mut first = OsString::with_capacity(1024);
    first.push("x");
    let mut second = OsString::with_capacity(2048);
    second.push("é");
    vec![first, second]
}

#[test]
fn payload_memory_exact_and_one_short_include_capacity_and_vec_arc_peak() {
    let arguments = arguments_with_spare_capacity();
    let required = argument_carrier_bytes(arguments.len()).unwrap()
        + arguments
            .iter()
            .map(|argument| argument.capacity() as u64)
            .sum::<u64>();
    assert!(required > argument_carrier_bytes(2).unwrap() + 3);
    let memory = ManagedMemoryBudget::new(required);
    let exact = bounded_arguments(arguments.into_iter(), 3, &memory).unwrap();
    assert_eq!(exact._reservation.capacity_bytes(), required);
    assert_eq!(memory.used_bytes(), required);
    assert!(memory.reserve_capacity(1).is_err());
    assert_eq!(memory.used_bytes(), required);
    drop(exact);
    assert_eq!(memory.used_bytes(), 0);
    let short = ManagedMemoryBudget::new(required - 1);
    let error = bounded_arguments(arguments_with_spare_capacity().into_iter(), 3, &short)
        .err()
        .unwrap();
    let expected = format!(
        "payload argument snapshot memory: {:?}",
        mech_core::MemoryRuntimeError::BudgetExceeded {
            operation: None,
            requested: required,
            limit: required - 1,
        }
    );
    assert_eq!(error, expected);
    assert_eq!(short.used_bytes(), 0);
    let recovered =
        bounded_arguments(arguments_with_spare_capacity().into_iter(), 3, &memory).unwrap();
    drop(recovered);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn payload_backend_keeps_ordinary_indices_count_and_charge_lifetime() {
    let memory = ManagedMemoryBudget::new(4096);
    let mut snapshot =
        bounded_arguments(["", "é", "--"].map(OsString::from).into_iter(), 4, &memory).unwrap();
    let reserved = snapshot._reservation.capacity_bytes();
    let backend = ArgumentSnapshotCliBackend::new(
        BoundedStdin { limit: 0 },
        std::mem::take(&mut snapshot.arguments),
    );
    assert_eq!(backend.argument_count().unwrap(), 3);
    assert_eq!(backend.argument(0).unwrap(), None);
    assert_eq!(backend.argument(1).unwrap(), Some(OsString::new()));
    assert_eq!(backend.argument(2).unwrap(), Some(OsString::from("é")));
    assert_eq!(backend.argument(3).unwrap(), Some(OsString::from("--")));
    assert_eq!(backend.argument(4).unwrap(), None);
    assert_eq!(memory.used_bytes(), reserved);
    drop(backend);
    assert_eq!(memory.used_bytes(), reserved);
    drop(snapshot);
    assert_eq!(memory.used_bytes(), 0);
}

#[cfg(unix)]
#[test]
fn payload_snapshot_does_not_decode_or_normalize_raw_os_arguments() {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    let memory = ManagedMemoryBudget::new(4096);
    let raw = vec![0xff, b'x'];
    let mut snapshot = bounded_arguments(
        vec![OsString::from_vec(raw.clone())].into_iter(),
        2,
        &memory,
    )
    .unwrap();
    let backend = ArgumentSnapshotCliBackend::new(
        BoundedStdin { limit: 0 },
        std::mem::take(&mut snapshot.arguments),
    );
    assert_eq!(
        backend.argument(1).unwrap().unwrap().as_os_str().as_bytes(),
        raw
    );
    drop(backend);
    drop(snapshot);
    assert_eq!(memory.used_bytes(), 0);
}

#[cfg(feature = "canonical-native-benchmark-policy-runtime")]
#[test]
fn secured_schema_stays_strict_when_benchmark_features_are_unified() {
    let mut launch = serde_json::json!({
        "grants": [], "max_ports": 0, "input_text_bytes": 0, "output_text_bytes": 0,
        "memory_bytes": 0, "work": 0, "transport_bytes": 0, "native_stack_bytes": 0,
        "library": { "output_elements": 0, "output_bytes": 0, "temporary_bytes": 0,
            "cloned_bytes": 0, "retained_nodes": 0, "compute_work": 0, "comparison_work": 0 }
    });
    assert!(Secured::parse(&serde_json::to_vec(&launch).unwrap()).is_ok());
    launch["benchmark_policy"] = "trusted".into();
    let bytes = serde_json::to_vec(&launch).unwrap();
    assert!(
        Secured::parse(&bytes)
            .err()
            .unwrap()
            .contains("unknown field")
    );
    assert_eq!(Benchmark::parse(&bytes).unwrap().1.name(), "trusted");
}
