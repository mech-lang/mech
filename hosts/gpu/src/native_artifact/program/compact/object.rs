//! Select the compact Object's native definitions before writing them.

use cranelift_codegen::{Context, control::ControlPlane, ir::Signature, isa::TargetIsa};
use cranelift_module::{
    DataDescription, DataId, FuncId, FuncOrDataId, Linkage, Module, ModuleDeclarations,
    ModuleError, ModuleReloc, ModuleRelocTarget, ModuleResult,
};
use cranelift_object::{ObjectModule, ObjectProduct};
use std::collections::{BTreeMap, BTreeSet};

struct Text {
    alignment: u64,
    bytes: Box<[u8]>,
    relocs: Vec<ModuleReloc>,
}

enum Body {
    Text(Text),
    Data(DataDescription),
}

/// Phases may compile alternative bodies while selecting a call target. Keep
/// their bytes and relocations so unused alternatives never need Object symbols.
pub(in crate::native_artifact::program) struct PreparedObject {
    object: ObjectModule,
    declarations: ModuleDeclarations,
    declaration_order: Vec<FuncOrDataId>,
    definition_order: Vec<FuncOrDataId>,
    bodies: BTreeMap<FuncOrDataId, Body>,
}

impl PreparedObject {
    pub(in crate::native_artifact::program) fn new(object: ObjectModule) -> Self {
        Self {
            object,
            declarations: ModuleDeclarations::default(),
            declaration_order: Vec::new(),
            definition_order: Vec::new(),
            bodies: BTreeMap::new(),
        }
    }

    fn linkage(&self, id: FuncOrDataId) -> Linkage {
        match id {
            FuncOrDataId::Func(id) => self.declarations.get_function_decl(id).linkage,
            FuncOrDataId::Data(id) => self.declarations.get_data_decl(id).linkage,
        }
    }

    fn record(&mut self, id: FuncOrDataId, body: Body) -> ModuleResult<()> {
        if !self.linkage(id).is_definable() {
            return Err(ModuleError::InvalidImportDefinition(format!("{id:?}")));
        }
        if self.bodies.contains_key(&id) {
            return Err(ModuleError::DuplicateDefinition(format!("{id:?}")));
        }
        self.bodies.insert(id, body);
        self.definition_order.push(id);
        Ok(())
    }

    fn reachable(&self) -> ModuleResult<BTreeSet<FuncOrDataId>> {
        let mut pending = self
            .declaration_order
            .iter()
            .copied()
            // Keep nonlocal functions and all data, including function pointers
            // in data that can be passed outside this module.
            .filter(|&id| matches!(id, FuncOrDataId::Data(_)) || self.linkage(id) != Linkage::Local)
            .collect::<Vec<_>>();
        let mut reachable = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if !reachable.insert(id) {
                continue;
            }
            match self.bodies.get(&id) {
                Some(Body::Text(text)) => {
                    pending.extend(text.relocs.iter().filter_map(|reloc| symbol(&reloc.name)));
                }
                Some(Body::Data(data)) => {
                    pending.extend(data.function_decls.values().filter_map(symbol));
                    pending.extend(data.data_decls.values().filter_map(symbol));
                }
                None if self.linkage(id).requires_definition() => {
                    return Err(ModuleError::Undeclared(format!("{id:?}")));
                }
                None => {}
            }
        }
        Ok(reachable)
    }

    pub(in crate::native_artifact::program) fn finish(mut self) -> ModuleResult<ObjectProduct> {
        let reachable = self.reachable()?;
        let mut symbols = BTreeMap::new();
        // Preserve declaration and definition order among the retained symbols.
        for id in self.declaration_order {
            if !reachable.contains(&id) {
                continue;
            }
            let emitted = match id {
                FuncOrDataId::Func(id) => {
                    let decl = self.declarations.get_function_decl(id);
                    let emitted = match &decl.name {
                        Some(name) => {
                            self.object
                                .declare_function(name, decl.linkage, &decl.signature)?
                        }
                        None => self.object.declare_anonymous_function(&decl.signature)?,
                    };
                    FuncOrDataId::Func(emitted)
                }
                FuncOrDataId::Data(id) => {
                    let decl = self.declarations.get_data_decl(id);
                    let emitted = match &decl.name {
                        Some(name) => {
                            self.object
                                .declare_data(name, decl.linkage, decl.writable, decl.tls)?
                        }
                        None => self
                            .object
                            .declare_anonymous_data(decl.writable, decl.tls)?,
                    };
                    FuncOrDataId::Data(emitted)
                }
            };
            symbols.insert(id, emitted);
        }
        for id in self.definition_order {
            let body = self.bodies.remove(&id).unwrap();
            let Some(&emitted) = symbols.get(&id) else {
                continue;
            };
            match (emitted, body) {
                (FuncOrDataId::Func(id), Body::Text(mut text)) => {
                    for reloc in &mut text.relocs {
                        relocate(&symbols, &mut reloc.name)?;
                    }
                    self.object.define_function_bytes(
                        id,
                        text.alignment,
                        &text.bytes,
                        &text.relocs,
                    )?;
                }
                (FuncOrDataId::Data(id), Body::Data(mut data)) => {
                    for name in data
                        .function_decls
                        .values_mut()
                        .chain(data.data_decls.values_mut())
                    {
                        relocate(&symbols, name)?;
                    }
                    self.object.define_data(id, &data)?;
                }
                _ => unreachable!("Object symbol changed kind"),
            }
        }
        Ok(self.object.finish())
    }
}

