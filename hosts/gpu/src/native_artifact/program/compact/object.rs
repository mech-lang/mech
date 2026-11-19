//! Select the compact Object's native definitions before writing them.

use cranelift_codegen::{Context, control::ControlPlane, ir, ir::Signature, isa::TargetIsa};
use cranelift_module::{
    DataDescription, DataId, FuncId, FuncOrDataId, Linkage, Module, ModuleDeclarations,
    ModuleError, ModuleReloc, ModuleRelocTarget, ModuleResult,
};
use cranelift_object::{ObjectModule, ObjectProduct};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use super::super::llvm_backend::{CompactBody, NativeLlvmToolchain, compact_definition};
use super::phase_layout;

struct Text {
    alignment: u64,
    bytes: Box<[u8]>,
    relocs: Vec<ModuleReloc>,
}

// The experiment retains only small legalized private CLIF bodies.
const MAX_INLINE_BYTES: usize = 2048;
const MAX_INLINE_VALUES: usize = 12_000;
const MAX_INLINE_BODIES: usize = 24;

enum Body {
    Text(Text),
    Native(CompactBody),
    Data(DataDescription),
}

struct LlvmDump {
    directory: PathBuf,
    symbols: BTreeSet<String>,
}

#[derive(Clone, Copy, Default)]
pub(in crate::native_artifact::program) enum DirectRuntimeCalls {
    #[default]
    Disabled,
    Allocation,
    AllocationAndByteAppends,
    Free,
    All,
}

impl DirectRuntimeCalls {
    pub(super) fn declare_in(
        &self,
        declarations: &ModuleDeclarations,
        id: FuncId,
        f: &mut ir::Function,
    ) -> ir::FuncRef {
        let decl = declarations.get_function_decl(id);
        let signature = f.import_signature(decl.signature.clone());
        let name = f.declare_imported_user_function(ir::UserExternalName {
            namespace: 0,
            index: id.as_u32(),
        });
        // These providers are linked into the compact AArch64 Object's host.
        // Let the linker resolve a branch relocation to each provider.
        let direct = decl.linkage == Linkage::Import && self.includes(decl.name.as_deref());
        f.import_function(ir::ExtFuncData {
            name: ir::ExternalName::user(name),
            signature,
            colocated: decl.linkage.is_final() || direct,
            patchable: false,
        })
    }
    fn includes(&self, name: Option<&str>) -> bool {
        match name {
            Some(
                "mech_compact_allocate"
                | "mech_compact_allocate_backing"
                | "mech_compact_writable"
                | "mech_compact_writable_scalar"
                | "mech_compact_writable_scalar_backing",
            ) => {
                matches!(
                    self,
                    Self::Allocation | Self::AllocationAndByteAppends | Self::All
                )
            }
            Some("mech_compact_finish_byte_appends") => {
                matches!(self, Self::AllocationAndByteAppends)
            }
            Some("mech_compact_free") => matches!(self, Self::Free | Self::All),
            Some(
                "mech_compact_release_owner"
                | "mech_compact_release_nullable"
                | "mech_compact_release_two"
                | "mech_compact_release_three"
                | "mech_compact_release_four",
            ) => !matches!(self, Self::Disabled),
            _ => false,
        }
    }
}

/// Phases may compile alternative bodies while selecting a call target. Keep
/// their bytes and relocations so unused alternatives never need Object symbols.
pub(in crate::native_artifact::program) struct PreparedObject {
    object: ObjectModule,
    declarations: ModuleDeclarations,
    declaration_order: Vec<FuncOrDataId>,
    definition_order: Vec<FuncOrDataId>,
    bodies: BTreeMap<FuncOrDataId, Body>,
    direct_runtime_calls: DirectRuntimeCalls,
    llvm_dump: Option<LlvmDump>,
    inline_bodies: Option<BTreeMap<FuncId, ir::Function>>,
}

