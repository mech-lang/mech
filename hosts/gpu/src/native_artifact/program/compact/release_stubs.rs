//! Share the checked entry's repeated argument setup for checked heap calls.
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::{
    binemit::Reloc,
    cursor::{Cursor, FuncCursor},
    ir::{
        AbiParam, FuncRef, Function, InstBuilder, InstructionData, Opcode, StackSlot,
        StackSlotData, StackSlotKind, types,
    },
    isa::CallConv,
};
use cranelift_module::{FuncId, Linkage, Module, ModuleReloc, ModuleRelocTarget};
use std::collections::BTreeMap;

const RELEASES: [&str; 5] = [
    "mech_compact_release_owner",
    "mech_compact_release_nullable",
    "mech_compact_release_two",
    "mech_compact_release_three",
    "mech_compact_release_four",
];

struct Stub {
    function: FuncId,
    target: FuncId,
    operands: u8,
}

pub(super) struct ReleaseStubs {
    heap_slot: StackSlot,
    stubs: Vec<Stub>,
}

impl ReleaseStubs {
    pub(super) fn prepare<M: Module>(
        module: &mut M,
        root: &mut Function,
    ) -> Result<Option<Self>, NativeCompileError> {
        if module.isa().name() != "aarch64" || root.signature.call_conv != CallConv::AppleAarch64 {
            return Ok(None);
        }
        let Some(entry) = root.layout.entry_block() else {
            return Ok(None);
        };
        let Some(&heap) = root.dfg.block_params(entry).first() else {
            return Ok(None);
        };
        let word = AbiParam::new(types::I64);
        let scope = std::env::var("MECH_COMPACT_SHARE_BACKING_SETUP").ok();
        let backing = matches!(scope.as_deref(), Some("all" | "backing"));
        let append = matches!(scope.as_deref(), Some("all" | "append"));
        let calls = root
            .layout
            .blocks()
            .flat_map(|block| root.layout.block_insts(block));
        let mut replace = Vec::new();
        let mut targets = BTreeMap::new();
        for inst in calls {
            let InstructionData::Call {
                opcode: Opcode::Call,
                func_ref,
                ..
            } = root.dfg.insts[inst]
            else {
                continue;
            };
            let Some(target) = super::inlining::module_callee(root, func_ref) else {
                continue;
            };
            let declaration = module.declarations().get_function_decl(target);
            let Some(name) = declaration.name.as_deref() else {
                continue;
            };
            let arguments = root.dfg.inst_args(inst);
            let signature = &declaration.signature;
            let params = signature.params.len();
            let returns = match (declaration.linkage, name) {
                (Linkage::Import, name)
                    if RELEASES.contains(&name) && (2..=5).contains(&params) =>
                {
                    1
                }
                (Linkage::Import, "mech_compact_allocate_backing") if backing && params == 5 => 2,
                (Linkage::Import, "mech_compact_writable_scalar_backing")
                    if backing && params == 7 =>
                {
                    2
                }
                (Linkage::Local, "mech_compact_append_backing_byte_consume")
                    if append && params == 6 =>
                {
                    3
                }
                _ => continue,
            };
            // Keep the target's original argument and result ABI. Only the
            // same immutable first heap word moves into this private frame.
            if signature.call_conv != CallConv::AppleAarch64
                || signature.params.iter().any(|parameter| *parameter != word)
                || signature.returns.len() != returns
                || signature.returns.iter().any(|parameter| *parameter != word)
                || root.dfg.signatures[root.dfg.ext_funcs[func_ref].signature] != *signature
                || arguments.len() != signature.params.len()
                || root.dfg.resolve_aliases(arguments[0]) != heap
            {
                continue;
            }
            targets.entry(target).or_insert_with(|| {
                (
                    format!("{name}_from_frame"),
                    signature.clone(),
                    (params - 1) as u8,
                )
            });
            replace.push((inst, target));
        }
        if replace.is_empty() {
            return Ok(None);
        }

        let heap_slot =
            root.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
        // A regular direct branch-and-link keeps the caller's SP. The
        // private stubs read this invariant, invocation-local slot.
        FuncCursor::new(root)
            .at_first_inst(entry)
            .ins()
            .stack_store(heap, heap_slot, 0);
        let mut imports = BTreeMap::<FuncId, FuncRef>::new();
        let mut stubs = Vec::new();
        for (target, (name, mut signature, operands)) in targets {
            signature.params.remove(0);
            let function = module
                .declare_function(&name, Linkage::Local, &signature)
                .map_err(backend)?;
            imports.insert(target, module.declare_func_in_func(function, root));
            stubs.push(Stub {
                function,
                target: target,
                operands,
            });
        }
        let relocated_backings = stubs
            .iter()
            .filter(|stub| {
                module
                    .declarations()
                    .get_function_decl(stub.target)
                    .name
                    .as_deref()
                    .is_some_and(|name| !RELEASES.contains(&name))
            })
            .count();
        if relocated_backings != 0 {
            eprintln!("compact root shared backing setup: {relocated_backings} additional stubs");
        }
        eprintln!(
            "compact root shared release setup: {} calls in {} stubs",
            replace.len(),
            stubs.len()
        );
        for (inst, target) in replace {
            let operands = root.dfg.inst_args(inst)[1..].to_vec();
            root.dfg.replace(inst).call(imports[&target], &operands);
        }
        Ok(Some(Self { heap_slot, stubs }))
    }

    pub(super) fn define<M: Module>(
        self,
        module: &mut M,
        frame: &cranelift_codegen::MachBufferFrameLayout,
    ) -> Result<(), NativeCompileError> {
        let offset = frame
            .stackslots
            .get(self.heap_slot)
            .ok_or_else(|| backend("release setup lost its heap slot"))?
            .offset;
        if offset % 8 != 0
            || offset / 8 > 4095
            || offset
                .checked_add(8)
                .is_none_or(|end| end > frame.frame_to_fp_offset)
        {
            return Err(backend(
                "release setup slot is outside the addressable checked frame",
            ));
        }
        for stub in self.stubs {
            let mut code = Vec::with_capacity((usize::from(stub.operands) + 2) * 4);
            // Shift the operands into their original registers before restoring x0. The
            // tail branch preserves the original return address and status.
            for destination in (1..=u32::from(stub.operands)).rev() {
                let mov = 0xaa00_03e0_u32 | ((destination - 1) << 16) | destination;
                code.extend(mov.to_le_bytes());
            }
            let load_heap = 0xf940_03e0_u32 | (offset / 8) << 10;
            code.extend(load_heap.to_le_bytes());
            let branch_offset = code.len() as u32;
            code.extend(0x1400_0000_u32.to_le_bytes());
            module
                .define_function_bytes(
                    stub.function,
                    4,
                    &code,
                    &[ModuleReloc {
                        offset: branch_offset,
                        kind: Reloc::Arm64Call,
                        name: ModuleRelocTarget::user(0, stub.target.as_u32()),
                        addend: 0,
                    }],
                )
                .map_err(backend)?;
        }
        Ok(())
    }
}
