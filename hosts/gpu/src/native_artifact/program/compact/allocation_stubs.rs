//! Share proved literal call setup without cloning the checked allocation policy.
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::binemit::Reloc;
use cranelift_codegen::ir::{
    AbiParam, Function, InstBuilder, InstructionData, Opcode, Value, ValueDef, types,
};
use cranelift_module::{Linkage, Module, ModuleReloc, ModuleRelocTarget};
use std::collections::BTreeMap;

fn constant(f: &Function, value: Value) -> Option<u64> {
    let value = f.dfg.resolve_aliases(value);
    let ValueDef::Result(inst, 0) = f.dfg.value_def(value) else {
        return None;
    };
    match f.dfg.insts[inst] {
        InstructionData::UnaryImm {
            opcode: Opcode::Iconst,
            imm,
        } if f.dfg.value_type(value) == types::I64 => Some(imm.bits() as u64),
        _ => None,
    }
}

pub(super) fn specialize<M: Module>(
    module: &mut M,
    f: &mut Function,
) -> Result<usize, NativeCompileError> {
    use cranelift_codegen::isa::CallConv;
    if !cfg!(all(target_arch = "aarch64", target_os = "macos"))
        || module.isa().name() != "aarch64"
        || f.signature.call_conv != CallConv::AppleAarch64
    {
        return Ok(0);
    }
    let word = AbiParam::new(types::I64);
    let mut calls = Vec::new();
    for block in f.layout.blocks() {
        for inst in f.layout.block_insts(block) {
            let InstructionData::Call {
                opcode: Opcode::Call,
                func_ref,
                ..
            } = f.dfg.insts[inst]
            else {
                continue;
            };
            let Some(target) = super::inlining::module_callee(f, func_ref) else {
                continue;
            };
            let decl = module.declarations().get_function_decl(target);
            if decl.linkage != Linkage::Import
                || decl.name.as_deref() != Some("mech_compact_allocate_backing")
                || decl.signature.call_conv != CallConv::AppleAarch64
                || decl.signature.params.as_slice() != [word; 5]
                || decl.signature.returns.as_slice() != [word; 2]
                || f.dfg.signatures[f.dfg.ext_funcs[func_ref].signature] != decl.signature
            {
                continue;
            }
            let args = f.dfg.inst_args(inst);
            let [heap, capacity, stride, alignment, zeroed] = args else {
                continue;
            };
            let Some(stride @ (1 | 2 | 4 | 8)) = constant(f, *stride) else {
                continue;
            };
            if constant(f, *alignment) != Some(stride) {
                continue;
            }
            let Some(zeroed) = constant(f, *zeroed) else {
                continue;
            };
            calls.push((inst, target, *heap, *capacity, stride, zeroed != 0));
        }
    }
    let mut imports = BTreeMap::new();
    for &(_, target, _, _, stride, zeroed) in &calls {
        if imports.contains_key(&(stride, zeroed)) {
            continue;
        }
        let mut signature = module.make_signature();
        signature.call_conv = CallConv::AppleAarch64;
        signature.params.extend([word; 2]);
        signature.returns.extend([word; 2]);
        let stub = module
            .declare_anonymous_function(&signature)
            .map_err(backend)?;
        let mut code = Vec::with_capacity(16);
        // x0 and x1 already carry the original heap and capacity. The private
        // leaf supplies only the three proved literal arguments, then tail
        // branches to the original checked provider without changing LR or SP.
        for (register, value) in [(2u32, stride), (3, stride), (4, u64::from(zeroed))] {
            code.extend((0xd280_0000u32 | ((value as u32) << 5) | register).to_le_bytes());
        }
        let branch_offset = code.len() as u32;
        code.extend(0x1400_0000u32.to_le_bytes());
        module
            .define_function_bytes(
                stub,
                4,
                &code,
                &[ModuleReloc {
                    offset: branch_offset,
                    kind: Reloc::Arm64Call,
                    name: ModuleRelocTarget::user(0, target.as_u32()),
                    addend: 0,
                }],
            )
            .map_err(backend)?;
        let import = module.declare_func_in_func(stub, f);
        imports.insert((stride, zeroed), import);
    }
    for &(inst, _, heap, capacity, stride, zeroed) in &calls {
        // The original two result Values and all subsequent status/owner checks
        // retain their identities; only three proved literal arguments disappear.
        f.dfg
            .replace(inst)
            .call(imports[&(stride, zeroed)], &[heap, capacity]);
    }
    Ok(calls.len())
}
