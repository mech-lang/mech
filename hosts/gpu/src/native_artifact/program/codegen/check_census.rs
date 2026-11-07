//! Diagnostic JIT-only native check counters. No callback executes per check.
//! Enabled only in a custom-cfg diagnostic build and only while compiling a JIT
//! owner. Object emission is never enabled. Counter allocations live until exit,
//! so subsequent compilations cannot invalidate existing generated pointers.
use cranelift_codegen::ir::{AtomicRmwOp, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use std::{cell::Cell, collections::BTreeMap, sync::{Mutex, OnceLock, atomic::{AtomicU64, Ordering}}};
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
struct Key { compilation: u64, kind: &'static str, owner: usize, site: usize, version: bool }
thread_local! { static CURRENT: Cell<Option<u64>> = const { Cell::new(None) }; }
static NEXT: AtomicU64 = AtomicU64::new(0);
static ROWS: OnceLock<Mutex<BTreeMap<Key, Box<[AtomicU64; 2]>>>> = OnceLock::new();
pub(in crate::native_artifact) struct Guard;
impl Drop for Guard { fn drop(&mut self) { CURRENT.with(|c| c.set(None)); } }
pub(in crate::native_artifact) fn begin() -> Guard {
    CURRENT.with(|c| { assert!(c.get().is_none()); c.set(Some(NEXT.fetch_add(1, Ordering::Relaxed))); });
    Guard
}
pub(in crate::native_artifact) fn record(b: &mut FunctionBuilder<'_>, kind: &'static str, owner: usize, site: usize, version: bool, condition: Option<Value>) {
    let Some(compilation) = CURRENT.with(Cell::get) else { return; };
    let key = Key { compilation, kind, owner, site, version };
    let address = {
        let mut rows = ROWS.get_or_init(|| Mutex::new(BTreeMap::new())).lock().unwrap();
        assert!(rows.len() < 100_000, "diagnostic census row bound exceeded");
        let words = rows.entry(key).or_insert_with(|| Box::new([AtomicU64::new(0), AtomicU64::new(0)]));
        words.as_ptr() as usize
    };
    let one = b.ins().iconst(types::I64, 1);
    let success = if let Some(condition) = condition {
        let truth = b.ins().icmp_imm(IntCC::NotEqual, condition, 0);
        b.ins().uextend(types::I64, truth)
    } else { one };
    for (at, value) in [(address, one), (address + 8, success)] {
        let at = b.ins().iconst(types::I64, at as i64);
        b.ins().atomic_rmw(types::I64, MemFlags::trusted(), AtomicRmwOp::Add, at, value);
    }
}
pub(in crate::native_artifact) fn snapshot() -> serde_json::Value {
    let rows = ROWS.get_or_init(|| Mutex::new(BTreeMap::new())).lock().unwrap();
    serde_json::json!({"schema":"mech.native-executed-check-census.v1", "diagnostic_only":true, "JIT_only":true, "timing_claim":false,
        "coverage":"numeric entry/funding/capacity/read-write/private-owner gates, consumed read bounds proofs, and child-window selection; overlapping component and combined gates are reported separately",
        "rows": rows.iter().map(|(key, words)| { let attempts=words[0].load(Ordering::SeqCst); let successes=words[1].load(Ordering::SeqCst); serde_json::json!({"key":key,"attempts":attempts,"successes":successes,"failures":attempts.checked_sub(successes).expect("census underflow")}) }).collect::<Vec<_>>()})
}
