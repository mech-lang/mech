use cranelift_codegen::{Context, control::ControlPlane, ir, isa::TargetIsa};
use cranelift_module::{
    DataDescription, DataId, FuncId, Linkage, Module, ModuleDeclarations, ModuleError, ModuleReloc,
    ModuleResult,
};
use std::collections::BTreeMap;

pub(super) struct Definition {
    pub id: FuncId,
    pub function: ir::Function,
}

/// Own each definition before the machine backend mutates its Context. No
/// retained reference points into a cleared/reused frontend or Context.
pub(in crate::native_artifact::program) struct RecordingModule<M> {
    inner: M,
    definitions: BTreeMap<u32, Definition>,
}

impl<M: Module> RecordingModule<M> {
    pub(in crate::native_artifact::program) fn new(inner: M) -> Self {
        Self {
            inner,
            definitions: BTreeMap::new(),
        }
    }

    pub(in crate::native_artifact::program) fn llvm_ir(&self) -> Result<String, String> {
        super::lower::module(self.declarations(), self.definitions.values())
    }
    pub(in crate::native_artifact::program) fn definition_names(
        &self,
    ) -> Result<Vec<String>, String> {
        self.definitions
            .values()
            .map(|d| {
                self.declarations()
                    .get_function_decl(d.id)
                    .name
                    .clone()
                    .ok_or_else(|| "anonymous sealed LLVM definition".into())
            })
            .collect()
    }
}

fn unsupported(message: &str) -> ModuleError {
    ModuleError::Backend(std::io::Error::other(message).into())
}

impl<M: Module> Module for RecordingModule<M> {
    fn isa(&self) -> &dyn TargetIsa {
        self.inner.isa()
    }
    fn declarations(&self) -> &ModuleDeclarations {
        self.inner.declarations()
    }
    fn declare_function(
        &mut self,
        name: &str,
        linkage: Linkage,
        signature: &ir::Signature,
    ) -> ModuleResult<FuncId> {
        self.inner.declare_function(name, linkage, signature)
    }
    fn declare_anonymous_function(&mut self, _: &ir::Signature) -> ModuleResult<FuncId> {
        Err(unsupported("LLVM SSA requires named function declarations"))
    }
    fn declare_data(
        &mut self,
        name: &str,
        linkage: Linkage,
        writable: bool,
        tls: bool,
    ) -> ModuleResult<DataId> {
        self.inner.declare_data(name, linkage, writable, tls)
    }
    fn declare_anonymous_data(&mut self, _: bool, _: bool) -> ModuleResult<DataId> {
        Err(unsupported("LLVM SSA requires named data declarations"))
    }
    fn define_function_with_control_plane(
        &mut self,
        id: FuncId,
        ctx: &mut Context,
        control: &mut ControlPlane,
    ) -> ModuleResult<()> {
        if self.definitions.contains_key(&id.as_u32()) {
            return Err(ModuleError::DuplicateDefinition(format!(
                "LLVM function {}",
                id.as_u32()
            )));
        }
        // Verify the exact retained SSA, not a post-legalization replacement.
        cranelift_codegen::verify_function(&ctx.func, self.isa())
            .map_err(|e| unsupported(&format!("LLVM input SSA verification: {e}")))?;
        let function = ctx.func.clone();
        self.inner
            .define_function_with_control_plane(id, ctx, control)?;
        self.definitions
            .insert(id.as_u32(), Definition { id, function });
        Ok(())
    }
    fn define_function_bytes(
        &mut self,
        _: FuncId,
        _: u64,
        _: &[u8],
        _: &[ModuleReloc],
    ) -> ModuleResult<()> {
        Err(unsupported(
            "LLVM cannot recover verified SSA from opaque machine bytes",
        ))
    }
    fn define_data(&mut self, id: DataId, data: &DataDescription) -> ModuleResult<()> {
        self.inner.define_data(id, data)
    }
}
