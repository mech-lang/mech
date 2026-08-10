use mech_core::LegacyValue;
use mech_runtime::RuntimeValueSnapshot;

fn removed_accessor(snapshot: &RuntimeValueSnapshot) {
  let _: &LegacyValue = snapshot.as_value();
}

fn removed_deref(snapshot: &RuntimeValueSnapshot) {
  let _: &LegacyValue = &*snapshot;
}

fn private_field(snapshot: &RuntimeValueSnapshot) {
  let _: &LegacyValue = &snapshot.value;
}

fn mutable_cell_escape(snapshot: &RuntimeValueSnapshot) {
  if let LegacyValue::F64(value) = &**snapshot {
    *value.borrow_mut() = 99.0;
  }
}

fn main() {}
