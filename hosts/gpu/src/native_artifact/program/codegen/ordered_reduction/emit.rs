//! Compact ordered reduction; long current intervals also use four-lane costs.
use super::*;
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::compiler::primitive;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder, Value, types};
use cranelift_frontend::FunctionBuilder;

impl Reduction {
    pub fn emit(
        &self,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        proof: &Proof,
        shadow: usize,
        ordinary: Block,
    ) {
        let c = &self.candidate;
        let start = words.read(b, c.induction);
        let stop = words.read(b, c.count);
        let tag = words.read(b, c.tag);
        let active = b.ins().icmp_imm(IntCC::Equal, tag, c.active as i64);
        let more = b.ins().icmp(IntCC::UnsignedLessThan, start, stop);
        let remaining = b.ins().isub(stop, start);
        let fits = b
            .ins()
            .icmp_imm(IntCC::UnsignedLessThanOrEqual, stop, u32::MAX as i64);
        let mut valid = b.ins().band(active, more);
        valid = b.ins().band(valid, fits);
        for read in &self.reads {
            let view = shadow + proof.view(read.array);
            let rows = words.read(b, view + 2);
            let columns = words.read(b, view + 3);
            let row = b.ins().icmp_imm(IntCC::Equal, rows, 1);
            let column = b.ins().icmp_imm(IntCC::Equal, columns, 1);
            let flat = b.ins().bor(row, column);
            valid = b.ins().band(valid, flat);
        }
        let setup = b.create_block();
        b.ins().brif(valid, setup, &[], ordinary, &[]);
        b.switch_to_block(setup);
        let best = words.read(b, self.best);
        let best = b.ins().ireduce(types::I32, best);
        let inputs = super::cost::Inputs::new(b, words, proof, shadow, &self.cost);
        let scalar = b.create_block();
        for ty in [types::I64, types::I32, types::I32] {
            b.append_block_param(scalar, ty);
        }
        let vector = b.create_block();
        let missing = b.ins().iconst(types::I32, -1);
        let long = b
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 16);
        b.ins().brif(
            long,
            vector,
            &[],
            scalar,
            &[start.into(), best.into(), missing.into()],
        );
        b.switch_to_block(vector);
        let prefix = b.ins().band_imm(remaining, -4);
        let end = b.ins().iadd(start, prefix);
        let costs = b.ins().splat(types::I32X4, best);
        let indices = super::cost::constant(b, [u32::MAX; 4]);
        let lanes = super::cost::constant(b, [0, 1, 2, 3]);
        let header = b.create_block();
        let body = b.create_block();
        let finish = b.create_block();
        for ty in [types::I64, types::I32X4, types::I32X4] {
            b.append_block_param(header, ty);
        }
        b.ins()
            .jump(header, &[start.into(), costs.into(), indices.into()]);
        b.switch_to_block(header);
        let params = b.block_params(header).to_vec();
        let k = params[0];
        let more = b.ins().icmp(IntCC::UnsignedLessThan, k, end);
        b.ins().brif(more, body, &[], finish, &[]);
        b.switch_to_block(body);
        let k32 = b.ins().ireduce(types::I32, k);
        let index = b.ins().splat(types::I32X4, k32);
        let index = b.ins().iadd(index, lanes);
        let cost = inputs.emit(b, proof, &self.cost, k, index);
        let better = b.ins().icmp(IntCC::UnsignedLessThan, cost, params[1]);
        let costs = b.ins().umin(cost, params[1]);
        let indices = b.ins().bitselect(better, index, params[2]);
        let next = b.ins().iadd_imm(k, 4);
        b.ins()
            .jump(header, &[next.into(), costs.into(), indices.into()]);
        b.switch_to_block(finish);
        let mut minimum = best;
        let mut winner = missing;
        for lane in 0..4 {
            let cost = b.ins().extractlane(params[1], lane);
            let index = b.ins().extractlane(params[2], lane);
            let less = b.ins().icmp(IntCC::UnsignedLessThan, cost, minimum);
            let equal = b.ins().icmp(IntCC::Equal, cost, minimum);
            let earlier = b.ins().icmp(IntCC::UnsignedLessThan, index, winner);
            let tie = b.ins().band(equal, earlier);
            let choose = b.ins().bor(less, tie);
            minimum = b.ins().select(choose, cost, minimum);
            winner = b.ins().select(choose, index, winner);
        }
        b.ins()
            .jump(scalar, &[end.into(), minimum.into(), winner.into()]);
        b.switch_to_block(scalar);
        let params = b.block_params(scalar).to_vec();
        let body = b.create_block();
        let finish = b.create_block();
        let more = b.ins().icmp(IntCC::UnsignedLessThan, params[0], stop);
        b.ins().brif(more, body, &[], finish, &[]);
        b.switch_to_block(body);
        let cost = inputs.scalar(b, proof, &self.cost, params[0]);
        let less = b.ins().icmp(IntCC::UnsignedLessThan, cost, params[1]);
        let minimum = b.ins().umin(cost, params[1]);
        let index = b.ins().ireduce(types::I32, params[0]);
        let winner = b.ins().select(less, index, params[2]);
        let next = b.ins().iadd_imm(params[0], 1);
        b.ins()
            .jump(scalar, &[next.into(), minimum.into(), winner.into()]);
        b.switch_to_block(finish);
        let minimum = params[1];
        let winner = params[2];
        let selected = b.create_block();
        let done = b.create_block();
        b.append_block_param(done, types::I64);
        let fallback = b.create_block();
        b.append_block_param(fallback, types::I64);
        let old_item = words.read(b, self.item);
        let changed = b.ins().icmp_imm(IntCC::NotEqual, winner, -1);
        b.ins()
            .brif(changed, selected, &[], done, &[old_item.into()]);
        b.switch_to_block(selected);
        let winner = b.ins().uextend(types::I64, winner);
        let item = emit_item(b, words, proof, shadow, &self.selected, winner, fallback);
        b.ins().jump(done, &[item.into()]);
        b.switch_to_block(fallback);
        // The pure reduction has not changed source state. Preserve the exact
        // original body if a shared scalar check unexpectedly declines.
        b.ins().jump(ordinary, &[]);
        b.switch_to_block(done);
        let item = b.block_params(done)[0];
        let minimum = b.ins().uextend(types::I64, minimum);
        let terminal = b.ins().iconst(types::I64, c.terminal as i64);
        words.write(b, c.tag, terminal);
        words.write(b, c.induction, stop);
        words.write(b, self.best, minimum);
        words.write(b, self.item, item);
        // Exact original prices/visits were already settled. The recorded
        // terminal dependency proof excludes skipped private temporaries, and
        // the original terminal traversal constructs the returned value.
        b.ins().jump(ordinary, &[]);
    }
}

fn emit_item(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    proof: &Proof,
    shadow: usize,
    item: &Item,
    index: Value,
    error: Block,
) -> Value {
    match item {
        Item::Constant(value) => b.ins().iconst(types::I64, *value as i64),
        Item::Word(word) => words.read(b, *word),
        Item::Index => index,
        Item::Length(array) => words.read(b, shadow + proof.view(*array) + 1),
        Item::Scalar(op, args) => {
            let args = args
                .iter()
                .map(|arg| emit_item(b, words, proof, shadow, arg, index, error))
                .collect::<Vec<_>>();
            primitive(b, *op, &args, error)
        }
    }
}
