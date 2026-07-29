use mech_core::Value;
use mech_runtime::RuntimeValueSnapshot;

fn removed_accessor(snapshot: &RuntimeValueSnapshot) {
  let _: &Value = snapshot.as_value();
}

fn removed_deref(snapshot: &RuntimeValueSnapshot) {
  let _: &Value = &*snapshot;
}

fn private_field(snapshot: &RuntimeValueSnapshot) {
  let _: &Value = &snapshot.value;
}

fn mutable_cell_escape(snapshot: &RuntimeValueSnapshot) {
  if let Value::F64(value) = &**snapshot {
    *value.borrow_mut() = 99.0;
  }
}

fn main() {}