fn symbol(name: &ModuleRelocTarget) -> Option<FuncOrDataId> {
    match *name {
        ModuleRelocTarget::User { namespace: 0, .. } => {
            Some(FuncOrDataId::Func(FuncId::from_name(name)))
        }
        ModuleRelocTarget::User { .. } => Some(FuncOrDataId::Data(DataId::from_name(name))),
        ModuleRelocTarget::FunctionOffset(id, _) => Some(FuncOrDataId::Func(id)),
        ModuleRelocTarget::LibCall(_) | ModuleRelocTarget::KnownSymbol(_) => None,
    }
}

fn relocate(
    symbols: &BTreeMap<FuncOrDataId, FuncOrDataId>,
    name: &mut ModuleRelocTarget,
) -> ModuleResult<()> {
    let Some(id) = symbol(name) else {
        return Ok(());
    };
    *name = match (symbols.get(&id), &*name) {
        (Some(FuncOrDataId::Func(id)), ModuleRelocTarget::FunctionOffset(_, offset)) => {
            ModuleRelocTarget::FunctionOffset(*id, *offset)
        }
        (Some(FuncOrDataId::Func(id)), _) => (*id).into(),
        (Some(FuncOrDataId::Data(id)), _) => (*id).into(),
        (None, _) => return Err(ModuleError::Undeclared(format!("{id:?}"))),
    };
    Ok(())
}

impl Module for PreparedObject {
    fn isa(&self) -> &dyn TargetIsa {
        self.object.isa()
    }

    fn declarations(&self) -> &ModuleDeclarations {
        &self.declarations
    }

    fn declare_function(
        &mut self,
        name: &str,
        linkage: Linkage,
        signature: &Signature,
    ) -> ModuleResult<FuncId> {
        let known = self.declarations.get_name(name).is_some();
        let (id, _) = self
            .declarations
            .declare_function(name, linkage, signature)?;
        if !known {
            self.declaration_order.push(FuncOrDataId::Func(id));
        }
        Ok(id)
    }

    fn declare_anonymous_function(&mut self, signature: &Signature) -> ModuleResult<FuncId> {
        let id = self.declarations.declare_anonymous_function(signature)?;
        self.declaration_order.push(FuncOrDataId::Func(id));
        Ok(id)
    }

    fn declare_data(
        &mut self,
        name: &str,
        linkage: Linkage,
        writable: bool,
        tls: bool,
    ) -> ModuleResult<DataId> {
        let known = self.declarations.get_name(name).is_some();
        let (id, _) = self
            .declarations
            .declare_data(name, linkage, writable, tls)?;
        if !known {
            self.declaration_order.push(FuncOrDataId::Data(id));
        }
        Ok(id)
    }

    fn declare_anonymous_data(&mut self, writable: bool, tls: bool) -> ModuleResult<DataId> {
        let id = self.declarations.declare_anonymous_data(writable, tls)?;
        self.declaration_order.push(FuncOrDataId::Data(id));
        Ok(id)
    }

