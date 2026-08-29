#[cfg(feature = "kind_define")]
use crate::{InterpreterExecution, KindDefine, MResult, ValueCell, kind_value};

#[cfg(feature = "kind_define")]
pub fn kind_define(knd_def: &KindDefine, p: &InterpreterExecution<'_>) -> MResult<ValueCell> {
    let id = knd_def.name.hash();
    let schema = crate::structures::schema_body_from_kind(&knd_def.kind.kind, p)?;
    let kinds = &mut p.state.borrow_mut().kinds;
    kinds.insert(id, schema);
    kind_value(&knd_def.kind.kind, p)
}
