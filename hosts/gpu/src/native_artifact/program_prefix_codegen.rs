//! Pause only at the original header, before admitting another Tail traversal.
//! All carrier tests and state transitions are emitted instructions, not Rust.
use super::super::{
    NativeCompileError,
    abi::STATUS_ABI,
    compiler::{guard, offset},
    program_abi, program_borrowed_layout,
    program_hooks::Hooks,
    program_prefix_layout::Region,
    program_tail_layout,
};
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

pub(in crate::native_artifact) fn resume(
    b: &mut FunctionBuilder,
    locals: Value,
    region: &Region,
    borrowed: &program_borrowed_layout::Layout,
    closed: &program_tail_layout::Layout,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    let site = region.tail.header() as usize;
    super::borrowed_codegen::resume(
        b,
        locals,
        &borrowed.regions[region.borrowed],
        site,
        hooks,
        error,
    )?;
    super::tail_codegen::resume(b, locals, closed, site, hooks, error)
}

pub(in crate::native_artifact) fn gate(
    b: &mut FunctionBuilder,
    frame: Value,
    locals: Value,
    phase: Value,
    region: &Region,
    borrowed: &program_borrowed_layout::Layout,
    closed: &program_tail_layout::Layout,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    let prefix = b.create_block();
    let evaluate = b.create_block();
    let proceed = b.create_block();
    let prefix_phase = b
        .ins()
        .icmp_imm(IntCC::Equal, phase, program_abi::PHASE_PREFIX as i64);
    b.ins().brif(prefix_phase, prefix, &[], proceed, &[]);
    b.switch_to_block(prefix);
    let eof = b.ins().load(
        types::I64,
        MemFlags::new(),
        frame,
        core::mem::offset_of!(program_abi::Frame, input_eof) as i32,
    );
    let finished = b.ins().icmp_imm(IntCC::Equal, eof, 1);
    b.ins().brif(finished, proceed, &[], evaluate, &[]);
    b.switch_to_block(evaluate);
    let available = b.ins().load(
        types::I64,
        MemFlags::new(),
        frame,
        core::mem::offset_of!(program_abi::Frame, available_lines) as i32,
    );
    let ordinal = b
        .ins()
        .load(types::I64, MemFlags::new(), locals, offset(region.carrier)?);
    let mut known = b.ins().iconst(types::I8, 0);
    let mut pending = b.ins().iconst(types::I8, 0);
    for state in region.frontier.states() {
        let matches = b
            .ins()
            .icmp_imm(IntCC::Equal, ordinal, i64::from(state.ordinal()));
        known = b.ins().bor(known, matches);
        let waiting = if let Some(field) = state.index_field() {
            let index = b.ins().load(
                types::I64,
                MemFlags::new(),
                locals,
                offset(region.carrier + field)?,
            );
            let absent = b.ins().icmp(IntCC::UnsignedGreaterThan, index, available);
            b.ins().band(matches, absent)
        } else {
            // Even an already returned range must await the SAME capture's
            // EOF/error validation before finish/join/output can be reached.
            matches
        };
        pending = b.ins().bor(pending, waiting);
    }
    guard(b, known, error, STATUS_ABI);
    let pause = b.create_block();
    b.ins().brif(pending, pause, &[], proceed, &[]);
    b.switch_to_block(pause);
    // Retention is needed only when a chunk is exhausted. Keep these loads,
    // min/range calculations and selections out of the per-line hot path.
    let zero = b.ins().iconst(types::I64, 0);
    let one = b.ins().iconst(types::I64, 1);
    let completed_end = b.ins().iadd(available, one);
    let mut keep_first = zero;
    let mut keep_end = zero;
    let mut keep_closed = zero;
    for state in region.frontier.states() {
        let matches = b
            .ins()
            .icmp_imm(IntCC::Equal, ordinal, i64::from(state.ordinal()));
        // The SAME emitted carrier determines storage liveness. The runtime
        // receives coordinates only; it never interprets a Mech state/tag.
        let (first, end, closed) = if let Some((first, end)) = state.returned_range() {
            let first = b.ins().load(
                types::I64,
                MemFlags::new(),
                locals,
                offset(region.carrier + first)?,
            );
            let end = b.ins().load(
                types::I64,
                MemFlags::new(),
                locals,
                offset(region.carrier + end)?,
            );
            let nonempty = b.ins().icmp(IntCC::UnsignedLessThan, first, end);
            // Both unordered and equal returned endpoints mean an empty
            // result under the original nonempty guard. Never revive bytes.
            (
                b.ins().select(nonempty, first, one),
                b.ins().select(nonempty, end, one),
                one,
            )
        } else {
            let mut first = None;
            for field in state.keep_fields() {
                let word = b.ins().load(
                    types::I64,
                    MemFlags::new(),
                    locals,
                    offset(region.carrier + field)?,
                );
                first = Some(first.map_or(word, |prior| b.ins().umin(prior, word)));
            }
            (
                b.ins().umin(
                    first.ok_or_else(|| {
                        super::super::compiler::backend("empty retention frontier")
                    })?,
                    completed_end,
                ),
                zero,
                zero,
            )
        };
        keep_first = b.ins().select(matches, first, keep_first);
        keep_end = b.ins().select(matches, end, keep_end);
        keep_closed = b.ins().select(matches, closed, keep_closed);
    }
    let site = region.tail.header() as usize;
    super::tail_codegen::pause(b, locals, closed, site, hooks, error)?;
    let layout = &borrowed.regions[region.borrowed];
    // Clear all raw pair lanes, including derived Strings from the last
    // traversal, BEFORE the helper drops their actual immutable owner loans.
    for word in layout.ready..layout.end {
        b.ins().store(MemFlags::new(), zero, locals, offset(word)?);
    }
    hooks.call(
        b,
        site,
        program_abi::PREFIX_PAUSE,
        [keep_first, keep_end, keep_closed],
        error,
    );
    let status = b
        .ins()
        .iconst(types::I64, program_abi::STATUS_INPUT_PENDING as i64);
    b.ins().return_(&[status]);
    b.switch_to_block(proceed);
    Ok(())
}
