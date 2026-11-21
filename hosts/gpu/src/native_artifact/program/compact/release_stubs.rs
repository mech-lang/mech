//! Share the checked entry's repeated argument setup for checked heap calls.
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::{
    binemit::Reloc,
    cursor::{Cursor, FuncCursor},
    ir::{
        AbiParam, FuncRef, Function, InstBuilder, InstructionData, Opcode, StackSlot,
        StackSlotData, StackSlotKind, Value, ValueDef, types,
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
    allocation_geometry: Option<(u64, bool)>,
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
        Self::prepare_invariant(module, root, heap, None)
    }

    pub(super) fn prepare_captured<M: Module>(
        module: &mut M,
        function: &mut Function,
        phase: usize,
    ) -> Result<Option<Self>, NativeCompileError> {
        let Some(entry) = function.layout.entry_block() else {
            return Ok(None);
        };
        let heap = function.layout.blocks().find_map(|block| {
            function.layout.block_insts(block).find_map(|inst| {
                let InstructionData::Call { func_ref, .. } = function.dfg.insts[inst] else {
                    return None;
                };
                let target = super::inlining::module_callee(function, func_ref)?;
                let declaration = module.declarations().get_function_decl(target);
                if declaration.linkage != Linkage::Import
                    || !declaration.name.as_deref().is_some_and(|name| {
                        RELEASES.contains(&name) || name == "mech_compact_allocate_backing"
                    })
                {
                    return None;
                }
                let &heap = function.dfg.inst_args(inst).first()?;
                let heap = function.dfg.resolve_aliases(heap);
                // The captured word was already loaded unconditionally before
                // this phase's control flow. Do not lift a conditional value.
                let ValueDef::Result(producer, _) = function.dfg.value_def(heap) else {
                    return None;
                };
                let capture_load = match function.dfg.insts[producer].opcode() {
                    Opcode::Load => true,
                    Opcode::Isplit => {
                        let &[pair] = function.dfg.inst_args(producer) else {
                            return None;
                        };
                        let pair = function.dfg.resolve_aliases(pair);
                        let ValueDef::Result(load, _) = function.dfg.value_def(pair) else {
                            return None;
                        };
                        function.layout.inst_block(load) == Some(entry)
                            && function.dfg.insts[load].opcode() == Opcode::Load
                            && function.dfg.value_type(pair) == types::I128
                    }
                    _ => false,
                };
                (function.layout.inst_block(producer) == Some(entry)
                    && function.dfg.value_type(heap) == types::I64
                    && capture_load)
                    .then_some(heap)
            })
        });
        let Some(heap) = heap else {
            return Ok(None);
        };
        Self::prepare_invariant(module, function, heap, Some(phase))
    }

    fn prepare_invariant<M: Module>(
        module: &mut M,
        root: &mut Function,
        heap: Value,
        phase: Option<usize>,
    ) -> Result<Option<Self>, NativeCompileError> {
        if module.isa().name() != "aarch64" || root.signature.call_conv != CallConv::AppleAarch64 {
            return Ok(None);
        }
        let entry = root.layout.entry_block().expect("proved entry");
        let word = AbiParam::new(types::I64);
        let scope = std::env::var("MECH_COMPACT_SHARE_BACKING_SETUP").ok();
        let backing = matches!(scope.as_deref(), Some("all" | "backing"));
        let append = matches!(scope.as_deref(), Some("all" | "append"));
        let literal_allocation =
            std::env::var_os("MECH_COMPACT_LITERAL_ALLOCATION_FROM_FRAME").is_some();
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
            let allocation_geometry = (literal_allocation
                && name == "mech_compact_allocate_backing")
                .then(|| {
                    let stride = super::allocation_stubs::constant(root, arguments[2])?;
                    let alignment = super::allocation_stubs::constant(root, arguments[3])?;
                    let zeroed = super::allocation_stubs::constant(root, arguments[4])?;
                    (matches!(stride, 1 | 2 | 4 | 8) && alignment == stride)
                        .then_some((stride, zeroed != 0))
                })
                .flatten();
            let key = (target, allocation_geometry);
            targets.entry(key).or_insert_with(|| {
                let name = match allocation_geometry {
                    Some((stride, zeroed)) => {
                        format!(
                            "{name}_stride_{stride}_zeroed_{}_from_frame",
                            u8::from(zeroed)
                        )
                    }
                    None => format!("{name}_from_frame"),
                };
                let name = if let Some(phase) = phase {
                    format!("{name}_phase_{phase}")
                } else {
                    name
                };
                (name, signature.clone(), (params - 1) as u8)
            });
            replace.push((inst, key));
        }
        if replace.is_empty() {
            return Ok(None);
        }

        let heap_slot =
            root.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
        // A regular direct branch-and-link keeps the caller's SP. The
        // private stubs read this invariant, invocation-local slot.
        let producer = match root.dfg.value_def(heap) {
            ValueDef::Result(inst, _) => Some(inst),
            _ => None,
        };
        let cursor = FuncCursor::new(root);
        let mut cursor = if let Some(producer) = producer {
            cursor.after_inst(producer)
        } else {
            cursor.at_first_inst(entry)
        };
        cursor.ins().stack_store(heap, heap_slot, 0);
        let mut imports = BTreeMap::<(FuncId, Option<(u64, bool)>), FuncRef>::new();
        let mut stubs = Vec::new();
        for (key, (name, mut signature, operands)) in targets {
            let (target, allocation_geometry) = key;
            signature.params.remove(0);
            if allocation_geometry.is_some() {
                signature.params.truncate(1);
            }
            let function = module
                .declare_function(&name, Linkage::Local, &signature)
                .map_err(backend)?;
            imports.insert(key, module.declare_func_in_func(function, root));
            stubs.push(Stub {
                function,
                target,
                operands,
                allocation_geometry,
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
        let literal_stubs = stubs
            .iter()
            .filter(|stub| stub.allocation_geometry.is_some())
            .count();
        if literal_stubs != 0 {
            eprintln!("compact root literal allocation frame stubs: {literal_stubs}");
        }
        eprintln!(
            "compact root shared release setup: {} calls in {} stubs",
            replace.len(),
            stubs.len()
        );
        for (inst, key) in replace {
            let arguments = root.dfg.inst_args(inst);
            let operands = if key.1.is_some() {
                vec![arguments[1]]
            } else {
                arguments[1..].to_vec()
            };
            root.dfg.replace(inst).call(imports[&key], &operands);
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
            // Shift live arguments before restoring literals and the original
            // heap. The tail branch retains SP, LR and the runtime result ABI.
            let operands = if stub.allocation_geometry.is_some() {
                1
            } else {
                u32::from(stub.operands)
            };
            for destination in (1..=operands).rev() {
                let mov = 0xaa00_03e0_u32 | ((destination - 1) << 16) | destination;
                code.extend(mov.to_le_bytes());
            }
            if let Some((stride, zeroed)) = stub.allocation_geometry {
                for (register, value) in [(2u32, stride), (3, stride), (4, u64::from(zeroed))] {
                    let mov = 0xd280_0000u32 | ((value as u32) << 5) | register;
                    code.extend(mov.to_le_bytes());
                }
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
