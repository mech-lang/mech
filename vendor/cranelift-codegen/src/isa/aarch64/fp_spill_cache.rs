//! Store private integer spill cells in otherwise unused caller-saved FP registers.
use super::inst::Inst;
use crate::isa::CallConv;
use crate::machinst::{CallType, MachInst, VCode};
use alloc::{format, string::String, vec, vec::Vec};
use core::fmt::Write;
use regalloc2::{Allocation, Block, Edit, Function, Output, PReg, PRegSet, ProgPoint, RegClass};

#[derive(Debug)]
pub(super) struct Assignment {
    pub slot: usize,
    pub register: PReg,
    pub weighted_reads: u64,
    pub weighted_writes: u64,
    pub mirrored: bool,
}

#[derive(Debug, Default)]
pub(super) struct Plan {
    pub rejected: Option<&'static str>,
    pub spill_area_offset: u32,
    pub unused_registers: Vec<PReg>,
    pub assignments: Vec<Assignment>,
    pub rewritten_moves: usize,
    pub mirrored_writes: usize,
    pub retained_pair_reads: usize,
}

impl Plan {
    pub fn rejected(reason: &'static str) -> Self {
        Self {
            rejected: Some(reason),
            ..Self::default()
        }
    }

    pub fn report(&self) -> String {
        let mut text = format!(
            "rejected={:?}\nspill_area_offset={}\nunused_registers={:?}\nrewritten_moves={}\nmirrored_writes={}\nretained_pair_reads={}\n",
            self.rejected,
            self.spill_area_offset,
            self.unused_registers,
            self.rewritten_moves,
            self.mirrored_writes,
            self.retained_pair_reads
        );
        for assignment in &self.assignments {
            writeln!(
                text,
                "slot={} offset={} register={:?} reads={} writes={} mirrored={}",
                assignment.slot,
                u64::from(self.spill_area_offset) + assignment.slot as u64 * 8,
                assignment.register,
                assignment.weighted_reads,
                assignment.weighted_writes,
                assignment.mirrored
            )
            .unwrap();
        }
        text
    }
}

#[derive(Clone, Default)]
struct Cell {
    blocked: bool,
    reads: u64,
    writes: u64,
}

fn block_cells(cells: &mut [Cell], start: usize, size: usize) -> Option<()> {
    let end = start.checked_add(size)?;
    for cell in cells.get_mut(start..end)? {
        cell.blocked = true;
    }
    Some(())
}

fn integer_reload(edit: &Edit) -> Option<(usize, PReg)> {
    let Edit::Move { from, to } = edit;
    let register = to.as_reg()?;
    (register.class() == RegClass::Int).then_some((from.as_stack()?.index(), register))
}

fn integer_store(edit: &Edit) -> bool {
    let Edit::Move { from, to } = edit;
    to.as_stack().is_some()
        && from
            .as_reg()
            .is_some_and(|reg| reg.class() == RegClass::Int)
}

// Preserve adjacent integer reloads at the same program point. They retain
// their original ordering and addresses, allowing the emitter to keep LDP.
// Conservative matches outside its immediate range only miss an opportunity.
fn paired_reloads(edits: &[(ProgPoint, Edit)]) -> Vec<bool> {
    let mut paired = vec![false; edits.len()];
    let mut index = 0;
    while index + 1 < edits.len() {
        if edits[index].0 == edits[index + 1].0
            && let Some((a, ra)) = integer_reload(&edits[index].1)
            && let Some((b, rb)) = integer_reload(&edits[index + 1].1)
            && a.abs_diff(b) == 1
            && ra != rb
        {
            paired[index] = true;
            paired[index + 1] = true;
            index += 2;
        } else {
            index += 1;
        }
    }
    paired
}

