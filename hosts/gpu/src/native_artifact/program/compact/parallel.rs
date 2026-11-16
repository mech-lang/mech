//! Native entries for the existing canonical independent-worker proof.
//! Captures are current typed values; generated worker code borrows their
//! immutable backing for the synchronous create/run/join interval.
use super::flow::Flow;
use crate::native_artifact::program::prepare::parallel::Region;
use crate::native_artifact::{NativeCompileError, compiler::backend};
use mech_engine::native_artifact::{NativeControlLoop as Loop, NativeProgramArtifactPlan as Plan};

pub(super) struct Entry {
    pub symbol: String,
    body: usize,
    push: usize,
    output: u32,
    pub inputs: Vec<u32>,
    sites: Vec<u32>,
}
impl Entry {
    pub fn derive(plan: &Plan, region: &Region) -> Result<Self, NativeCompileError> {
        let Loop::Collection(generator) = &plan.iterations()[region.iteration as usize] else {
            return Err(backend("native worker lacks canonical generator"));
        };
        let element = generator
            .element()
            .ok_or_else(|| backend("native worker lacks typed element"))?;
        let mut inputs = region
            .captures
            .iter()
            .copied()
            .filter(|&value| !plan.constants().iter().any(|c| c.value().get() == value))
            .collect::<Vec<_>>();
        inputs.push(element.get());
        inputs.sort_unstable();
        inputs.dedup();
        Ok(Self {
            symbol: format!("mech_compact_worker_{}", region.collection),
            body: region.body as usize,
            push: region.push as usize,
            output: region.output,
            inputs,
            sites: region.sites.to_vec(),
        })
    }
    pub fn flow(&self, plan: &Plan) -> Result<Flow, NativeCompileError> {
        let flow = Flow::derive_entry(
            plan,
            self.body,
            self.inputs.clone(),
            vec![self.output],
            Some(self.push),
            Some(self.body),
        )?;
        if flow.reachable.iter().enumerate().any(|(site, &reachable)| {
            reachable && site != self.push && self.sites.binary_search(&(site as u32)).is_err()
        }) {
            return Err(backend(
                "native worker escapes its canonical independent body",
            ));
        }
        Ok(flow)
    }
}

use super::{arrays::Arrays, collections::Collections, layout::Layout, packed};
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::compiler::guard;
use cranelift_codegen::ir::{
    AbiParam, Block, InstBuilder, MemFlags, StackSlotData, StackSlotKind, condcodes::IntCC, types,
};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::{FuncId, Linkage, Module};
use mech_engine::{ComprehensionKind, MatrixBlockRows};

