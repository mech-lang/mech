use super::super::{MechRuntime, RuntimeConfig, Value, hash_str};
use mech_core::Ref;

#[test]
fn runtime_has_interpreter_finds_root_interpreter() {
    let runtime = MechRuntime::new(RuntimeConfig::default()).unwrap();
    assert!(runtime.has_interpreter(0));
}

#[test]
fn runtime_bind_ans_for_interpreter_binds_ans() {
    let mut runtime = MechRuntime::new(RuntimeConfig::default()).unwrap();
    let value = Value::U64(Ref::new(42));
    runtime.bind_ans_for_interpreter(0, &value).unwrap();
    let ans_id = hash_str("ans");
    let bound = runtime
        .program()
        .interpreter()
        .symbols()
        .borrow()
        .get(ans_id)
        .map(|value| value.borrow().clone());
    assert_eq!(bound, Some(value));
}
