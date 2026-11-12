//! Reuse an equal interval across entries into a certified immutable scan.
//! The saved interval lives in the current enclosing native loop activation.
use super::{Certificate, Change, Resolve, Version};
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::program::prepare::readonly_numeric::{Operation, Proof};
use cranelift_codegen::Context;
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    Block, BlockArg, Function, InstBuilder, StackSlot, StackSlotData, StackSlotKind, Value, types,
};
use mech_engine::native_artifact::{NativeControlLoop, NativeProgramArtifactPlan, NativeTailLoop};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Scope {
    pub(super) block: Block,
    pub(super) header: usize,
    pub(super) array: usize,
}

fn tail(plan: &NativeProgramArtifactPlan, iteration: u32) -> Option<&NativeTailLoop> {
    match plan.iterations().get(iteration as usize)? {
        NativeControlLoop::Tail(tail) => Some(tail),
        _ => None,
    }
}

fn inside(plan: &NativeProgramArtifactPlan, site: usize, outer: u32) -> bool {
    let mut current = plan.blocks()[site].iteration();
    while let Some(iteration) = current {
        if iteration == outer {
            return true;
        }
        current = tail(plan, iteration).and_then(NativeTailLoop::parent);
    }
    false
}

pub(super) fn scope(
    code: CodePlan<'_>,
    proof: &Proof,
    version: &Version,
    blocks: &BTreeMap<u32, Block>,
) -> Option<Scope> {
    let [left, right] = version.captures.as_slice() else {
        return None;
    };
    if left.array != right.array || proof.arrays[left.array].writable {
        return None;
    }
    let CodePlan::Program(program) = code else {
        return None;
    };
    let plan = &program.metadata().plan;
    let phase = proof.phase.as_ref()?;
    let mut current = tail(plan, plan.blocks()[version.header].iteration()?)?.parent();
    // Confine the extra state to a nested recurrence that repeatedly enters a
    // fixed update region. Wider effect sets and growing storage keep their
    // existing matcher paths. The equality view itself stays immutable.
    while let Some(iteration) = current {
        let writer = tail(plan, iteration)?;
        let mut written = BTreeSet::new();
        for (site, action) in &proof.actions {
            match action {
                Operation::Write { array, .. } if inside(plan, *site, iteration) => {
                    written.insert(*array);
                }
                _ if action.grows() && inside(plan, *site, iteration) => return None,
                _ => {}
            }
        }
        if written.is_empty() {
            current = writer.parent();
            continue;
        }
        if written.len() != 1 {
            return None;
        }
        let enclosing = writer.parent()?;
        let outer = tail(plan, enclosing)?;
        if phase.loops.first()?.iteration == enclosing
            || !phase.loops.iter().any(|entry| entry.iteration == enclosing)
        {
            return None;
        }
        return Some(Scope {
            block: *blocks.get(&outer.header())?,
            header: outer.header() as usize,
            array: left.array,
        });
    }
    None
}

#[derive(Clone, Copy)]
pub(super) struct State {
    slot: StackSlot,
}

// Clone a block's argument list for a transparent edge adapter.
fn adapter(function: &mut Function, destination: Block) -> (Block, Vec<BlockArg>) {
    let types = function
        .dfg
        .block_params(destination)
        .iter()
        .map(|&p| function.dfg.value_type(p))
        .collect::<Vec<_>>();
    let block = function.dfg.make_block();
    let arguments = types
        .into_iter()
        .map(|ty| BlockArg::Value(function.dfg.append_block_param(block, ty)))
        .collect();
    function.layout.append_block(block);
    (block, arguments)
}

#[derive(Debug)]
pub(super) enum Extent {
    Fixed(u64),
    Variable,
}

