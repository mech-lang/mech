use crate::{LegacyValue, MechMap, MechRecord, MechSet, Ref};

pub(super) fn scalar(value: f64) -> Ref<f64> {
    Ref::new(value)
}

pub(super) fn scalar_value(value: &Ref<f64>) -> LegacyValue {
    LegacyValue::F64(value.clone())
}

pub(super) fn as_scalar(value: &LegacyValue) -> Ref<f64> {
    match value {
        LegacyValue::F64(value) => value.clone(),
        _ => panic!("expected f64 value"),
    }
}

pub(super) fn scalar_payload(value: &LegacyValue) -> f64 {
    *as_scalar(value).borrow()
}

pub(super) fn set_contains_scalar(set: &Ref<MechSet>, value: f64) -> bool {
    let probe = scalar_value(&scalar(value));
    set.borrow().set.contains(&probe)
}

pub(super) fn map_get_scalar(map: &Ref<MechMap>, key: f64) -> Option<Ref<f64>> {
    let probe = scalar_value(&scalar(key));
    map.borrow().map.get(&probe).map(as_scalar)
}

pub(super) fn record_value(fields: Vec<(&str, LegacyValue)>) -> (Ref<MechRecord>, LegacyValue) {
    let record = Ref::new(MechRecord::new(fields));
    let value = LegacyValue::Record(record.clone());
    (record, value)
}