impl PreparedObject {
    pub(in crate::native_artifact::program) fn new(object: ObjectModule) -> Self {
        let llvm_dump = std::env::var_os("MECH_COMPACT_DUMP_LLVM_DIRECTORY")
            .zip(std::env::var("MECH_COMPACT_DUMP_LLVM_SYMBOLS").ok())
            .map(|(directory, symbols)| LlvmDump {
                directory: directory.into(),
                symbols: symbols.split(',').map(str::to_owned).collect(),
            });
        Self {
            object,
            declarations: ModuleDeclarations::default(),
            declaration_order: Vec::new(),
            definition_order: Vec::new(),
            bodies: BTreeMap::new(),
            direct_runtime_calls: DirectRuntimeCalls::Disabled,
            llvm_dump,
            inline_bodies: std::env::var_os("MECH_COMPACT_INLINE_SINGLE_CALLS")
                .map(|_| BTreeMap::new()),
        }
    }

    fn inline_private_helpers(
        &self,
        id: FuncId,
        context: &mut Context,
        control: &mut ControlPlane,
    ) -> ModuleResult<()> {
        use cranelift_codegen::binemit::Reloc;

        let Some(bodies) = &self.inline_bodies else {
            return Ok(());
        };
        let outside_loops = (std::env::var_os("MECH_COMPACT_INLINE_SINGLE_CALLS").as_deref()
            == Some(std::ffi::OsStr::new("outside-loops")))
        .then(|| {
            context.compute_cfg();
            context.compute_domtree();
            context.compute_loop_analysis();
            let function = &context.func;
            function
                .layout
                .blocks()
                .filter(|&block| context.loop_analysis.innermost_loop(block).is_none())
                .flat_map(|block| function.layout.block_insts(block))
                .filter_map(|inst| match function.dfg.insts[inst] {
                    ir::InstructionData::Call { func_ref, .. } => {
                        super::inlining::module_callee(function, func_ref)
                    }
                    _ => None,
                })
                .collect::<BTreeSet<_>>()
        });
        let Some(compiled) = context.compiled_code() else {
            return Ok(());
        };
        let mut referenced = BTreeMap::<FuncId, (usize, bool)>::new();
        for reloc in compiled.buffer.relocs() {
            let reloc = ModuleReloc::from_mach_reloc(reloc, &context.func, id);
            if let Some(FuncOrDataId::Func(target)) = symbol(&reloc.name) {
                let entry = referenced.entry(target).or_default();
                entry.0 += 1;
                entry.1 |= reloc.kind == Reloc::Arm64Call;
            }
        }
        let selected = self.once_called_helpers(bodies, &referenced, outside_loops.as_ref());
        if selected.is_empty() {
            return Ok(());
        }
        let mut candidate = Context::for_function(context.func.clone());
        if !candidate
            .inline(super::inlining::OnceCalled(&selected))
            .map_err(|error| ModuleError::Backend(error.into()))?
        {
            return Ok(());
        }
        candidate
            .verify(self.isa())
            .map_err(|error| ModuleError::Backend(error.into()))?;
        let bytes = candidate.compile(self.isa(), control)?.buffer.data().len();
        let retained = candidate
            .compiled_code()
            .expect("compiled candidate has code")
            .buffer
            .relocs()
            .iter()
            .map(|r| ModuleReloc::from_mach_reloc(r, &candidate.func, id))
            .filter_map(|r| symbol(&r.name))
            .collect::<BTreeSet<_>>();
        let retired_bytes = selected
            .keys()
            .map(|&target| FuncOrDataId::Func(target))
            .filter(|target| !retained.contains(target))
            .filter_map(|target| match self.bodies.get(&target) {
                Some(Body::Text(text)) => Some(text.bytes.len()),
                _ => None,
            })
            .sum::<usize>();
        let baseline = compiled.buffer.data().len() + retired_bytes;
        eprintln!("compact native inline projection: {baseline} -> {bytes} bytes");
        if bytes < baseline {
            *context = candidate;
        }
        Ok(())
    }

