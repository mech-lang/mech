//! Existing total pointwise proofs and shared scalar/vector code, with only
//! dense current captures crossing this private diagnostic boundary.
use super::layout::Layout;
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::{common::ssa_words::Words, control::plan::CodePlan};
use crate::native_artifact::compiler::{backend, guard, offset};
use crate::native_artifact::program::{
    codegen::word_collections,
    prepare::{PreparedNativeProgram, word_collections::Region},
};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder, StackSlotData, StackSlotKind, Value, types};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::{FuncId, Module};

pub(super) struct Kernel {
    pub proof: Region,
    function: FuncId,
    observe: bool,
    pub summary: serde_json::Value,
}

pub(super) fn define<M: Module>(
    module: &mut M,
    prepared: &PreparedNativeProgram,
    layout: &Layout,
    observe: bool,
) -> Result<Vec<Kernel>, NativeCompileError> {
    prepared.metadata().word_collections.iter().map(|proof| {
        if layout.ty(proof.source).element()?.packed_bytes != Some(proof.input_width.into()) {
            return Err(backend("compact word source differs from its total proof"));
        }
        for &capture in &proof.captures {
            if layout.ty(capture).words != 1 || !layout.ty(capture).owners.is_empty() {
                return Err(backend("compact word capture is not one immutable scalar"));
            }
        }
        let (function, stack, bytes) = word_collections::define_compact(
            module, CodePlan::Program(prepared), proof,
        )?;
        Ok(Kernel {
            proof: proof.clone(), function, observe,
            summary: serde_json::json!({"collection":proof.collection,"iteration":proof.iteration,
                "header":proof.header,"symbol":format!("mech_compact_word_collection_{}",proof.collection),
                "capture_words":proof.captures.len(),"input_width":proof.input_width,
                "output_width":proof.output_width,"code_bytes":bytes,"stack_bytes":stack,
                "shared_total_proof":true,"shared_scalar_vector_emitter":true}),
        })
    }).collect()
}

impl Kernel {
    pub fn emit<M: Module>(
        &self,
        module: &mut M,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        layout: &Layout,
        source: Value,
        sink: Value,
        count: Value,
        error: Block,
    ) -> Result<(), NativeCompileError> {
        if let Some(observer) = super::phases::observer(module, b, self.observe)? {
            let header = b.ins().iconst(
                types::I64,
                ((1u64 << 62) | u64::from(self.proof.header)) as i64,
            );
            let selected = b.ins().iconst(types::I64, 1);
            // Kernel calls are observed only in the separate diagnostic image.
            // The primary image defines no observer import or call.
            let heap = b.ins().iconst(types::I64, 0);
            b.ins().call(observer, &[heap, header, selected]);
        }
        let captures = b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            (self.proof.captures.len().max(1) * 8) as u32,
            3,
        ));
        for (at, &capture) in self.proof.captures.iter().enumerate() {
            let value = words.read(b, layout.slot(capture));
            b.ins().stack_store(value, captures, offset(at)?);
        }
        let captures = b.ins().stack_addr(types::I64, captures, 0);
        let function = module.declare_func_in_func(self.function, b.func);
        let call = b.ins().call(function, &[captures, source, sink, count]);
        let status = b.inst_results(call)[0];
        let valid = b.ins().icmp_imm(IntCC::Equal, status, 0);
        // The shared range proof makes source failures unreachable for current
        // valid typed inputs. Keep the checked kernel's explicit status path.
        guard(b, valid, error, 0x19403);
        Ok(())
    }
}