pub(super) fn extent(context: &Context, cert: &Certificate, change: &Change) -> Option<Extent> {
    let function = &context.func;
    if change.limit_exit == cert.header {
        return None;
    }
    let initial = change.initial(function, cert)?;
    let resolve = Resolve {
        context,
        header: cert.header,
    };
    let initial = resolve.value(initial)?;
    Some(
        match (0..=64).find(|&width| resolve.add(change.limit, initial, width)) {
            Some(width) => Extent::Fixed(width),
            None => Extent::Variable,
        },
    )
}

pub(super) fn begin(context: &mut Context, scope: Scope, change: &Change) -> Option<State> {
    let function = &context.func;
    if !function.layout.is_block_inserted(scope.block)
        || !context
            .domtree
            .dominates(scope.block, change.entry, &function.layout)
    {
        return None;
    }
    let incoming = context
        .cfg
        .pred_iter(scope.block)
        .filter(|p| {
            !context
                .domtree
                .dominates(scope.block, p.inst, &function.layout)
        })
        .map(|p| p.inst)
        .collect::<BTreeSet<_>>();
    if incoming.is_empty() {
        return None;
    }
    let f = &mut context.func;
    let slot = f.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 24, 3));
    let (reset, reset_args) = adapter(f, scope.block);
    for inst in incoming {
        f.rewrite_branch_destination(inst, scope.block, reset);
    }
    let mut cursor = FuncCursor::new(f).at_bottom(reset);
    let zero = cursor.ins().iconst(types::I64, 0);
    for offset in [0, 8, 16] {
        cursor.ins().stack_store(zero, slot, offset);
    }
    cursor.ins().jump(scope.block, &reset_args);
    Some(State { slot })
}

impl State {
    pub(super) fn record_words(self, f: &mut Function, cert: &Certificate, change: &Change) {
        // Only the original bound exit has proved every compared word.
        let at = f.dfg.block_params(cert.header)[0];
        let (record, record_args) = adapter(f, change.limit_exit);
        f.rewrite_branch_destination(change.limit_check, change.limit_exit, record);
        let mut cursor = FuncCursor::new(f).at_bottom(record);
        let end = cursor.ins().iadd(change.bases[0], at);
        cursor.ins().stack_store(end, self.slot, 16);
        cursor.ins().jump(change.limit_exit, &record_args);
    }

    pub(super) fn proved_prefix_end(
        self,
        c: &mut FuncCursor<'_>,
        change: &Change,
        initial: Value,
    ) -> Value {
        let start = c.ins().iadd(change.bases[0], initial);
        let end = c.ins().iadd(change.bases[0], change.limit);
        let distance = c.ins().isub(change.bases[1], change.bases[0]);
        let old_start = c.ins().stack_load(types::I64, self.slot, 0);
        let old_distance = c.ins().stack_load(types::I64, self.slot, 8);
        let old_end = c.ins().stack_load(types::I64, self.slot, 16);
        let same_distance = c.ins().icmp(IntCC::Equal, distance, old_distance);
        let starts_inside = c
            .ins()
            .icmp(IntCC::UnsignedGreaterThanOrEqual, start, old_start);
        let ends_inside = c.ins().icmp(IntCC::UnsignedLessThanOrEqual, end, old_end);
        let inside = c.ins().band(starts_inside, ends_inside);
        let covered = c.ins().band(same_distance, inside);
        // The admitted prefix contains at least one readable word. Its
        // one-past-end address and every covering endpoint are nonzero.
        let absent = c.ins().iconst(types::I64, 0);
        c.ins().select(covered, old_end, absent)
    }