    fn once_called_helpers<'a>(
        &self,
        bodies: &'a BTreeMap<FuncId, ir::Function>,
        referenced: &BTreeMap<FuncId, (usize, bool)>,
        outside_loops: Option<&BTreeSet<FuncId>>,
    ) -> BTreeMap<FuncId, &'a ir::Function> {
        let mut outside = BTreeSet::new();
        for body in self.bodies.values() {
            match body {
                Body::Text(text) => {
                    outside.extend(text.relocs.iter().filter_map(|r| symbol(&r.name)))
                }
                Body::Native(root) => {
                    outside.extend(root.relocations.iter().filter_map(|r| symbol(&r.name)))
                }
                Body::Data(data) => outside.extend(data.function_decls.values().filter_map(symbol)),
            }
        }
        let names = std::env::var("MECH_COMPACT_INLINE_SYMBOLS").ok();
        bodies
            .iter()
            .filter(|(target, _)| {
                referenced.get(*target) == Some(&(1, true))
                    && !outside.contains(&FuncOrDataId::Func(**target))
                    && outside_loops.is_none_or(|callees| callees.contains(*target))
                    && names.as_deref().is_none_or(|selection| {
                        self.declarations
                            .get_function_decl(**target)
                            .name
                            .as_deref()
                            .is_some_and(|name| selection.split(',').any(|symbol| symbol == name))
                    })
            })
            .map(|(&id, function)| (id, function))
            .take(8)
            .collect()
    }

    fn compile_preselected_root(
        &self,
        context: &mut Context,
        control: &mut ControlPlane,
    ) -> ModuleResult<bool> {
        let Some(bodies) = self
            .inline_bodies
            .as_ref()
            .filter(|bodies| !bodies.is_empty())
        else {
            return Ok(false);
        };
        let references_helper = context
            .func
            .layout
            .blocks()
            .flat_map(|block| context.func.layout.block_insts(block))
            .any(|inst| {
                let callee = match context.func.dfg.insts[inst] {
                    ir::InstructionData::Call { func_ref, .. }
                    | ir::InstructionData::FuncAddr { func_ref, .. }
                    | ir::InstructionData::TryCall { func_ref, .. } => func_ref,
                    _ => return false,
                };
                super::inlining::module_callee(&context.func, callee)
                    .is_some_and(|callee| bodies.contains_key(&callee))
            });
        if !references_helper {
            return Ok(false);
        }
        // Use the same optimized IR that the compile-and-compare policy would
        // leave before inlining. Keep the original context for fallback.
        let mut prepared = Context::for_function(context.func.clone());
        let mut control_for_candidate = control.clone();
        let isa = self.isa();
        prepared
            .verify_if(isa)
            .and_then(|()| prepared.optimize(isa, &mut control_for_candidate))
            .map_err(|error| ModuleError::Backend(error.into()))?;
        let function = &prepared.func;
        let mut referenced = BTreeMap::<FuncId, (usize, bool)>::new();
        for inst in function
            .layout
            .blocks()
            .flat_map(|block| function.layout.block_insts(block))
        {
            let (callee, call) = match function.dfg.insts[inst] {
                ir::InstructionData::Call {
                    opcode, func_ref, ..
                } => (func_ref, opcode == ir::Opcode::Call),
                ir::InstructionData::FuncAddr { func_ref, .. }
                | ir::InstructionData::TryCall { func_ref, .. } => (func_ref, false),
                _ => continue,
            };
            if let Some(target) = super::inlining::module_callee(function, callee) {
                let external = &function.dfg.ext_funcs[callee];
                let entry = referenced.entry(target).or_default();
                entry.0 += 1;
                entry.1 |= call && external.colocated && !external.patchable;
            }
        }
        let selected = self.once_called_helpers(bodies, &referenced, None);
        if selected.is_empty() {
            return Ok(false);
        }
        let mut candidate = Context::for_function(prepared.func);
        if !candidate
            .inline(super::inlining::OnceCalled(&selected))
            .map_err(|error| ModuleError::Backend(error.into()))?
        {
            return Ok(false);
        }
        candidate
            .verify(isa)
            .map_err(|error| ModuleError::Backend(error.into()))?;
        let bytes = candidate
            .compile(isa, &mut control_for_candidate)?
            .buffer
            .data()
            .len();
        eprintln!(
            "compact native preselected inline: {} bodies, {bytes} bytes",
            selected.len()
        );
        *context = candidate;
        *control = control_for_candidate;
        Ok(true)
    }

    pub(in crate::native_artifact::program) fn define_llvm_root(
        &mut self,
        id: FuncId,
        context: &Context,
        toolchain: &NativeLlvmToolchain,
    ) -> ModuleResult<(u32, u64)> {
        let entry = self
            .declarations
            .get_function_decl(id)
            .name
            .as_deref()
            .ok_or_else(|| llvm_diagnostic_failure("compact LLVM root needs a name"))?;
        let ir = self.verified_llvm_definition(id, context, true)?;
        let root = toolchain
            .compile_compact_root(&ir, entry, &self.declarations)
            .map_err(llvm_diagnostic_failure)?;
        let facts = (root.code_bytes, root.frame_bytes);
        self.record(FuncOrDataId::Func(id), Body::Native(root))?;
        Ok(facts)
    }

    fn verified_llvm_definition(
        &self,
        id: FuncId,
        context: &Context,
        fold_readonly_data: bool,
    ) -> ModuleResult<String> {
        context
            .verify(self.isa())
            .map_err(llvm_diagnostic_failure)?;
        let data_objects = self
            .bodies
            .iter()
            .filter_map(|(id, body)| match (id, body) {
                (FuncOrDataId::Data(id), Body::Data(data)) => Some((*id, data)),
                _ => None,
            })
            .collect();
        compact_definition(
            &self.declarations,
            id,
            &context.func,
            &data_objects,
            fold_readonly_data,
        )
        .map_err(llvm_diagnostic_failure)
    }

    pub(in crate::native_artifact::program) fn direct_runtime_calls(&self) -> DirectRuntimeCalls {
        self.direct_runtime_calls
    }

    pub(in crate::native_artifact::program) fn use_direct_runtime_calls(&mut self, mode: &str) {
        if cfg!(all(target_arch = "aarch64", target_os = "macos")) && self.isa().name() == "aarch64"
        {
            self.direct_runtime_calls = match mode {
                "1" => DirectRuntimeCalls::All,
                "allocation" => DirectRuntimeCalls::Allocation,
                "allocation-and-byte-appends" => DirectRuntimeCalls::AllocationAndByteAppends,
                "free" => DirectRuntimeCalls::Free,
                _ => DirectRuntimeCalls::Disabled,
            };
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
                Some(Body::Native(root)) => {
                    pending.extend(
                        root.relocations
                            .iter()
                            .filter_map(|reloc| symbol(&reloc.name)),
                    );
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

    pub(in crate::native_artifact::program) fn place_exports_first(&mut self) {
        let declarations = &self.declarations;
        self.definition_order.sort_by_key(|id| match *id {
            FuncOrDataId::Func(id) => declarations.get_function_decl(id).linkage != Linkage::Export,
            FuncOrDataId::Data(_) => true,
        });
    }

    pub(in crate::native_artifact::program) fn place_layout_regions_last(&mut self) {
        let declarations = &self.declarations;
        self.definition_order.sort_by_key(|id| match *id {
            FuncOrDataId::Func(id) => declarations
                .get_function_decl(id)
                .name
                .as_deref()
                .is_some_and(|name| name.starts_with("mech_compact_checked_layout_")),
            FuncOrDataId::Data(_) => false,
        });
    }

    pub(in crate::native_artifact::program) fn finish(self) -> ModuleResult<ObjectProduct> {
        let (product, root) = self.finish_product()?;
        if root.is_some() {
            return Err(llvm_diagnostic_failure(
                "native root requires the compact linker",
            ));
        }
        Ok(product)
    }

    pub(in crate::native_artifact::program) fn finish_linked(
        self,
        toolchain: &NativeLlvmToolchain,
    ) -> ModuleResult<Vec<u8>> {
        let (product, root) = self.finish_product()?;
        let (root, private_entry) =
            root.ok_or_else(|| llvm_diagnostic_failure("missing native root for partial link"))?;
        let support = product.emit().map_err(llvm_diagnostic_failure)?;
        toolchain
            .link_compact_root(&support, &root, private_entry.as_deref())
            .map_err(llvm_diagnostic_failure)
    }

    fn finish_product(
        mut self,
    ) -> ModuleResult<(ObjectProduct, Option<(Vec<u8>, Option<String>)>)> {
        let reachable = self.reachable()?;
        let mut roots = self.bodies.iter().filter_map(|(&id, body)| match body {
            Body::Native(root) => Some((id, root)),
            _ => None,
        });
        let native = roots.next();
        if roots.next().is_some() {
            return Err(llvm_diagnostic_failure(
                "multiple native roots in the compact Object",
            ));
        }
        let root_id = native.map(|(id, _)| id);
        let native_refs: BTreeSet<_> = native
            .into_iter()
            .flat_map(|(_, root)| {
                root.relocations
                    .iter()
                    .filter_map(|reloc| symbol(&reloc.name))
            })
            .collect();
        let mut symbols = BTreeMap::new();
        // Preserve declaration and definition order among the retained symbols.
        for &id in &self.declaration_order {
            if !reachable.contains(&id) {
                continue;
            }
            let original_linkage = self.linkage(id);
            let linkage = if root_id == Some(id) {
                Linkage::Import
            } else if native_refs.contains(&id) && original_linkage == Linkage::Local {
                // These definitions must resolve across both input Objects
                // while remaining private to the linked program.
                Linkage::Hidden
            } else {
                original_linkage
            };
            let emitted = match id {
                FuncOrDataId::Func(id) => {
                    let decl = self.declarations.get_function_decl(id);
                    let emitted = match &decl.name {
                        Some(name) => {
                            self.object
                                .declare_function(name, linkage, &decl.signature)?
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
                                .declare_data(name, linkage, decl.writable, decl.tls)?
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
        let tail_layout = (self.isa().name() == "aarch64"
            && self.isa().flags().enable_regalloc_spill_pairs())
        .then(phase_layout::profiles)
        .flatten();
        let mut native_root = None;
        for id in self.definition_order {
            let body = self.bodies.remove(&id).unwrap();
            let Some(&emitted) = symbols.get(&id) else {
                continue;
            };
            match (emitted, body) {
                (FuncOrDataId::Func(emitted), Body::Text(mut text)) => {
                    for reloc in &mut text.relocs {
                        relocate(&symbols, &mut reloc.name)?;
                    }
                    if let (Some(profiles), FuncOrDataId::Func(id)) = (&tail_layout, id) {
                        let decl = self.declarations.get_function_decl(id);
                        if decl.linkage == Linkage::Local
                            && let Some(name) = &decl.name
                        {
                            phase_layout::pad_object_tail(profiles, name, &mut text.bytes)
                                .map_err(|error| ModuleError::Backend(error.into()))?;
                        }
                    }
                    self.object.define_function_bytes(
                        emitted,
                        text.alignment,
                        &text.bytes,
                        &text.relocs,
                    )?;
                }
                (FuncOrDataId::Func(_), Body::Native(root)) => {
                    let FuncOrDataId::Func(root_id) = id else {
                        unreachable!("native root changed kind");
                    };
                    let decl = self.declarations.get_function_decl(root_id);
                    let private_entry = matches!(decl.linkage, Linkage::Local | Linkage::Hidden)
                        .then(|| decl.name.as_ref().map(|name| format!("_{name}")))
                        .flatten();
                    native_root = Some((root.object, private_entry));
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
        Ok((self.object.finish(), native_root))
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

    fn declare_func_in_func(&mut self, id: FuncId, f: &mut ir::Function) -> ir::FuncRef {
        self.direct_runtime_calls
            .declare_in(&self.declarations, id, f)
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
        let declaration = self.declarations.get_function_decl(id);
        if declaration.name.as_deref() == Some("mech_compact_closed_checked")
            && let Some(path) = std::env::var_os("MECH_COMPACT_DUMP_LLVM_ROOT")
        {
            let ir = self.verified_llvm_definition(id, context, false)?;
            std::fs::write(path, ir).map_err(llvm_diagnostic_failure)?;
        }
        if let Some(dump) = &self.llvm_dump
            && let Some(name) = declaration.name.as_deref()
            && dump.symbols.contains(name)
        {
            let ir = self.verified_llvm_definition(id, context, false)?;
            std::fs::create_dir_all(&dump.directory).map_err(llvm_diagnostic_failure)?;
            std::fs::write(dump.directory.join(format!("{name}.ll")), ir)
                .map_err(llvm_diagnostic_failure)?;
        }
        let checked_root = declaration.name.as_deref() == Some("mech_compact_closed_checked");
        let checked_body = checked_root
            || declaration
                .name
                .as_deref()
                .is_some_and(|name| name.starts_with("mech_compact_checked_phase_"));
        let preselected = checked_body
            && self.isa().name() == "aarch64"
            && std::env::var_os("MECH_COMPACT_INLINE_SINGLE_CALLS").as_deref()
                == Some(std::ffi::OsStr::new("preselected"))
            && self.compile_preselected_root(context, control)?;
        if !preselected {
            context.compile(self.isa(), control)?;
        }
        if checked_body {
            if !preselected {
                self.inline_private_helpers(id, context, control)?;
            }
        } else if declaration.linkage == Linkage::Local
            && context.compiled_code().unwrap().buffer.data().len() <= MAX_INLINE_BYTES
            && context.func.dfg.num_values() <= MAX_INLINE_VALUES
            && let Some(bodies) = &mut self.inline_bodies
            && std::env::var("MECH_COMPACT_INLINE_SYMBOLS")
                .ok()
                .is_none_or(|symbols| {
                    declaration
                        .name
                        .as_deref()
                        .is_some_and(|name| symbols.split(',').any(|symbol| symbol == name))
                })
            && bodies.len() < MAX_INLINE_BODIES
        {
            bodies.insert(id, context.func.clone());
        }
        let buffer = &context.compiled_code().unwrap().buffer;
        let mut bytes = buffer.data().into();
        if checked_root
            && context.func.aarch64_branch_on_zero
            && let Some(profile) = std::env::var_os("MECH_COMPACT_ROOT_TAIL_PROFILE")
        {
            phase_layout::pad_checked_root_tail(
                std::path::Path::new(&profile),
                buffer.alignment,
                &mut bytes,
            )
            .map_err(|error| ModuleError::Backend(error.into()))?;
        }
        let relocs = buffer
            .relocs()
            .iter()
            .map(|reloc| ModuleReloc::from_mach_reloc(reloc, &context.func, id))
            .collect();
        self.record(
            FuncOrDataId::Func(id),
            Body::Text(Text {
                alignment: u64::from(buffer.alignment),
                bytes,
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

fn llvm_diagnostic_failure(error: impl ToString) -> ModuleError {
    ModuleError::Backend(std::io::Error::other(error.to_string()).into())
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
    fn places_exported_entry_before_its_local_dependency() {
        let mut module = module();
        let local = function(&mut module, "checked_fallback", Linkage::Local);
        let layout = function(&mut module, "mech_compact_checked_layout_1", Linkage::Local);
        let entry = function(&mut module, "entry", Linkage::Export);
        define(&mut module, layout, &[]);
        define(&mut module, local, &[]);
        define(&mut module, entry, &[local, layout]);
        module.place_exports_first();
        module.place_layout_regions_last();

        let product = module.finish().unwrap();
        let position = |name| {
            let symbol = product.object.symbol_id(name).unwrap();
            product.object.symbol(symbol).value
        };
        assert!(position(b"entry") < position(b"checked_fallback"));
        assert!(position(b"checked_fallback") < position(b"mech_compact_checked_layout_1"));
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

    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    #[test]
    fn compact_runtime_imports_can_use_direct_branches() {
        use cranelift_codegen::binemit::Reloc;
        let names = [
            "mech_compact_free",
            "mech_compact_allocate",
            "mech_compact_finish_byte_appends",
            "unrelated_import",
        ];
        for (mode, expected) in [
            ("free", &[0][..]),
            ("allocation-and-byte-appends", &[1, 2][..]),
        ] {
            let mut module = module();
            module.use_direct_runtime_calls(mode);
            let entry = function(&mut module, "entry", Linkage::Export);
            let providers = names.map(|name| function(&mut module, name, Linkage::Import));
            define(&mut module, entry, &providers);
            let Body::Text(body) = module.bodies.get(&FuncOrDataId::Func(entry)).unwrap() else {
                unreachable!();
            };
            for (index, provider) in providers.into_iter().enumerate() {
                let relocs = body
                    .relocs
                    .iter()
                    .filter(|r| symbol(&r.name) == Some(FuncOrDataId::Func(provider)))
                    .collect::<Vec<_>>();
                assert!(!relocs.is_empty());
                assert_eq!(
                    relocs.iter().any(|r| r.kind == Reloc::Arm64Call),
                    expected.contains(&index),
                    "function {} in {mode}",
                    names[index]
                );
            }
            assert!(!module.finish().unwrap().emit().unwrap().is_empty());
        }
    }
}
