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
use mech_engine::native_artifact::NativeMatchAction;

/// Branch around an unfinished count definition. The owner installs all of
/// the proved identical count slots from final source facts before EOF entry;
/// no prefix length or placeholder becomes a language value in the meantime.
pub(in crate::native_artifact) fn count_store(
    b: &mut FunctionBuilder,
    frame: Value,
    phase: Value,
    region: Option<&Region>,
    site: usize,
) -> Option<Block> {
    let region = region?;
    if !region
        .frontier
        .read()
        .count()
        .lines()
        .lengths()
        .contains(&(site as u32))
    {
        return None;
    }
    let pending = count_pending(b, frame, phase);
    let ordinary = b.create_block();
    let joined = b.create_block();
    b.ins().brif(pending, joined, &[], ordinary, &[]);
    b.switch_to_block(ordinary);
    Some(joined)
}

fn count_pending(b: &mut FunctionBuilder, frame: Value, phase: Value) -> Value {
    let prefix = b
        .ins()
        .icmp_imm(IntCC::Equal, phase, program_abi::PHASE_PREFIX as i64);
    let eof = b.ins().load(
        types::I64,
        MemFlags::new(),
        frame,
        core::mem::offset_of!(program_abi::Frame, input_eof) as i32,
    );
    let unfinished = b.ins().icmp_imm(IntCC::Equal, eof, 0);
    b.ins().band(prefix, unfinished)
}

/// The count proof covers every numeric read. Before EOF, identity aliases
/// remain uninitialized and an available index proves only the original
/// `index > final_count` predicate false. All other reads await the EOF edge.
pub(in crate::native_artifact) fn count_action(
    b: &mut FunctionBuilder,
    frame: Value,
    locals: Value,
    phase: Value,
    region: Option<&Region>,
    site: usize,
    action: &NativeMatchAction,
    error: Block,
) -> Result<bool, NativeCompileError> {
    let Some(region) = region else {
        return Ok(false);
    };
    let count = region.frontier.read().count();
    let comparison = count
        .guards()
        .iter()
        .find(|guard| guard.comparison() as usize == site);
    if comparison.is_none() && !count.aliases().contains(&(site as u32)) {
        return Ok(false);
    }
    let pending = count_pending(b, frame, phase);
    let unfinished = b.create_block();
    let ordinary = b.create_block();
    let joined = b.create_block();
    b.ins().brif(pending, unfinished, &[], ordinary, &[]);
    b.switch_to_block(unfinished);
    if let Some(comparison) = comparison {
        let index = super::super::control_compiler::load(b, locals, comparison.index())?;
        let available = b.ins().load(
            types::I64,
            MemFlags::new(),
            frame,
            core::mem::offset_of!(program_abi::Frame, available_lines) as i32,
        );
        // The proved original header must have paused before an unavailable
        // index reaches this site. Check that invariant without loading count.
        let covered = b
            .ins()
            .icmp(IntCC::UnsignedLessThanOrEqual, index, available);
        guard(b, covered, error, STATUS_ABI);
        let false_ = b.ins().iconst(types::I64, 0);
        b.ins().store(
            MemFlags::new(),
            false_,
            locals,
            offset(comparison.predicate().get() as usize)?,
        );
    }
    b.ins().jump(joined, &[]);
    b.switch_to_block(ordinary);
    super::super::control_compiler::emit_scalar_action(b, locals, action, error)?;
    b.ins().jump(joined, &[]);
    b.switch_to_block(joined);
    Ok(true)
}

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

/// End the scanner at its original exit, before that exit is visited. Closed
/// Tail fuel was reconciled by the ordinary exit prologue. Clear every raw
/// source lane before the helper retires the immutable owner loan, so the
/// owning session can release external row loans before final range consumers.
pub(in crate::native_artifact) fn scanned(
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
    let finish = b.create_block();
    let proceed = b.create_block();
    let prefix = b
        .ins()
        .icmp_imm(IntCC::Equal, phase, program_abi::PHASE_PREFIX as i64);
    b.ins().brif(prefix, finish, &[], proceed, &[]);
    b.switch_to_block(finish);
    let eof = b.ins().load(
        types::I64,
        MemFlags::new(),
        frame,
        core::mem::offset_of!(program_abi::Frame, input_eof) as i32,
    );
    let final_input = b.ins().icmp_imm(IntCC::Equal, eof, 1);
    guard(b, final_input, error, STATUS_ABI);
    let zero = b.ins().iconst(types::I64, 0);
    let layout = &borrowed.regions[region.borrowed];
    for word in layout.ready..layout.end {
        b.ins().store(MemFlags::new(), zero, locals, offset(word)?);
    }
    if let Some(ordinal) = closed.sites[region.tail.header() as usize] {
        let words = closed.regions[ordinal].words;
        for word in words..words + 3 {
            b.ins().store(MemFlags::new(), zero, locals, offset(word)?);
        }
    }
    hooks.call(
        b,
        region.tail.exit() as usize,
        program_abi::PREFIX_SCANNED,
        [zero; 3],
        error,
    );
    let status = b
        .ins()
        .iconst(types::I64, program_abi::STATUS_INPUT_SCANNED as i64);
    b.ins().return_(&[status]);
    b.switch_to_block(proceed);
    Ok(())
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
