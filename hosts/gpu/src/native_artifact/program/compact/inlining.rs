//! Inline selected private compact bodies with their original CLIF signature.

use cranelift_codegen::{
    inline::{Inline, InlineCommand},
    ir::{ExternalName, FuncRef, Function, Inst, Opcode, Value},
};
use cranelift_module::FuncId;
use std::{borrow::Cow, collections::BTreeMap};

pub(super) struct OnceCalled<'a>(pub(super) &'a BTreeMap<FuncId, &'a Function>);

pub(super) fn module_callee(caller: &Function, callee: FuncRef) -> Option<FuncId> {
    let ExternalName::User(name) = caller.dfg.ext_funcs[callee].name else {
        return None;
    };
    let name = &caller.params.user_named_funcs()[name];
    (name.namespace == 0).then(|| FuncId::from_u32(name.index))
}

impl Inline for OnceCalled<'_> {
    fn inline(
        &mut self,
        caller: &Function,
        _: Inst,
        opcode: Opcode,
        callee: FuncRef,
        _: &[Value],
    ) -> InlineCommand<'_> {
        let Some(id) = module_callee(caller, callee).filter(|_| opcode == Opcode::Call) else {
            return InlineCommand::KeepCall;
        };
        let external = &caller.dfg.ext_funcs[callee];
        self.0
            .get(&id)
            .filter(|target| target.signature == caller.dfg.signatures[external.signature])
            .map_or(InlineCommand::KeepCall, |target| InlineCommand::Inline {
                callee: Cow::Borrowed(*target),
                visit_callee: false,
            })
    }
}
