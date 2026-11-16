//! Retain one definition for identical local direct-call numeric bodies.
use super::body_relocations::matching_relocations;
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::{Context, control::ControlPlane, ir::Signature};
use cranelift_module::{FuncId, Module, ModuleError, ModuleReloc};
use std::collections::BTreeMap;

struct PhaseBody {
    signature: Signature,
    code: Box<[u8]>,
    relocs: Vec<ModuleReloc>,
    function: FuncId,
    symbol: String,
}

pub(super) struct PhaseBodies {
    by_size: BTreeMap<(u32, usize), Vec<PhaseBody>>,
    share_relocations: bool,
}

impl PhaseBodies {
    pub(super) fn new(share_relocations: bool) -> Self {
        Self {
            by_size: BTreeMap::new(),
            share_relocations,
        }
    }
}

pub(super) fn define_body<M: Module>(
    module: &mut M,
    function: FuncId,
    symbol: &str,
    context: &mut Context,
    bodies: Option<&mut PhaseBodies>,
) -> Result<(FuncId, String), NativeCompileError> {
    if super::word_pairs::available(module.isa())
        && std::env::var_os("MECH_COMPACT_PAIR_WORDS")
            .is_some_and(|scope| scope == "helpers" || scope == "all")
    {
        let paired = super::word_pairs::fold(&mut context.func, None, false);
        if paired != 0 {
            context.verify(module.isa()).map_err(backend)?;
        }
    }
    let Some(bodies) = bodies else {
        module.define_function(function, context).map_err(backend)?;
        return Ok((function, symbol.into()));
    };

    context
        .compile(module.isa(), &mut ControlPlane::default())
        .map_err(ModuleError::from)
        .map_err(backend)?;
    let buffer = &context.compiled_code().unwrap().buffer;
    let bytes = buffer.data();
    let alignment = buffer.alignment;
    let relocs = buffer
        .relocs()
        .iter()
        .map(|reloc| ModuleReloc::from_mach_reloc(reloc, &context.func, function))
        .collect::<Vec<_>>();
    let candidates = (relocs.is_empty() || bodies.share_relocations)
        .then(|| bodies.by_size.entry((alignment, bytes.len())).or_default());
    // Phase addresses are used only as direct call targets. Relative branch
    // targets within each body can be compared by offset. External targets
    // must refer to exactly the same function or data.
    if let Some(retained) = candidates.as_ref().and_then(|candidates| {
        candidates.iter().find(|candidate| {
            candidate.signature == context.func.signature
                && candidate.code.as_ref() == bytes
                && matching_relocations(candidate.function, &candidate.relocs, function, &relocs)
        })
    }) {
        return Ok((retained.function, retained.symbol.clone()));
    }

    module
        .define_function_bytes(function, alignment.into(), bytes, &relocs)
        .map_err(backend)?;
    if let Some(candidates) = candidates {
        candidates.push(PhaseBody {
            signature: context.func.signature.clone(),
            code: bytes.into(),
            relocs,
            function,
            symbol: symbol.into(),
        });
    }
    Ok((function, symbol.into()))
}

#[cfg(test)]
#[path = "phase_bodies_tests.rs"]
mod tests;