    fn define_function_with_control_plane(
        &mut self,
        id: FuncId,
        context: &mut Context,
        control: &mut ControlPlane,
    ) -> ModuleResult<()> {
        context.compile(self.isa(), control)?;
        let buffer = &context.compiled_code().unwrap().buffer;
        let relocs = buffer
            .relocs()
            .iter()
            .map(|reloc| ModuleReloc::from_mach_reloc(reloc, &context.func, id))
            .collect();
        self.record(
            FuncOrDataId::Func(id),
            Body::Text(Text {
                alignment: u64::from(buffer.alignment),
                bytes: buffer.data().into(),
                relocs,
            }),
        )
    }

    fn define_function_bytes(
        &mut self,
        id: FuncId,
        alignment: u64,
        bytes: &[u8],
        relocs: &[ModuleReloc],
    ) -> ModuleResult<()> {
        self.record(
            FuncOrDataId::Func(id),
            Body::Text(Text {
                alignment,
                bytes: bytes.into(),
                relocs: relocs.to_vec(),
            }),
        )
    }

    fn define_data(&mut self, id: DataId, data: &DataDescription) -> ModuleResult<()> {
        self.record(FuncOrDataId::Data(id), Body::Data(data.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::{ir::InstBuilder, settings};
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
    use cranelift_module::default_libcall_names;
    use cranelift_object::ObjectBuilder;

    fn module() -> PreparedObject {
        let isa = cranelift_native::builder()
            .unwrap()
            .finish(settings::Flags::new(settings::builder()))
            .unwrap();
        let builder = ObjectBuilder::new(isa, "compact_test", default_libcall_names()).unwrap();
        PreparedObject::new(ObjectModule::new(builder))
    }

    fn function(module: &mut PreparedObject, name: &str, linkage: Linkage) -> FuncId {
        module
            .declare_function(name, linkage, &module.make_signature())
            .unwrap()
    }

    fn define(module: &mut PreparedObject, function: FuncId, calls: &[FuncId]) {
        let mut context = module.make_context();
        let mut builder_context = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut context.func, &mut builder_context);
        let block = builder.create_block();
        builder.switch_to_block(block);
        for &callee in calls {
            let callee = module.declare_func_in_func(callee, builder.func);
            builder.ins().call(callee, &[]);
        }
        builder.ins().return_(&[]);
        builder.seal_all_blocks();
        builder.finalize();
        module.define_function(function, &mut context).unwrap();
    }

    #[test]
    fn retains_selected_dependencies_and_function_pointers() {
        let mut module = module();
        let discarded = function(&mut module, "unselected", Linkage::Local);
        let discarded_child = function(&mut module, "unselected_child", Linkage::Local);
        let selected = function(&mut module, "selected", Linkage::Local);
        let fallback = function(&mut module, "checked_fallback", Linkage::Local);
        let indirect = function(&mut module, "referenced_by_data", Linkage::Local);
        let entry = function(&mut module, "entry", Linkage::Export);
        define(&mut module, discarded, &[discarded_child]);
        for leaf in [discarded_child, fallback, indirect] {
            define(&mut module, leaf, &[]);
        }
        define(&mut module, selected, &[fallback]);
        define(&mut module, entry, &[selected]);
        let table = module
            .declare_data("callbacks", Linkage::Local, false, false)
            .unwrap();
        let mut data = DataDescription::new();
        let pointer = module.declare_func_in_data(indirect, &mut data);
        data.define_zeroinit(module.isa().pointer_bytes().into());
        data.write_function_addr(0, pointer);
        module.define_data(table, &data).unwrap();

        let product = module.finish().unwrap();
        for kept in [
            "entry",
            "selected",
            "checked_fallback",
            "referenced_by_data",
        ] {
            assert!(
                product.object.symbol_id(kept.as_bytes()).is_some(),
                "{kept}"
            );
        }
        for discarded in ["unselected", "unselected_child"] {
            assert!(product.object.symbol_id(discarded.as_bytes()).is_none());
        }
        assert!(!product.emit().unwrap().is_empty());
    }

    #[test]
    fn requires_a_definition_for_a_retained_local_target() {
        let mut module = module();
        let missing = function(&mut module, "missing", Linkage::Local);
        let entry = function(&mut module, "entry", Linkage::Export);
        define(&mut module, entry, &[missing]);
        assert!(matches!(module.finish(), Err(ModuleError::Undeclared(_))));
    }
}