const MAX_WORKERS: usize = 8;
pub(super) struct Worker {
    pub entry: Entry,
    pub iteration: u32,
    collection: u32,
    header: u32,
    source: u32,
    element: u32,
    function: FuncId,
    pub report: serde_json::Value,
}
impl Worker {
    pub fn new<M: Module>(
        module: &mut M,
        plan: &Plan,
        region: &Region,
        entry: Entry,
        report: serde_json::Value,
    ) -> Result<Self, NativeCompileError> {
        let mut signature = module.make_signature();
        signature.params = vec![AbiParam::new(types::I64); 3];
        signature.returns = vec![AbiParam::new(types::I64); 2];
        let function = module
            .declare_function(&entry.symbol, Linkage::Local, &signature)
            .map_err(backend)?;
        let Loop::Collection(generator) = &plan.iterations()[region.iteration as usize] else {
            return Err(backend("native worker generator absent"));
        };
        Ok(Self {
            entry,
            iteration: region.iteration,
            collection: region.collection,
            header: region.header,
            source: region.source,
            element: generator.element().unwrap().get(),
            function,
            report,
        })
    }
    pub fn emit<M: Module>(
        &self,
        module: &mut M,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        layout: &Layout,
        arrays: &Arrays,
        collections: &Collections,
        plan: &Plan,
        exit: Block,
        error: Block,
    ) -> Result<(), NativeCompileError> {
        let kind = plan.collections()[self.collection as usize].kind();
        if !matches!(
            kind,
            ComprehensionKind::Matrix
                | ComprehensionKind::MatrixPreserveShape
                | ComprehensionKind::MatrixHorizontalBlocks {
                    rows: MatrixBlockRows::Fixed(1)
                }
        ) {
            return Ok(());
        }
        let block_output = matches!(kind, ComprehensionKind::MatrixHorizontalBlocks { .. });
        let source_ty = layout.ty(self.source).element()?;
        let output_ty = layout.ty(self.entry.output);
        let element = layout
            .ty(plan.collections()[self.collection as usize].output().get())
            .element()?;
        if block_output {
            if output_ty.element()?.as_ref() != element.as_ref() {
                return Err(backend("native worker block element differs"));
            }
        } else if output_ty.as_ref() != element.as_ref() {
            return Err(backend("native worker result element differs"));
        }
        let input_words = self
            .entry
            .inputs
            .iter()
            .map(|&v| layout.ty(v).words)
            .sum::<usize>();
        let output_words = output_ty.words;
        let bytes = |words: usize| {
            words
                .checked_mul(MAX_WORKERS)
                .and_then(|v| v.checked_mul(8))
                .and_then(|v| u32::try_from(v).ok())
                .ok_or_else(|| backend("native worker packet size overflow"))
        };
        let inputs = b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            bytes(input_words)?,
            3,
        ));
        let outputs = b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            bytes(output_words)?,
            3,
        ));
        let input = b.ins().stack_addr(types::I64, inputs, 0);
        let output = b.ins().stack_addr(types::I64, outputs, 0);
        let zero = b.ins().iconst(types::I64, 0);
        let one = b.ins().iconst(types::I64, 1);
        let (index, count) = collections.parallel_cursor(b, self.iteration)?;
        let initial = b.ins().icmp_imm(IntCC::Equal, index, 0);
        let enough = b.ins().icmp_imm(IntCC::UnsignedGreaterThan, count, 1);
        let bounded = b
            .ins()
            .icmp_imm(IntCC::UnsignedLessThanOrEqual, count, MAX_WORKERS as i64);
        let valid = b.ins().band(initial, enough);
        let valid = b.ins().band(valid, bounded);
        let fast = b.create_block();
        let ordinary = b.create_block();
        b.ins().brif(valid, fast, &[], ordinary, &[]);
        b.switch_to_block(fast);
        let source = words.read(b, layout.slot(self.source));
        let header = b.create_block();
        b.append_block_param(header, types::I64);
        let fill = b.create_block();
        let run = b.create_block();
        b.ins().jump(header, &[zero.into()]);
        b.switch_to_block(header);
        let at = b.block_params(header)[0];
        let more = b.ins().icmp(IntCC::UnsignedLessThan, at, count);
        b.ins().brif(more, fill, &[], run, &[]);
        b.switch_to_block(fill);
        let offset = b.ins().imul_imm(at, (input_words * 8) as i64);
        let packet = b.ins().iadd(input, offset);
        let offset = b.ins().imul_imm(at, source_ty.packed_bytes.unwrap() as i64);
        let selected = b.ins().iadd(source, offset);
        let selected = packed::load(b, selected, source_ty)?;
        let mut slot = 0;
        for &value in &self.entry.inputs {
            let ty = layout.ty(value);
            for lane in 0..ty.words {
                let word = if value == self.element {
                    selected[lane]
                } else if ty.owners.contains(&lane) {
                    zero
                } else {
                    words.read(b, layout.slot(value) + lane)
                };
                b.ins()
                    .store(MemFlags::new(), word, packet, (slot * 8) as i32);
                slot += 1;
            }
        }
        let next = b.ins().iadd_imm(at, 1);
        b.ins().jump(header, &[next.into()]);
        b.switch_to_block(run);
        let mut signature = module.make_signature();
        signature.params = vec![AbiParam::new(types::I64); 7];
        signature.returns = vec![AbiParam::new(types::I64); 2];
        let helper = module
            .declare_function("mech_compact_parallel", Linkage::Import, &signature)
            .map_err(backend)?;
        let helper = module.declare_func_in_func(helper, b.func);
        let function = module.declare_func_in_func(self.function, b.func);
        let function = b.ins().func_addr(types::I64, function);
        let in_words = b.ins().iconst(types::I64, input_words as i64);
        let out_words = b.ins().iconst(types::I64, output_words as i64);
        let call = b.ins().call(
            helper,
            &[
                arrays.heap,
                function,
                input,
                in_words,
                out_words,
                count,
                output,
            ],
        );
        let status = b.inst_results(call)[0];
        let origin = b.inst_results(call)[1];
        let runtime_failure = b.ins().icmp_imm(IntCC::Equal, origin, -1);
        let header = b.ins().iconst(types::I64, self.header as i64);
        let origin = b.ins().select(runtime_failure, header, origin);
        let failed = b.create_block();
        let success = b.create_block();
        b.ins().brif(status, failed, &[], success, &[]);
        b.switch_to_block(failed);
        b.ins().return_(&[status, origin]);
        b.switch_to_block(success);
        let total = if block_output {
            let sum = b.create_block();
            b.append_block_param(sum, types::I64);
            b.append_block_param(sum, types::I64);
            let add = b.create_block();
            let done = b.create_block();
            b.append_block_param(done, types::I64);
            b.ins().jump(sum, &[zero.into(), zero.into()]);
            b.switch_to_block(sum);
            let at = b.block_params(sum)[0];
            let total = b.block_params(sum)[1];
            let more = b.ins().icmp(IntCC::UnsignedLessThan, at, count);
            b.ins().brif(more, add, &[], done, &[total.into()]);
            b.switch_to_block(add);
            let offset = b.ins().imul_imm(at, (output_words * 8) as i64);
            let packet = b.ins().iadd(output, offset);
            let len = b.ins().load(types::I64, MemFlags::new(), packet, 8);
            let rows = b.ins().load(types::I64, MemFlags::new(), packet, 24);
            let row = b.ins().icmp_imm(IntCC::Equal, rows, 1);
            guard(b, row, error, 0x19403);
            let (total, overflow) = b.ins().uadd_overflow(total, len);
            let valid = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
            guard(b, valid, error, 1);
            let at = b.ins().iadd_imm(at, 1);
            b.ins().jump(sum, &[at.into(), total.into()]);
            b.switch_to_block(done);
            b.block_params(done)[0]
        } else {
            count
        };
        let mut view = arrays.allocate(b, total, element, false, error)?;
        let copy = b.create_block();
        b.append_block_param(copy, types::I64);
        b.append_block_param(copy, types::I64);
        let item = b.create_block();
        let done = b.create_block();
        b.ins().jump(copy, &[zero.into(), zero.into()]);
        b.switch_to_block(copy);
        let at = b.block_params(copy)[0];
        let position = b.block_params(copy)[1];
        let more = b.ins().icmp(IntCC::UnsignedLessThan, at, count);
        b.ins().brif(more, item, &[], done, &[]);
        b.switch_to_block(item);
        let offset = b.ins().imul_imm(at, (output_words * 8) as i64);
        let packet = b.ins().iadd(output, offset);
        let offset = b
            .ins()
            .imul_imm(position, element.packed_bytes.unwrap() as i64);
        let destination = b.ins().iadd(view[0], offset);
        let position = if block_output {
            let pointer = b.ins().load(types::I64, MemFlags::new(), packet, 0);
            let len = b.ins().load(types::I64, MemFlags::new(), packet, 8);
            let owner = b.ins().load(types::I64, MemFlags::new(), packet, 40);
            let bytes = b.ins().imul_imm(len, element.packed_bytes.unwrap() as i64);
            arrays.copy(b, destination, pointer, bytes);
            arrays.release(b, owner, error);
            b.ins().iadd(position, len)
        } else {
            let values = (0..output_words)
                .map(|lane| {
                    b.ins()
                        .load(types::I64, MemFlags::new(), packet, (lane * 8) as i32)
                })
                .collect::<Vec<_>>();
            packed::store(b, destination, element, &values)?;
            b.ins().iadd_imm(position, 1)
        };
        let next = b.ins().iadd_imm(at, 1);
        b.ins().jump(copy, &[next.into(), position.into()]);
        b.switch_to_block(done);
        view[1] = total;
        view[3] = one;
        view[4] = total;
        collections.parallel_result(
            b,
            arrays,
            self.collection,
            self.iteration,
            &view,
            count,
            error,
        )?;
        b.ins().jump(exit, &[]);
        b.switch_to_block(ordinary);
        Ok(())
    }
}
