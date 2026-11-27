//! Emit the bounded path for the verified packed tree, leaving the checked path intact.
use super::tree::Tree;
use cranelift_codegen::{
    cursor::{Cursor, FuncCursor},
    ir::{Block, Function, InstBuilder, MemFlags, Value, condcodes::IntCC, types},
};

fn block(f: &mut Function) -> Block {
    let block = f.dfg.make_block();
    f.layout.append_block(block);
    block
}

fn address(b: &mut FuncCursor<'_>, data: Value, index: Value) -> Value {
    let bytes = b.ins().ishl_imm(index, 2);
    b.ins().iadd(data, bytes)
}

pub(crate) fn lower(f: &mut Function, tree: Tree) {
    let loop_top = block(f);
    let both_leaves = block(f);
    let use_branch = block(f);
    let both_branches = block(f);
    let mixed = block(f);
    let next = block(f);
    let visit = block(f);
    let [leaf, branch, built] =
        core::array::from_fn(|_| f.dfg.append_block_param(loop_top, types::I64));
    let next_leaf = f.dfg.append_block_param(next, types::I64);
    let next_branch = f.dfg.append_block_param(next, types::I64);
    let frequency = f.dfg.append_block_param(next, types::I32);

    let at = tree.original.block(&f.dfg.value_lists);
    let args = tree.original.args(&f.dfg.value_lists).collect::<Vec<_>>();
    let mut b = FuncCursor::new(f).at_inst(tree.entry);
    let [start_leaf, start_branch, start_built] = tree.cursors;
    let position = b.ins().bor(start_leaf, start_branch);
    let position = b.ins().bor(position, start_built);
    let initial = b.ins().icmp_imm(IntCC::Equal, position, 0);
    let bounded = b.ins().icmp_imm(IntCC::UnsignedLessThan, tree.last, 512);
    let extent = b.ins().iadd_imm(tree.last, 3);
    let fits = b
        .ins()
        .icmp(IntCC::UnsignedLessThanOrEqual, extent, tree.length);
    let valid = b.ins().band(initial, bounded);
    let valid = b.ins().band(valid, fits);
    b.func.dfg.replace(tree.entry).brif(
        valid,
        loop_top,
        &[start_leaf.into(), start_branch.into(), start_built.into()],
        at,
        &args,
    );

    b.goto_bottom(loop_top);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, built, tree.last);
    b.ins().brif(more, visit, &[], tree.success, &[]);

    b.goto_bottom(visit);
    let symbols = 1023;
    let frequencies = 0xffff_fc00u32 as i32 as i64;
    let flags = MemFlags::trusted();
    let leaf_addr = address(&mut b, tree.data, leaf);
    let branch_addr = address(&mut b, tree.data, branch);
    let l0 = b.ins().load(types::I32, flags, leaf_addr, 0);
    let l1 = b.ins().load(types::I32, flags, leaf_addr, 4);
    let r0 = b.ins().load(types::I32, flags, branch_addr, 0);
    let r1 = b.ins().load(types::I32, flags, branch_addr, 4);
    let fl0 = b.ins().band_imm(l0, frequencies);
    let fl1 = b.ins().band_imm(l1, frequencies);
    let fr0 = b.ins().band_imm(r0, frequencies);
    let fr1 = b.ins().band_imm(r1, frequencies);
    let next_l = b.ins().iadd_imm(leaf, 1);
    let next_r = b.ins().iadd_imm(branch, 1);
    let after_l = b.ins().iadd_imm(leaf, 2);
    let after_r = b.ins().iadd_imm(branch, 2);
    let have_two_leaves = b
        .ins()
        .icmp(IntCC::UnsignedLessThanOrEqual, next_l, tree.last);
    let branches_empty = b.ins().icmp(IntCC::Equal, branch, built);
    let leaves_cheaper = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, fl1, fr0);
    let prefer_leaves = b.ins().bor(branches_empty, leaves_cheaper);
    let take_leaves = b.ins().band(have_two_leaves, prefer_leaves);
    b.ins().brif(take_leaves, both_leaves, &[], use_branch, &[]);

    b.goto_bottom(both_leaves);
    let sum = b.ins().iadd(fl0, fl1);
    b.ins()
        .jump(next, &[after_l.into(), branch.into(), sum.into()]);

    b.goto_bottom(use_branch);
    let node = b.ins().ireduce(types::I32, built);
    let parent = b.ins().ishl_imm(node, 10);
    let symbol = b.ins().band_imm(r0, symbols);
    let attached = b.ins().bor(parent, symbol);
    b.ins().store(flags, attached, branch_addr, 0);
    let have_two_branches = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, after_r, built);
    let leaves_empty = b.ins().icmp(IntCC::UnsignedGreaterThan, leaf, tree.last);
    let branches_cheaper = b.ins().icmp(IntCC::UnsignedLessThan, fr1, fl0);
    let prefer_branches = b.ins().bor(leaves_empty, branches_cheaper);
    let take_branches = b.ins().band(have_two_branches, prefer_branches);
    b.ins().brif(take_branches, both_branches, &[], mixed, &[]);

    b.goto_bottom(both_branches);
    let symbol = b.ins().band_imm(r1, symbols);
    let attached = b.ins().bor(parent, symbol);
    b.ins().store(flags, attached, branch_addr, 4);
    let sum = b.ins().iadd(fr0, fr1);
    b.ins()
        .jump(next, &[leaf.into(), after_r.into(), sum.into()]);

    b.goto_bottom(mixed);
    let sum = b.ins().iadd(fl0, fr0);
    b.ins()
        .jump(next, &[next_l.into(), next_r.into(), sum.into()]);

    b.goto_bottom(next);
    let destination = address(&mut b, tree.data, built);
    let old = b.ins().load(types::I32, flags, destination, 0);
    let symbol = b.ins().band_imm(old, symbols);
    let node = b.ins().bor(frequency, symbol);
    b.ins().store(flags, node, destination, 0);
    let next_built = b.ins().iadd_imm(built, 1);
    b.ins().jump(
        loop_top,
        &[next_leaf.into(), next_branch.into(), next_built.into()],
    );
}