pub(super) fn apply(code: &VCode<Inst>, output: &mut Output, limit: u8, keep_pairs: bool) -> Plan {
    if code.abi.call_conv() != CallConv::AppleAarch64 {
        return Plan::rejected("unaudited calling convention");
    }
    if output.num_spillslots == 0 || code.spillslot_size(RegClass::Int) != 1 {
        return Plan::rejected("no one-word integer spill cells");
    }
    if !code.debug_value_labels().is_empty() || !output.debug_locations.is_empty() {
        return Plan::rejected("debug value locations");
    }
    let mut used = PRegSet::default();
    let mut cells = vec![Cell::default(); output.num_spillslots];
    for block_index in 0..code.num_blocks() {
        for inst in code.block_insns(Block::new(block_index)).iter() {
            if code[inst].call_type() != CallType::None || code[inst].is_safepoint() {
                return Plan::rejected("call or safepoint");
            }
            if code.get_user_stack_map(inst).is_some() {
                return Plan::rejected("user stack map");
            }
            used.union_from(code.inst_clobbers(inst));
            for (operand, &allocation) in code
                .inst_operands(inst)
                .iter()
                .zip(output.inst_allocs(inst))
            {
                if let Some(reg) = allocation.as_reg() {
                    used.add(reg);
                }
                if let Some(slot) = allocation.as_stack()
                    && block_cells(
                        &mut cells,
                        slot.index(),
                        code.spillslot_size(operand.class()),
                    )
                    .is_none()
                {
                    return Plan::rejected("invalid direct stack operand");
                }
            }
        }
    }
    // Edits use the allocator's private spill-slot namespace, not source stack
    // slots. Include every existing edit register, including transient moves.
    for (_, Edit::Move { from, to }) in &output.edits {
        for allocation in [from, to] {
            if let Some(reg) = allocation.as_reg() {
                used.add(reg);
            }
        }
        for (memory, other) in [(from, to), (to, from)] {
            if let Some(slot) = memory.as_stack() {
                let Some(reg) = other.as_reg() else {
                    return Plan::rejected("non-register spill transfer");
                };
                let size = code.spillslot_size(reg.class());
                if reg.class() != RegClass::Int || size != 1 {
                    if block_cells(&mut cells, slot.index(), size).is_none() {
                        return Plan::rejected("invalid overlapping spill cell");
                    }
                } else if slot.index() >= cells.len() {
                    return Plan::rejected("invalid integer spill cell");
                }
            }
        }
    }
    // Use only normal allocatable caller-saved FP registers. These are absent
    // from every operand, edit and implicit clobber in this entire leaf body.
    let available = code.abi.machine_env().preferred_regs_by_class[RegClass::Float as usize];
    let unused_registers = available
        .into_iter()
        .filter(|reg| !used.contains(*reg))
        .collect::<Vec<_>>();
    let mut edit_weights = vec![1u64; output.edits.len()];
    for block_index in 0..code.num_blocks() {
        let block = Block::new(block_index);
        let instructions = code.block_insns(block);
        let first = ProgPoint::before(instructions.first());
        let mut index = output.edits.partition_point(|&(point, _)| point < first);
        let weight = u64::from(code.block_frequency(block).unwrap_or(1).max(1));
        while let Some((point, _)) = output.edits.get(index) {
            if point.inst().index() >= instructions.last().index() + 1 {
                break;
            }
            edit_weights[index] = weight;
            index += 1;
        }
    }
    for ((_, Edit::Move { from, to }), weight) in output.edits.iter().zip(edit_weights) {
        if let Some(slot) = from.as_stack() {
            let cell = &mut cells[slot.index()];
            cell.reads = cell.reads.saturating_add(weight);
        }
        if let Some(slot) = to.as_stack() {
            let cell = &mut cells[slot.index()];
            cell.writes = cell.writes.saturating_add(weight);
        }
    }
    let mut ranked = cells
        .iter()
        .enumerate()
        .filter(|(_, cell)| !cell.blocked && cell.reads != 0 && cell.writes != 0)
        .map(|(slot, cell)| (slot, cell.reads.saturating_add(cell.writes)))
        .collect::<Vec<_>>();
    ranked.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let paired = if keep_pairs {
        paired_reloads(&output.edits)
    } else {
        vec![false; output.edits.len()]
    };
    let mut mirrored_cells = vec![false; cells.len()];
    for ((_, edit), &paired) in output.edits.iter().zip(&paired) {
        if paired && let Some((slot, _)) = integer_reload(edit) {
            mirrored_cells[slot] = true;
        }
    }
    let assignments = ranked
        .into_iter()
        .zip(unused_registers.iter().copied())
        .take(usize::from(limit))
        .map(|((slot, _), register)| Assignment {
            slot,
            register,
            weighted_reads: cells[slot].reads,
            weighted_writes: cells[slot].writes,
            mirrored: mirrored_cells[slot],
        })
        .collect::<Vec<_>>();
    let mut registers = vec![None; cells.len()];
    for assignment in &assignments {
        registers[assignment.slot] = Some(assignment.register);
    }
    let mut rewritten_moves = 0;
    let mut mirrored_writes = 0;
    let mut retained_pair_reads = 0;
    if keep_pairs {
        let original = core::mem::take(&mut output.edits);
        let mut index = 0;
        while index < original.len() {
            let point = original[index].0;
            if integer_store(&original[index].1) {
                let mut end = index + 1;
                while end < original.len()
                    && original[end].0 == point
                    && integer_store(&original[end].1)
                {
                    end += 1;
                }
                // These stores change no register. Copy mirrored sources
                // before the run, preserving adjacent stores for STP too.
                // Repeated writes to a cell remain ordered; no read occurs
                // inside this run, and the final register/memory values agree.
                for (_, Edit::Move { from, to }) in &original[index..end] {
                    let slot = to.as_stack().unwrap().index();
                    if mirrored_cells[slot]
                        && let Some(register) = registers[slot]
                    {
                        output.edits.push((
                            point,
                            Edit::Move {
                                from: *from,
                                to: Allocation::reg(register),
                            },
                        ));
                        mirrored_writes += 1;
                        rewritten_moves += 1;
                    }
                }
                for (_, Edit::Move { from, to }) in &original[index..end] {
                    let slot = to.as_stack().unwrap().index();
                    let destination = if !mirrored_cells[slot]
                        && let Some(register) = registers[slot]
                    {
                        rewritten_moves += 1;
                        Allocation::reg(register)
                    } else {
                        *to
                    };
                    output.edits.push((
                        point,
                        Edit::Move {
                            from: *from,
                            to: destination,
                        },
                    ));
                }
                index = end;
            } else {
                let Edit::Move { mut from, to } = original[index].1.clone();
                if let Some(slot) = from.as_stack()
                    && let Some(register) = registers[slot.index()]
                {
                    if paired[index] {
                        retained_pair_reads += 1;
                    } else {
                        from = Allocation::reg(register);
                        rewritten_moves += 1;
                    }
                }
                output.edits.push((point, Edit::Move { from, to }));
                index += 1;
            }
        }
    } else {
        for (_, Edit::Move { from, to }) in &mut output.edits {
            for allocation in [from, to] {
                if let Some(slot) = allocation.as_stack()
                    && let Some(register) = registers[slot.index()]
                {
                    *allocation = Allocation::reg(register);
                    rewritten_moves += 1;
                }
            }
        }
    }
    // Each selected physical cell has one dedicated register for the whole
    // function, including every branch and reuse of the cell. All its writes
    // and reads are rewritten, except retained paired reads whose memory
    // values are mirrored on every write. The frame allocation stays intact.
    Plan {
        rejected: None,
        spill_area_offset: code.abi.spill_area_offset(),
        unused_registers,
        assignments,
        rewritten_moves,
        mirrored_writes,
        retained_pair_reads,
    }
}
