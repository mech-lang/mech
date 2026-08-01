#[cfg(feature = "kind_define")]
use crate::{InterpreterExecution, KindDefine, MResult, Value, kind_annotation};

#[cfg(feature = "kind_define")]
pub fn kind_define(knd_def: &KindDefine, p: &InterpreterExecution<'_>) -> MResult<Value> {
    let id = knd_def.name.hash();
    let kind = kind_annotation(&knd_def.kind.kind, p)?;
    let value_kind = kind.to_value_kind(&p.state.borrow().kinds)?;
    let functions = p.functions();
    let mut kinds = &mut p.state.borrow_mut().kinds;
    kinds.insert(id, value_kind.clone());
    Ok(Value::Kind(value_kind))
}