    pub(super) fn record_prefix(
        self,
        f: &mut Function,
        change: &Change,
        width: u64,
        proved_end: Value,
    ) {
        let (finish, args) = adapter(f, change.limit_exit);
        let record = f.dfg.make_block();
        f.layout.append_block(record);
        f.rewrite_branch_destination(change.limit_check, change.limit_exit, finish);
        let mut c = FuncCursor::new(f).at_bottom(finish);
        // If an older proof supplied the prefix, leave its full interval live.
        c.ins()
            .brif(proved_end, change.limit_exit, &args, record, &[]);
        c.goto_bottom(record);
        let end = c.ins().iadd(change.bases[0], change.limit);
        let start = c.ins().iadd_imm(end, -(width as i64));
        let distance = c.ins().isub(change.bases[1], change.bases[0]);
        c.ins().stack_store(start, self.slot, 0);
        c.ins().stack_store(distance, self.slot, 8);
        c.ins().stack_store(end, self.slot, 16);
        c.ins().jump(change.limit_exit, &args);
    }

    pub(super) fn advance_forwarded(
        self,
        c: &mut FuncCursor<'_>,
        change: &Change,
        initial: Value,
        proved_end: Value,
    ) -> Value {
        let reuse = c.func.dfg.make_block();
        let lookup = c.func.dfg.make_block();
        let resume = c.func.dfg.make_block();
        let next = c.func.dfg.append_block_param(resume, types::I64);
        for block in [reuse, lookup, resume] {
            c.func.layout.append_block(block);
        }
        c.ins().brif(proved_end, reuse, &[], lookup, &[]);
        c.goto_bottom(reuse);
        // Admission has checked the current limit. The selected edge
        // established the same ordered bases and a covered cursor.
        let left = c.ins().iadd(change.bases[0], initial);
        let available = c.ins().isub(proved_end, left);
        let remaining = c.ins().isub(change.limit, initial);
        let prefix = c.ins().umin(available, remaining);
        let words = c.ins().band_imm(prefix, -8);
        let advanced = c.ins().iadd(initial, words);
        c.ins().jump(resume, &[advanced.into()]);
        c.goto_bottom(lookup);
        let advanced = self.advance(c, change, initial, true);
        c.ins().jump(resume, &[advanced.into()]);
        c.goto_bottom(resume);
        next
    }

    pub(super) fn advance(
        &self,
        cursor: &mut FuncCursor<'_>,
        change: &Change,
        initial: Value,
        keep_prefix: bool,
    ) -> Value {
        let left = cursor.ins().iadd(change.bases[0], initial);
        let right = cursor.ins().iadd(change.bases[1], initial);
        let distance = cursor.ins().isub(right, left);
        let old_left = cursor.ins().stack_load(types::I64, self.slot, 0);
        let old_distance = cursor.ins().stack_load(types::I64, self.slot, 8);
        let old_end = cursor.ins().stack_load(types::I64, self.slot, 16);
        let same_distance = cursor.ins().icmp(IntCC::Equal, distance, old_distance);
        let after_start = cursor
            .ins()
            .icmp(IntCC::UnsignedGreaterThanOrEqual, left, old_left);
        let end_comparison = if keep_prefix {
            IntCC::UnsignedLessThanOrEqual
        } else {
            IntCC::UnsignedLessThan
        };
        let before_end = cursor.ins().icmp(end_comparison, left, old_end);
        let within = cursor.ins().band(after_start, before_end);
        let covered = cursor.ins().band(same_distance, within);
        let available = cursor.ins().isub(old_end, left);
        let remaining = cursor.ins().isub(change.limit, initial);
        let prefix = cursor.ins().umin(available, remaining);
        let words = cursor.ins().band_imm(prefix, -8);
        let zero = cursor.ins().iconst(types::I64, 0);
        let increment = cursor.ins().select(covered, words, zero);
        // A newly checked prefix ends at the current start. When the saved
        // interval reaches this start, retain its earlier bound.
        let start = if keep_prefix {
            cursor.ins().select(covered, old_left, left)
        } else {
            left
        };
        cursor.ins().stack_store(start, self.slot, 0);
        cursor.ins().stack_store(distance, self.slot, 8);
        cursor.ins().stack_store(left, self.slot, 16);
        cursor.ins().iadd(initial, increment)
    }
}
