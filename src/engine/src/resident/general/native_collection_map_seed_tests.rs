use super::*;
use crate::resident::general::native_collection_calls::prepare_native_program_library_call;
use crate::{CanonicalSourceFrontend, ProgramArtifact};
use mech_core::{
    ManagedMemoryBudget, ValueDataDraft as Data, ValueDraft,
    snapshot::{MapEntryDraft, SnapshotValidationContext},
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
fn catalog() -> mech_core::FunctionCatalog {
    let mut b = mech_core::FunctionCatalogBuilder::new();
    crate::install_intrinsic_runtime(&mut b).unwrap();
    crate::install_intrinsic_compiler_runtime(&mut b).unwrap();
    crate::install_intrinsic_resident(&mut b).unwrap();
    crate::install_intrinsic_source(&mut b).unwrap();
    b.build().unwrap()
}
fn artifacts() -> [ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(
            DocumentId(0x53454544),
            Revision(0),
            "(at<u64>,end<u64>,dictionary<{string:u64}>)\n",
        )
        .unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let a = CanonicalSourceFrontend
        .compile_document(&DocumentSyntax::cast(parsed.syntax()).unwrap())
        .unwrap()
        .compile_artifact()
        .unwrap();
    let b = crate::decode_program_artifact_bytecode_v1(
        &crate::encode_program_artifact_bytecode_v1(&a).unwrap(),
    )
    .unwrap();
    [a, b]
}
fn prepared(a: &ProgramArtifact) -> NativePreparedLibraryCall {
    let c = catalog();
    let plan = crate::native_artifact::lower_native_program_artifact(a, &c).unwrap();
    let t = plan
        .blocks()
        .iter()
        .find_map(|b| match b.action() {
            crate::native_artifact::NativeIterationAction::LibraryCall { template, .. }
                if template.primitive() == Primitive::CompositePack(3) =>
            {
                Some(template)
            }
            _ => None,
        })
        .unwrap();
    prepare_native_program_library_call(a, t, &c).unwrap()
}
fn limits() -> MemoryBudgetLimits {
    let mut l = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    l.max_compute_work = Some(u64::MAX);
    l.max_comparison_work = Some(u64::MAX);
    l.max_temporary_bytes = Some(u64::MAX);
    l.max_retained_nodes = Some(u64::MAX);
    l.max_output_elements = Some(u64::MAX);
    l.max_output_bytes = Some(u64::MAX);
    l.max_cloned_bytes = Some(u64::MAX);
    l
}
fn inputs(
    a: &ProgramArtifact,
    c: &NativePreparedLibraryCall,
    m: &ManagedMemoryBudget,
    keys: &[String],
) -> Vec<Option<Value>> {
    let mech_core::ResolvedOperationContract::Declared(contract) = c.template.contract() else {
        unreachable!()
    };
    contract
        .inputs
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let data = if i == 2 {
                Data::Map(
                    keys.iter()
                        .enumerate()
                        .map(|(i, k)| MapEntryDraft {
                            items: vec![Data::String(k.clone()), Data::U64(91 + i as u64)].into(),
                        })
                        .collect(),
                )
            } else {
                Data::U64(if i == 0 { 0 } else { keys.len() as u64 })
            };
            let v = ValueDraft {
                schema: p.schema,
                shape_values: Box::new([]),
                data,
            }
            .finalize(&SnapshotValidationContext::new(a.schemas()))
            .unwrap();
            let mut r = m
                .reserve_capacity(v.memory_budget_admission_bytes(m).unwrap())
                .unwrap();
            Some(v.into_memory_budget(&mut r).unwrap())
        })
        .collect()
}
fn ordinary(
    c: &NativePreparedLibraryCall,
    x: &[Option<Value>],
    l: MemoryBudgetLimits,
    w: &NativeLibraryWorkBudget,
) -> Result<Value, InvokeError> {
    let mut out = None;
    c.invoke_current(x, &mut out, l, w)?;
    out.ok_or(InvokeError::InvalidInput)
}
fn frame(
    c: &NativePreparedLibraryCall,
    x: &[Option<Value>],
    w: &NativeLibraryWorkBudget,
) -> NativeMapSeedFrame {
    match c
        .map_seed_or_canonical(x, limits(), w, ResourceDemand::default())
        .unwrap()
    {
        NativeMapSeedChoice::Frame(f) => f,
        NativeMapSeedChoice::Canonical(_) => panic!("eligible frame not selected"),
    }
}
#[test]
fn native_map_seed_keeps_real_generic_children_exact_logical_footprint_and_original_factory() {
    for a in artifacts() {
        let c = prepared(&a);
        let foreign_factory = prepared(&a);
        let long: Vec<_> = (0..73)
            .map(|i| format!("{i}:{}", "東京\0é".repeat(30)))
            .collect();
        for keys in [vec![], vec!["".into(), "a\0b".into(), "é🧑‍🚀".into()], long] {
            let m = ManagedMemoryBudget::new(64 << 20);
            let w = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &m).unwrap();
            let x = inputs(&a, &c, &m, &keys);
            let canonical = ordinary(&c, &x, limits(), &w).unwrap();
            let expected = canonical.canonical_data_draft().unwrap();
            let logical = canonical.retained_footprint(a.schemas()).unwrap();
            drop(canonical);
            let before = (m.used_bytes(), m.retained_nodes());
            let f = frame(&c, &x, &w);
            assert_eq!(
                (f.logical_retained_bytes(), f.logical_nodes()),
                (logical.retained_bytes, logical.node_count)
            );
            assert_eq!(f.map_ordinal(), 2);
            assert!(
                f.children()[2]
                    .as_ref()
                    .unwrap()
                    .shares_frozen_storage(x[2].as_ref().unwrap())
            );
            let foreign_m = ManagedMemoryBudget::new(64 << 20);
            let foreign_w = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &foreign_m).unwrap();
            assert!(f.check_account(&foreign_w).is_err());
            drop(foreign_w);
            assert_eq!((foreign_m.used_bytes(), foreign_m.retained_nodes()), (0, 0));
            let actual = c.materialize_map_seed(f, &w).unwrap();
            assert_eq!(actual.canonical_data_draft().unwrap(), expected);
            drop(actual);
            assert_eq!((m.used_bytes(), m.retained_nodes()), before);
            let f = frame(&c, &x, &w);
            assert!(foreign_factory.materialize_map_seed(f, &w).is_err());
            assert_eq!((m.used_bytes(), m.retained_nodes()), before);
            let f = frame(&c, &x, &w);
            drop(x);
            assert!(matches!(
                f.children()[2].as_ref().unwrap().data(),
                ValueData::Map(_)
            ));
            let actual = c.materialize_map_seed(f, &w).unwrap();
            assert_eq!(actual.canonical_data_draft().unwrap(), expected);
            drop((actual, w));
            assert_eq!((m.used_bytes(), m.retained_nodes()), (0, 0));
        }
    }
}
#[test]
fn native_map_seed_enclosing_scope_keeps_immediate_canonical_path_and_logical_denials() {
    for a in artifacts() {
        let c = prepared(&a);
        let m = ManagedMemoryBudget::new(64 << 20);
        let w = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &m).unwrap();
        let x = inputs(&a, &c, &m, &["a".into(), "é".into()]);
        let before = (m.used_bytes(), m.retained_nodes());
        let expected = ordinary(&c, &x, limits(), &w)
            .unwrap()
            .canonical_data_draft()
            .unwrap();
        w.with_call_limits(u64::MAX, u64::MAX, || {
            match c
                .map_seed_or_canonical(&x, limits(), &w, ResourceDemand::default())
                .unwrap()
            {
                NativeMapSeedChoice::Canonical(v) => {
                    assert_eq!(v.canonical_data_draft().unwrap(), expected)
                }
                NativeMapSeedChoice::Frame(_) => panic!("enclosing cap suspended"),
            }
        })
        .unwrap();
        assert_eq!((m.used_bytes(), m.retained_nodes()), before);
        for dimension in ["elements", "output", "clone"] {
            let mut l = limits();
            match dimension {
                "elements" => l.max_output_elements = Some(2),
                "output" => l.max_output_bytes = Some(0),
                "clone" => l.max_cloned_bytes = Some(0),
                _ => unreachable!(),
            };
            assert!(ordinary(&c, &x, l, &w).is_err());
            assert!(
                c.map_seed_or_canonical(&x, l, &w, ResourceDemand::default())
                    .is_err()
            );
            assert_eq!((m.used_bytes(), m.retained_nodes()), before);
        }
        assert!(!w.has_active_call_limit().unwrap());
        drop((x, w));
        assert_eq!((m.used_bytes(), m.retained_nodes()), (0, 0));
    }
}
#[test]
fn native_map_seed_late_fallback_preserves_exact_relative_prefix_and_global_intervening_work() {
    for a in artifacts() {
        let c = prepared(&a);
        let m = ManagedMemoryBudget::new(64 << 20);
        let x = inputs(&a, &c, &m, &["a".into(), "long\0東京".repeat(5)]);
        let before = (m.used_bytes(), m.retained_nodes());
        let trial = |cap: u64, global: u64, intervening: u64| {
            let w = NativeLibraryWorkBudget::new(global, u64::MAX, &m).unwrap();
            let mut l = limits();
            l.max_compute_work = Some(cap);
            let r = (|| {
                let selected = c.map_seed_or_canonical(&x, l, &w, ResourceDemand::default())?;
                let NativeMapSeedChoice::Frame(f) = selected else {
                    return Err(InvokeError::InvalidInput);
                };
                w.charge(intervening, 0)?;
                c.materialize_map_seed(f, &w).map(drop)
            })();
            let used = w.used().0;
            assert!(!w.has_active_call_limit().unwrap());
            drop(w);
            assert_eq!((m.used_bytes(), m.retained_nodes()), before);
            (r.is_ok(), used)
        };
        let mut lo = 0;
        let mut hi = 1_000_000_000;
        assert!(trial(hi, u64::MAX, 37).0);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if trial(mid, u64::MAX, 37).0 {
                hi = mid
            } else {
                lo = mid + 1
            }
        }
        assert!(trial(lo, u64::MAX, 37).0);
        assert!(!trial(lo - 1, u64::MAX, 37).0);
        assert!(trial(lo, u64::MAX, 0).0);
        let used = trial(lo, u64::MAX, 37).1;
        assert!(trial(lo, used, 37).0);
        assert!(!trial(lo, used - 1, 37).0);
        println!("native-map-seed relative exact={lo} global exact={used} intervening=37");
        drop(x);
        assert_eq!((m.used_bytes(), m.retained_nodes()), (0, 0));
    }
}
