use crate::moves::{MoveAndScratchResolver, ParallelMoves};
use crate::{cfg::CFGInfo, ion::Stats, Allocation, RegAllocError};
use crate::{ssa::validate_ssa, Edit, Function, MachineEnv, Output, ProgPoint};
use crate::{
    AllocationKind, Block, FxHashMap, Inst, InstPosition, Operand, OperandConstraint, OperandKind,
    OperandPos, PReg, PRegSet, RegClass, SpillSlot, VReg,
};
use alloc::format;
use alloc::{vec, vec::Vec};
use core::convert::TryInto;
use core::fmt;
use core::iter::FromIterator;
use core::ops::{BitAnd, BitOr, Deref, DerefMut, Index, IndexMut, Not};

mod iter;
mod lru;
mod vregset;
use iter::*;
use lru::*;
use vregset::VRegSet;

#[cfg(test)]
mod tests;

#[derive(Debug)]
struct Allocs {
    allocs: Vec<Allocation>,
    /// `inst_alloc_offsets[i]` is the offset into `allocs` for the allocations of
    /// instruction `i`'s operands.
    inst_alloc_offsets: Vec<u32>,
}

impl Allocs {
    fn new<F: Function>(func: &F) -> (Self, u32) {
        let mut allocs = Vec::new();
        let mut inst_alloc_offsets = Vec::with_capacity(func.num_insts());
        let mut max_operand_len = 0;
        let mut no_of_operands = 0;
        for inst in 0..func.num_insts() {
            let operands_len = func.inst_operands(Inst::new(inst)).len() as u32;
            max_operand_len = max_operand_len.max(operands_len);
            inst_alloc_offsets.push(no_of_operands as u32);
            no_of_operands += operands_len;
        }
        allocs.resize(no_of_operands as usize, Allocation::none());
        (
            Self {
                allocs,
                inst_alloc_offsets,
            },
            max_operand_len,
        )
    }
}

impl Index<(usize, usize)> for Allocs {
    type Output = Allocation;

    /// Retrieve the allocation for operand `idx.1` at instruction `idx.0`
    fn index(&self, idx: (usize, usize)) -> &Allocation {
        &self.allocs[self.inst_alloc_offsets[idx.0] as usize + idx.1]
    }
}

impl IndexMut<(usize, usize)> for Allocs {
    fn index_mut(&mut self, idx: (usize, usize)) -> &mut Allocation {
        &mut self.allocs[self.inst_alloc_offsets[idx.0] as usize + idx.1]
    }
}

#[derive(Debug)]
struct Stack<'a, F: Function> {
    num_spillslots: u32,
    func: &'a F,
}

impl<'a, F: Function> Stack<'a, F> {
    fn new(func: &'a F) -> Self {
        Self {
            num_spillslots: 0,
            func,
        }
    }

    /// Allocates a spill slot on the stack for `vreg`
    fn allocstack(&mut self, class: RegClass) -> SpillSlot {
        trace!("Allocating a spillslot for class {class:?}");
        let size: u32 = self.func.spillslot_size(class).try_into().unwrap();
        // Rest of this function was copied verbatim
        // from `Env::allocate_spillslot` in src/ion/spill.rs.
        let mut offset = self.num_spillslots;
        // Align up to `size`.
        debug_assert!(size.is_power_of_two());
        offset = (offset + size - 1) & !(size - 1);
        let slot = if self.func.multi_spillslot_named_by_last_slot() {
            offset + size - 1
        } else {
            offset
        };
        offset += size;
        self.num_spillslots = offset;
        trace!("Allocated slot: {slot}");
        SpillSlot::new(slot as usize)
    }
}

#[derive(Debug)]
pub struct State<'a, F: Function> {
    func: &'a F,
    /// The final output edits.
    edits: Vec<(ProgPoint, Edit)>,
    fixed_stack_slots: PRegSet,
    /// The scratch registers being used in the instruction being
    /// currently processed.
    scratch_regs: PartedByRegClass<Option<PReg>>,
    dedicated_scratch_regs: PartedByRegClass<Option<PReg>>,
    /// The set of registers that can be used for allocation in the
    /// early and late phases of an instruction.
    ///
    /// Allocatable registers that contain no vregs, registers that can be
    /// evicted can be in the set, and fixed stack slots are in this set.
    available_pregs: PartedByOperandPos<PRegSet>,
    /// Number of registers available for allocation for Reg and Any
    /// operands
    num_available_pregs: PartedByExclusiveOperandPos<PartedByRegClass<i16>>,
    /// The current allocations for all virtual registers.
    vreg_allocs: Vec<Allocation>,
    /// Spillslots for all virtual registers.
    /// `vreg_spillslots[i]` is the spillslot for virtual register `i`.
    vreg_spillslots: Vec<SpillSlot>,
    /// `vreg_in_preg[i]` is the virtual register currently in the physical register
    /// with index `i`.
    vreg_in_preg: Vec<VReg>,
    stack: Stack<'a, F>,
    /// Least-recently-used caches for register classes Int, Float, and Vector, respectively.
    lrus: Lrus,
}

impl<'a, F: Function> State<'a, F> {
    fn is_stack(&self, alloc: Allocation) -> bool {
        alloc.is_stack()
            || (alloc.is_reg() && self.fixed_stack_slots.contains(alloc.as_reg().unwrap()))
    }

    fn get_spillslot(&mut self, vreg: VReg) -> SpillSlot {
        if self.vreg_spillslots[vreg.vreg()].is_invalid() {
            self.vreg_spillslots[vreg.vreg()] = self.stack.allocstack(vreg.class());
        }
        self.vreg_spillslots[vreg.vreg()]
    }

    fn evict_vreg_in_preg(
        &mut self,
        inst: Inst,
        preg: PReg,
        pos: InstPosition,
    ) -> Result<(), RegAllocError> {
        trace!("Removing the vreg in preg {} for eviction", preg);
        let evicted_vreg = self.vreg_in_preg[preg.index()];
        trace!("The removed vreg: {}", evicted_vreg);
        debug_assert_ne!(evicted_vreg, VReg::invalid());
        if self.vreg_spillslots[evicted_vreg.vreg()].is_invalid() {
            self.vreg_spillslots[evicted_vreg.vreg()] = self.stack.allocstack(evicted_vreg.class());
        }
        let slot = self.vreg_spillslots[evicted_vreg.vreg()];
        self.vreg_allocs[evicted_vreg.vreg()] = Allocation::stack(slot);
        trace!("Move reason: eviction");
        self.add_move(
            inst,
            self.vreg_allocs[evicted_vreg.vreg()],
            Allocation::reg(preg),
            evicted_vreg.class(),
            pos,
        )
    }

    fn alloc_scratch_reg(
        &mut self,
        inst: Inst,
        class: RegClass,
        pos: InstPosition,
    ) -> Result<(), RegAllocError> {
        let avail_regs =
            self.available_pregs[OperandPos::Late] & self.available_pregs[OperandPos::Early];
        trace!("Checking {avail_regs} for scratch register for {class:?}");
        if let Some(preg) = self.lrus[class].last(avail_regs) {
            if self.vreg_in_preg[preg.index()] != VReg::invalid() {
                self.evict_vreg_in_preg(inst, preg, pos)?;
            }
            self.scratch_regs[class] = Some(preg);
            self.available_pregs[OperandPos::Early].remove(preg);
            self.available_pregs[OperandPos::Late].remove(preg);
            Ok(())
        } else {
            trace!("Can't get a scratch register for {class:?}");
            Err(RegAllocError::TooManyLiveRegs)
        }
    }

    fn add_move(
        &mut self,
        inst: Inst,
        from: Allocation,
        to: Allocation,
        class: RegClass,
        pos: InstPosition,
    ) -> Result<(), RegAllocError> {
        if self.is_stack(from) && self.is_stack(to) {
            if self.scratch_regs[class].is_none() {
                self.alloc_scratch_reg(inst, class, pos)?;
                let dec_clamp_zero = |x: &mut i16| {
                    *x = 0i16.max(*x - 1);
                };
                dec_clamp_zero(&mut self.num_available_pregs[ExclusiveOperandPos::Both][class]);
                dec_clamp_zero(
                    &mut self.num_available_pregs[ExclusiveOperandPos::EarlyOnly][class],
                );
                dec_clamp_zero(&mut self.num_available_pregs[ExclusiveOperandPos::LateOnly][class]);
                trace!(
                    "Recording edit: {:?}",
                    (ProgPoint::new(inst, pos), Edit::Move { from, to }, class)
                );
            }
            trace!("Edit is stack-to-stack. Generating two edits with a scratch register");
            let scratch_reg = self.scratch_regs[class].unwrap();
            let scratch_alloc = Allocation::reg(scratch_reg);
            trace!("Move 1: {scratch_alloc:?} to {to:?}");
            self.edits.push((
                ProgPoint::new(inst, pos),
                Edit::Move {
                    from: scratch_alloc,
                    to,
                },
            ));
            trace!("Move 2: {from:?} to {scratch_alloc:?}");
            self.edits.push((
                ProgPoint::new(inst, pos),
                Edit::Move {
                    from,
                    to: scratch_alloc,
                },
            ));
        } else {
            self.edits
                .push((ProgPoint::new(inst, pos), Edit::Move { from, to }));
        }
        Ok(())
    }

    /// Given that `pred` is a predecessor of `block`, check if `vreg` is defined on `pred`s branch instruction
    /// in a fixed register and if it is, insert an edit to move from that fixed register to `slot` at the beginning of `block`.
    fn move_if_def_pred_branch(
        &mut self,
        block: Block,
        pred: Block,
        vreg: VReg,
        slot: SpillSlot,
    ) -> Result<(), RegAllocError> {
        let pred_last_inst = self.func.block_insns(pred).last();
        let move_from = self.func.inst_operands(pred_last_inst)
            .iter()
            .find_map(|op| if op.kind() == OperandKind::Def && op.vreg() == vreg {
                if self.func.block_preds(block).len() > 1 {
                    panic!("Multiple predecessors when a branch arg/livein is defined on the branch");
                }
                match op.constraint() {
                    OperandConstraint::FixedReg(reg) => {
                        trace!("Vreg {vreg} defined on pred {pred:?} branch");
                        Some(Allocation::reg(reg))
                    },
                    // In these cases, the vreg is defined directly into the block param
                    // spillslot.
                    OperandConstraint::Stack | OperandConstraint::Any => None,
                    constraint => panic!("fastalloc does not support using any-reg or reuse constraints ({}) defined on a branch instruction as a branch arg/livein on the same instruction", constraint),
                }
            } else {
                None
            });
        if let Some(from) = move_from {
            let to = Allocation::stack(slot);
            trace!("Inserting edit to move from {from} to {to}");
            self.add_move(
                self.func.block_insns(block).first(),
                from,
                to,
                vreg.class(),
                InstPosition::Before,
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct PartedByOperandPos<T> {
    items: [T; 2],
}

impl<T: Copy> Copy for PartedByOperandPos<T> {}

impl<T: BitAnd<Output = T> + Copy> BitAnd for PartedByOperandPos<T> {
    type Output = Self;
    fn bitand(self, other: Self) -> Self {
        Self {
            items: [
                self.items[0] & other.items[0],
                self.items[1] & other.items[1],
            ],
        }
    }
}

impl<T: BitOr<Output = T> + Copy> BitOr for PartedByOperandPos<T> {
    type Output = Self;
    fn bitor(self, other: Self) -> Self {
        Self {
            items: [
                self.items[0] | other.items[0],
                self.items[1] | other.items[1],
            ],
        }
    }
}

impl Not for PartedByOperandPos<PRegSet> {
    type Output = Self;
    fn not(self) -> Self {
        Self {
            items: [self.items[0].invert(), self.items[1].invert()],
        }
    }
}

impl<T> Index<OperandPos> for PartedByOperandPos<T> {
    type Output = T;
    fn index(&self, index: OperandPos) -> &Self::Output {
        &self.items[index as usize]
    }
}

impl<T> IndexMut<OperandPos> for PartedByOperandPos<T> {
    fn index_mut(&mut self, index: OperandPos) -> &mut Self::Output {
        &mut self.items[index as usize]
    }
}

impl<T: fmt::Display> fmt::Display for PartedByOperandPos<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{ early: {}, late: {} }}", self.items[0], self.items[1])
    }
}

#[derive(Debug, Clone, Copy)]
enum ExclusiveOperandPos {
    EarlyOnly = 0,
    LateOnly = 1,
    Both = 2,
}

#[derive(Debug, Clone)]
struct PartedByExclusiveOperandPos<T> {
    items: [T; 3],
}

impl<T: PartialEq> PartialEq for PartedByExclusiveOperandPos<T> {
    fn eq(&self, other: &Self) -> bool {
        self.items.eq(&other.items)
    }
}

impl<T> Index<ExclusiveOperandPos> for PartedByExclusiveOperandPos<T> {
    type Output = T;
    fn index(&self, index: ExclusiveOperandPos) -> &Self::Output {
        &self.items[index as usize]
    }
}

impl<T> IndexMut<ExclusiveOperandPos> for PartedByExclusiveOperandPos<T> {
    fn index_mut(&mut self, index: ExclusiveOperandPos) -> &mut Self::Output {
        &mut self.items[index as usize]
    }
}

impl From<Operand> for ExclusiveOperandPos {
    fn from(op: Operand) -> Self {
        match (op.kind(), op.pos()) {
            (OperandKind::Use, OperandPos::Late) | (OperandKind::Def, OperandPos::Early) => {
                ExclusiveOperandPos::Both
            }
            _ if matches!(op.constraint(), OperandConstraint::Reuse(_)) => {
                ExclusiveOperandPos::Both
            }
            (_, OperandPos::Early) => ExclusiveOperandPos::EarlyOnly,
            (_, OperandPos::Late) => ExclusiveOperandPos::LateOnly,
        }
    }
}

impl<'a, F: Function> Deref for Env<'a, F> {
    type Target = State<'a, F>;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<'a, F: Function> DerefMut for Env<'a, F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

#[derive(Debug)]
pub struct Env<'a, F: Function> {
    func: &'a F,

    /// The virtual registers that are currently live.
    live_vregs: VRegSet,
    /// `reused_input_to_reuse_op[i]` is the operand index of the reuse operand
    /// that uses the `i`th operand in the current instruction as its input.
    reused_input_to_reuse_op: Vec<usize>,
    /// Number of operands with any-reg constraints in the current inst
    /// to allocate for
    num_any_reg_ops: PartedByExclusiveOperandPos<PartedByRegClass<i16>>,
    init_num_available_pregs: PartedByRegClass<i16>,
    init_available_pregs: PRegSet,
    allocatable_regs: PRegSet,
    preferred_victim: PartedByRegClass<PReg>,
    vreg_to_live_inst_range: Vec<(ProgPoint, ProgPoint, Allocation)>,

    fixed_stack_slots: PRegSet,

    // Output.
    allocs: Allocs,
    state: State<'a, F>,
    stats: Stats,
    debug_locations: Vec<(u32, ProgPoint, ProgPoint, Allocation)>,
}

impl<'a, F: Function> Env<'a, F> {
    fn new(func: &'a F, env: &'a MachineEnv) -> Self {
        let mut regs = [
            env.preferred_regs_by_class[RegClass::Int as usize].clone(),
            env.preferred_regs_by_class[RegClass::Float as usize].clone(),
            env.preferred_regs_by_class[RegClass::Vector as usize].clone(),
        ];
        regs[0].union_from(env.non_preferred_regs_by_class[RegClass::Int as usize]);
        regs[1].union_from(env.non_preferred_regs_by_class[RegClass::Float as usize]);
        regs[2].union_from(env.non_preferred_regs_by_class[RegClass::Vector as usize]);
        let allocatable_regs = PRegSet::from(env);
        let num_available_pregs: PartedByRegClass<i16> = PartedByRegClass {
            items: [
                (env.preferred_regs_by_class[RegClass::Int as usize].len()
                    + env.non_preferred_regs_by_class[RegClass::Int as usize].len())
                .try_into()
                .unwrap(),
                (env.preferred_regs_by_class[RegClass::Float as usize].len()
                    + env.non_preferred_regs_by_class[RegClass::Float as usize].len())
                .try_into()
                .unwrap(),
                (env.preferred_regs_by_class[RegClass::Vector as usize].len()
                    + env.non_preferred_regs_by_class[RegClass::Vector as usize].len())
                .try_into()
                .unwrap(),
            ],
        };
        let init_available_pregs = {
            let mut regs = allocatable_regs;
            for preg in env.fixed_stack_slots.iter() {
                regs.add(*preg);
            }
            regs
        };
        let dedicated_scratch_regs = PartedByRegClass {
            items: [
                env.scratch_by_class[0],
                env.scratch_by_class[1],
                env.scratch_by_class[2],
            ],
        };
        trace!("{:#?}", env);
        let (allocs, max_operand_len) = Allocs::new(func);
        let fixed_stack_slots = PRegSet::from_iter(env.fixed_stack_slots.iter().cloned());
        Self {
            func,
            allocatable_regs,
            live_vregs: VRegSet::with_capacity(func.num_vregs()),
            fixed_stack_slots,
            vreg_to_live_inst_range: vec![
                (
                    ProgPoint::invalid(),
                    ProgPoint::invalid(),
                    Allocation::none()
                );
                func.num_vregs()
            ],
            preferred_victim: PartedByRegClass {
                items: [
                    regs[0].max_preg().unwrap_or(PReg::invalid()),
                    regs[1].max_preg().unwrap_or(PReg::invalid()),
                    regs[2].max_preg().unwrap_or(PReg::invalid()),
                ],
            },
            reused_input_to_reuse_op: vec![usize::MAX; max_operand_len as usize],
            init_available_pregs,
            init_num_available_pregs: num_available_pregs.clone(),
            num_any_reg_ops: PartedByExclusiveOperandPos {
                items: [
                    PartedByRegClass { items: [0; 3] },
                    PartedByRegClass { items: [0; 3] },
                    PartedByRegClass { items: [0; 3] },
                ],
            },
            allocs,
            state: State {
                func,
                // This guess is based on the sightglass benchmarks:
                // The average number of edits per instruction is 1.
                edits: Vec::with_capacity(func.num_insts()),
                fixed_stack_slots,
                scratch_regs: dedicated_scratch_regs.clone(),
                dedicated_scratch_regs,
                num_available_pregs: PartedByExclusiveOperandPos {
                    items: [
                        num_available_pregs.clone(),
                        num_available_pregs.clone(),
                        num_available_pregs.clone(),
                    ],
                },
                available_pregs: PartedByOperandPos {
                    items: [init_available_pregs, init_available_pregs],
                },
                lrus: Lrus::new(&regs[0], &regs[1], &regs[2]),
                vreg_in_preg: vec![VReg::invalid(); PReg::NUM_INDEX],
                stack: Stack::new(func),
                vreg_allocs: vec![Allocation::none(); func.num_vregs()],
                vreg_spillslots: vec![SpillSlot::invalid(); func.num_vregs()],
            },
            stats: Stats::default(),
            debug_locations: Vec::with_capacity(func.debug_value_labels().len()),
        }
    }

    fn reset_available_pregs_and_scratch_regs(&mut self) {
        trace!("Resetting the available pregs");
        self.available_pregs = PartedByOperandPos {
            items: [self.init_available_pregs, self.init_available_pregs],
        };
        self.scratch_regs = self.dedicated_scratch_regs.clone();
        self.num_available_pregs = PartedByExclusiveOperandPos {
            items: [self.init_num_available_pregs; 3],
        };
        debug_assert_eq!(
            self.num_any_reg_ops,
            PartedByExclusiveOperandPos {
                items: [PartedByRegClass { items: [0; 3] }; 3]
            }
        );
    }

    fn reserve_reg_for_operand(
        &mut self,
        op: Operand,
        op_idx: usize,
        preg: PReg,
    ) -> Result<(), RegAllocError> {
        trace!("Reserving register {preg} for operand {op}");
        let early_avail_pregs = self.available_pregs[OperandPos::Early];
        let late_avail_pregs = self.available_pregs[OperandPos::Late];
        match (op.pos(), op.kind()) {
            (OperandPos::Early, OperandKind::Use) => {
                if op.as_fixed_nonallocatable().is_none() && !early_avail_pregs.contains(preg) {
                    trace!("fixed {preg} for {op} isn't available");
                    return Err(RegAllocError::TooManyLiveRegs);
                }
                self.available_pregs[OperandPos::Early].remove(preg);
                if self.reused_input_to_reuse_op[op_idx] != usize::MAX {
                    if op.as_fixed_nonallocatable().is_none() && !late_avail_pregs.contains(preg) {
                        trace!("fixed {preg} for {op} isn't available");
                        return Err(RegAllocError::TooManyLiveRegs);
                    }
                    self.available_pregs[OperandPos::Late].remove(preg);
                }
            }
            (OperandPos::Late, OperandKind::Def) => {
                if op.as_fixed_nonallocatable().is_none() && !late_avail_pregs.contains(preg) {
                    trace!("fixed {preg} for {op} isn't available");
                    return Err(RegAllocError::TooManyLiveRegs);
                }
                self.available_pregs[OperandPos::Late].remove(preg);
            }
            _ => {
                if op.as_fixed_nonallocatable().is_none()
                    && (!early_avail_pregs.contains(preg) || !late_avail_pregs.contains(preg))
                {
                    trace!("fixed {preg} for {op} isn't available");
                    return Err(RegAllocError::TooManyLiveRegs);
                }
                self.available_pregs[OperandPos::Early].remove(preg);
                self.available_pregs[OperandPos::Late].remove(preg);
            }
        }
        Ok(())
    }

    fn allocd_within_constraint(&self, op: Operand, inst: Inst) -> bool {
        let alloc = self.vreg_allocs[op.vreg().vreg()];
        match op.constraint() {
            OperandConstraint::Any => {
                if let Some(preg) = alloc.as_reg() {
                    let exclusive_pos: ExclusiveOperandPos = op.into();
                    if !self.is_stack(alloc)
                        && self.num_available_pregs[exclusive_pos][op.class()]
                            < self.num_any_reg_ops[exclusive_pos][op.class()]
                    {
                        trace!("Need more registers to cover all any-reg ops. Going to evict {op} from {preg}");
                        return false;
                    }
                    if !self.available_pregs[op.pos()].contains(preg) {
                        // If a register isn't in the available pregs list, then
                        // there are two cases: either it's reserved for a
                        // fixed register constraint or a vreg allocated in the instruction
                        // is already assigned to it.
                        //
                        // For example:
                        // 1. use v0, use v0, use v0
                        //
                        // Say p0 is assigned to v0 during the processing of the first operand.
                        // When the second v0 operand is being processed, v0 will still be in
                        // v0, so it is still allocated within constraints.
                        trace!("The vreg in {preg}: {}", self.vreg_in_preg[preg.index()]);
                        self.vreg_in_preg[preg.index()] == op.vreg() &&
                            // If it's a late operand, it shouldn't be allocated to a
                            // clobber. For example:
                            // use v0 (fixed: p0), late use v0
                            // If p0 is a clobber, then v0 shouldn't be allocated to it.
                            (op.pos() != OperandPos::Late || !self.func.inst_clobbers(inst).contains(preg))
                    } else {
                        true
                    }
                } else {
                    !alloc.is_none()
                }
            }
            OperandConstraint::Reg => {
                if self.is_stack(alloc) {
                    return false;
                }
                if let Some(preg) = alloc.as_reg() {
                    if !self.available_pregs[op.pos()].contains(preg) {
                        trace!("The vreg in {preg}: {}", self.vreg_in_preg[preg.index()]);
                        self.vreg_in_preg[preg.index()] == op.vreg()
                            && (op.pos() != OperandPos::Late
                                || !self.func.inst_clobbers(inst).contains(preg))
                    } else {
                        true
                    }
                } else {
                    false
                }
            }
            // It is possible for an operand to have a fixed register constraint to
            // a clobber.
            OperandConstraint::FixedReg(preg) => alloc.is_reg() && alloc.as_reg().unwrap() == preg,
            OperandConstraint::Reuse(_) => {
                unreachable!()
            }

            OperandConstraint::Stack => self.is_stack(alloc),
            OperandConstraint::Limit(_) => {
                todo!("limit constraints are not yet supported in fastalloc")
            }
        }
    }

    fn freealloc(&mut self, vreg: VReg) {
        trace!("Freeing vreg {}", vreg);
        let alloc = self.vreg_allocs[vreg.vreg()];
        match alloc.kind() {
            AllocationKind::Reg => {
                let preg = alloc.as_reg().unwrap();
                self.vreg_in_preg[preg.index()] = VReg::invalid();
            }
            AllocationKind::Stack => (),
            AllocationKind::None => unreachable!("Attempting to free an unallocated operand!"),
        }
        self.vreg_allocs[vreg.vreg()] = Allocation::none();
        self.live_vregs.remove(vreg.vreg());
        trace!(
            "{} curr alloc is now {}",
            vreg,
            self.vreg_allocs[vreg.vreg()]
        );
    }

    fn select_suitable_reg_in_lru(&self, op: Operand) -> Result<PReg, RegAllocError> {
        let draw_from = match (op.pos(), op.kind()) {
            // No need to consider reuse constraints because they are
            // handled elsewhere
            (OperandPos::Late, OperandKind::Use) | (OperandPos::Early, OperandKind::Def) => {
                self.available_pregs[OperandPos::Late] & self.available_pregs[OperandPos::Early]
            }
            _ => self.available_pregs[op.pos()],
        };
        if draw_from.is_empty(op.class()) {
            trace!("No registers available for {op} in selection");
            return Err(RegAllocError::TooManyLiveRegs);
        }
        let Some(preg) = self.lrus[op.class()].last(draw_from) else {
            trace!(
                "Failed to find an available {:?} register in the LRU for operand {op}",
                op.class()
            );
            return Err(RegAllocError::TooManyLiveRegs);
        };
        Ok(preg)
    }

    /// Allocates a physical register for the operand `op`.
    fn alloc_reg_for_operand(
        &mut self,
        inst: Inst,
        op: Operand,
    ) -> Result<Allocation, RegAllocError> {
        trace!("available regs: {}", self.available_pregs);
        trace!("Int LRU: {:?}", self.lrus[RegClass::Int]);
        trace!("Float LRU: {:?}", self.lrus[RegClass::Float]);
        trace!("Vector LRU: {:?}", self.lrus[RegClass::Vector]);
        trace!("");
        let preg = self.select_suitable_reg_in_lru(op)?;
        if self.vreg_in_preg[preg.index()] != VReg::invalid() {
            self.evict_vreg_in_preg(inst, preg, InstPosition::After)?;
        }
        trace!("The allocated register for vreg {}: {}", op.vreg(), preg);
        self.lrus[op.class()].poke(preg);
        self.available_pregs[op.pos()].remove(preg);
        match (op.pos(), op.kind()) {
            (OperandPos::Late, OperandKind::Use) => {
                self.available_pregs[OperandPos::Early].remove(preg);
            }
            (OperandPos::Early, OperandKind::Def) => {
                self.available_pregs[OperandPos::Late].remove(preg);
            }
            (OperandPos::Late, OperandKind::Def)
                if matches!(op.constraint(), OperandConstraint::Reuse(_)) =>
            {
                self.available_pregs[OperandPos::Early].remove(preg);
            }
            _ => (),
        };
        Ok(Allocation::reg(preg))
    }

    /// Allocates for the operand `op` with index `op_idx` into the
    /// vector of instruction `inst`'s operands.
    fn alloc_operand(
        &mut self,
        inst: Inst,
        op: Operand,
        op_idx: usize,
    ) -> Result<Allocation, RegAllocError> {
        let new_alloc = match op.constraint() {
            OperandConstraint::Any => {
                if (op.kind() == OperandKind::Def
                    && self.vreg_allocs[op.vreg().vreg()] == Allocation::none())
                    // Not safe to alloc register because any-reg operands still
                    // need them
                    || self.num_any_reg_ops[op.into()][op.class()] >= self.num_available_pregs[op.into()][op.class()]
                {
                    // If the def is never used, it can just be put in its spillslot.
                    Allocation::stack(self.get_spillslot(op.vreg()))
                } else {
                    match self.alloc_reg_for_operand(inst, op) {
                        Ok(alloc) => alloc,
                        Err(RegAllocError::TooManyLiveRegs) => {
                            Allocation::stack(self.get_spillslot(op.vreg()))
                        }
                        Err(err) => return Err(err),
                    }
                }
            }
            OperandConstraint::Reg => {
                let alloc = self.alloc_reg_for_operand(inst, op)?;
                self.num_any_reg_ops[op.into()][op.class()] -= 1;
                trace!(
                    "Number of {:?} any-reg ops to allocate now: {}",
                    Into::<ExclusiveOperandPos>::into(op),
                    self.num_any_reg_ops[op.into()]
                );
                alloc
            }
            OperandConstraint::FixedReg(preg) => {
                trace!("The fixed preg: {} for operand {}", preg, op);

                Allocation::reg(preg)
            }
            OperandConstraint::Reuse(_) => {
                // This is handled elsewhere.
                unreachable!();
            }

            OperandConstraint::Stack => Allocation::stack(self.get_spillslot(op.vreg())),
            OperandConstraint::Limit(_) => {
                todo!("limit constraints are not yet supported in fastalloc")
            }
        };
        self.allocs[(inst.index(), op_idx)] = new_alloc;
        Ok(new_alloc)
    }

    /// Allocate operand the `op_idx`th operand `op` in instruction `inst` within its constraint.
    /// Since only fixed register constraints are allowed, `fixed_spillslot` is used when a
    /// fixed stack allocation is needed, like when transferring a stack allocation from a
    /// reuse operand allocation to the reused input.
    fn process_operand_allocation(
        &mut self,
        inst: Inst,
        op: Operand,
        op_idx: usize,
    ) -> Result<(), RegAllocError> {
        if let Some(preg) = op.as_fixed_nonallocatable() {
            self.allocs[(inst.index(), op_idx)] = Allocation::reg(preg);
            trace!(
                "Allocation for instruction {:?} and operand {}: {}",
                inst,
                op,
                self.allocs[(inst.index(), op_idx)]
            );
            return Ok(());
        }
        if !self.allocd_within_constraint(op, inst) {
            trace!(
                "{op} isn't allocated within constraints (the alloc: {}).",
                self.vreg_allocs[op.vreg().vreg()]
            );
            let curr_alloc = self.vreg_allocs[op.vreg().vreg()];
            let new_alloc = self.alloc_operand(inst, op, op_idx)?;
            if curr_alloc.is_none() {
                self.live_vregs.insert(op.vreg());
                self.vreg_to_live_inst_range[op.vreg().vreg()].1 = match (op.pos(), op.kind()) {
                    (OperandPos::Late, OperandKind::Use) | (_, OperandKind::Def) => {
                        // Live range ends just before the early phase of the
                        // next instruction.
                        ProgPoint::before(Inst::new(inst.index() + 1))
                    }
                    (OperandPos::Early, OperandKind::Use) => {
                        // Live range ends just before the late phase of the current instruction.
                        ProgPoint::after(inst)
                    }
                };
                self.vreg_to_live_inst_range[op.vreg().vreg()].2 = new_alloc;

                trace!("Setting vreg_allocs[{op}] to {new_alloc:?}");
                self.vreg_allocs[op.vreg().vreg()] = new_alloc;
                if let Some(preg) = new_alloc.as_reg() {
                    self.vreg_in_preg[preg.index()] = op.vreg();
                }
            }
            // Need to insert a move to propagate flow from the current
            // allocation to the subsequent places where the value was
            // used (in `prev_alloc`, that is).
            else {
                trace!("Move reason: Prev allocation doesn't meet constraints");
                if op.kind() == OperandKind::Def {
                    trace!("Adding edit from {new_alloc:?} to {curr_alloc:?} after inst {inst:?} for {op}");
                    self.add_move(inst, new_alloc, curr_alloc, op.class(), InstPosition::After)?;
                }
                // Edits for use operands are added later to avoid inserting
                // edits out of order.

                if let Some(preg) = new_alloc.as_reg() {
                    // Don't change the allocation.
                    self.vreg_in_preg[preg.index()] = VReg::invalid();
                }
            }
            trace!(
                "Allocation for instruction {:?} and operand {}: {}",
                inst,
                op,
                self.allocs[(inst.index(), op_idx)]
            );
        } else {
            trace!("{op} is already allocated within constraints");
            self.allocs[(inst.index(), op_idx)] = self.vreg_allocs[op.vreg().vreg()];
            if op.constraint() == OperandConstraint::Reg {
                self.num_any_reg_ops[op.into()][op.class()] -= 1;
                trace!("{op} is already within constraint. Number of reg-only ops that need to be allocated now: {}", self.num_any_reg_ops[op.into()]);
            }
            if let Some(preg) = self.allocs[(inst.index(), op_idx)].as_reg() {
                if self.allocatable_regs.contains(preg) {
                    self.lrus[preg.class()].poke(preg);
                }
                self.available_pregs[op.pos()].remove(preg);
                self.available_pregs[op.pos()].remove(preg);
                match (op.pos(), op.kind()) {
                    (OperandPos::Late, OperandKind::Use) => {
                        self.available_pregs[OperandPos::Early].remove(preg);
                        self.available_pregs[OperandPos::Early].remove(preg);
                    }
                    (OperandPos::Early, OperandKind::Def) => {
                        self.available_pregs[OperandPos::Late].remove(preg);
                        self.available_pregs[OperandPos::Late].remove(preg);
                    }
                    _ => (),
                };
            }
            trace!(
                "Allocation for instruction {:?} and operand {}: {}",
                inst,
                op,
                self.allocs[(inst.index(), op_idx)]
            );
        }
        trace!(
            "Late available regs: {}",
            self.available_pregs[OperandPos::Late]
        );
        trace!(
            "Early available regs: {}",
            self.available_pregs[OperandPos::Early]
        );
        Ok(())
    }

    fn remove_clobbers_from_available_pregs(&mut self, clobbers: PRegSet) {
        trace!("Removing clobbers {clobbers} from late available reg sets");
        let all_but_clobbers = clobbers.invert();
        self.available_pregs[OperandPos::Late].intersect_from(all_but_clobbers);
    }

    /// If instruction `inst` is a branch in `block`,
    /// this function places branch arguments in the spillslots
    /// expected by the destination blocks.
    fn process_branch(&mut self, block: Block, inst: Inst) -> Result<(), RegAllocError> {
        trace!("Processing branch instruction {inst:?} in block {block:?}");

        let mut int_parallel_moves = ParallelMoves::new();
        let mut float_parallel_moves = ParallelMoves::new();
        let mut vec_parallel_moves = ParallelMoves::new();

        for (succ_idx, succ) in self.func.block_succs(block).iter().enumerate() {
            for (pos, vreg) in self
                .func
                .branch_blockparams(block, inst, succ_idx)
                .iter()
                .enumerate()
            {
                if self
                    .func
                    .inst_operands(inst)
                    .iter()
                    .find(|op| op.vreg() == *vreg && op.kind() == OperandKind::Def)
                    .is_some()
                {
                    // vreg is defined in this instruction, so it's dead already.
                    // Can't move it.
                    continue;
                }
                let succ_params = self.func.block_params(*succ);
                let succ_param_vreg = succ_params[pos];
                if self.vreg_spillslots[succ_param_vreg.vreg()].is_invalid() {
                    self.vreg_spillslots[succ_param_vreg.vreg()] =
                        self.stack.allocstack(succ_param_vreg.class());
                }
                if self.vreg_spillslots[vreg.vreg()].is_invalid() {
                    self.vreg_spillslots[vreg.vreg()] = self.stack.allocstack(vreg.class());
                }
                let vreg_spill = Allocation::stack(self.vreg_spillslots[vreg.vreg()]);
                let curr_alloc = self.vreg_allocs[vreg.vreg()];
                if curr_alloc.is_none() {
                    self.live_vregs.insert(*vreg);
                    self.vreg_to_live_inst_range[vreg.vreg()].1 = ProgPoint::before(inst);
                } else if curr_alloc != vreg_spill {
                    self.add_move(
                        inst,
                        vreg_spill,
                        curr_alloc,
                        vreg.class(),
                        InstPosition::Before,
                    )?;
                }
                self.vreg_allocs[vreg.vreg()] = vreg_spill;
                let parallel_moves = match vreg.class() {
                    RegClass::Int => &mut int_parallel_moves,
                    RegClass::Float => &mut float_parallel_moves,
                    RegClass::Vector => &mut vec_parallel_moves,
                };
                let from = Allocation::stack(self.vreg_spillslots[vreg.vreg()]);
                let to = Allocation::stack(self.vreg_spillslots[succ_param_vreg.vreg()]);
                trace!("Recording parallel move from {from} to {to}");
                parallel_moves.add(from, to, Some(*vreg));
            }
        }

        let resolved_int = int_parallel_moves.resolve();
        let resolved_float = float_parallel_moves.resolve();
        let resolved_vec = vec_parallel_moves.resolve();
        let mut scratch_regs = self.scratch_regs.clone();
        let mut num_spillslots = self.stack.num_spillslots;
        let mut avail_regs =
            self.available_pregs[OperandPos::Early] & self.available_pregs[OperandPos::Late];

        trace!("Resolving parallel moves");
        for (resolved, class) in [
            (resolved_int, RegClass::Int),
            (resolved_float, RegClass::Float),
            (resolved_vec, RegClass::Vector),
        ] {
            let scratch_resolver = MoveAndScratchResolver {
                find_free_reg: || {
                    if let Some(reg) = scratch_regs[class] {
                        trace!("Retrieved reg {reg} for scratch resolver");
                        scratch_regs[class] = None;
                        Some(Allocation::reg(reg))
                    } else {
                        let Some(preg) = self.lrus[class].last(avail_regs) else {
                            trace!("Couldn't find any reg for scratch resolver");
                            return None;
                        };
                        avail_regs.remove(preg);
                        trace!("Retrieved reg {preg} for scratch resolver");
                        Some(Allocation::reg(preg))
                    }
                },
                get_stackslot: || {
                    let size: u32 = self.func.spillslot_size(class).try_into().unwrap();
                    let mut offset = num_spillslots;
                    debug_assert!(size.is_power_of_two());
                    offset = (offset + size - 1) & !(size - 1);
                    let slot = if self.func.multi_spillslot_named_by_last_slot() {
                        offset + size - 1
                    } else {
                        offset
                    };
                    offset += size;
                    num_spillslots = offset;
                    trace!("Retrieved slot {slot} for scratch resolver");
                    Allocation::stack(SpillSlot::new(slot as usize))
                },
                is_stack_alloc: |alloc| self.is_stack(alloc),
                borrowed_scratch_reg: self.preferred_victim[class],
            };
            let moves = scratch_resolver.compute(resolved);
            trace!("Resolved {class:?} parallel moves");
            for (from, to, _) in moves.into_iter().rev() {
                self.edits
                    .push((ProgPoint::before(inst), Edit::Move { from, to }))
            }
            self.stack.num_spillslots = num_spillslots;
        }
        trace!("Completed processing branch");
        Ok(())
    }

    fn alloc_def_op(
        &mut self,
        op_idx: usize,
        op: Operand,
        operands: &[Operand],
        block: Block,
        inst: Inst,
    ) -> Result<(), RegAllocError> {
        trace!("Allocating def operand {op}");
        if let OperandConstraint::Reuse(reused_idx) = op.constraint() {
            let reused_op = operands[reused_idx];
            // Alloc as an operand alive in both early and late phases
            let new_reuse_op = Operand::new(
                op.vreg(),
                reused_op.constraint(),
                OperandKind::Def,
                OperandPos::Early,
            );
            trace!("allocating reuse op {op} as {new_reuse_op}");
            self.process_operand_allocation(inst, new_reuse_op, op_idx)?;
        } else if self.func.is_branch(inst) {
            // If the defined vreg is used as a branch arg and it has an
            // any or stack constraint, define it into the block param spillslot
            let mut param_spillslot = None;
            'outer: for (succ_idx, succ) in self.func.block_succs(block).iter().cloned().enumerate()
            {
                for (param_idx, branch_arg_vreg) in self
                    .func
                    .branch_blockparams(block, inst, succ_idx)
                    .iter()
                    .cloned()
                    .enumerate()
                {
                    if op.vreg() == branch_arg_vreg {
                        if matches!(
                            op.constraint(),
                            OperandConstraint::Any | OperandConstraint::Stack
                        ) {
                            let block_param = self.func.block_params(succ)[param_idx];
                            param_spillslot = Some(self.get_spillslot(block_param));
                        }
                        break 'outer;
                    }
                }
            }
            if let Some(param_spillslot) = param_spillslot {
                let spillslot = self.vreg_spillslots[op.vreg().vreg()];
                self.vreg_spillslots[op.vreg().vreg()] = param_spillslot;
                let op = Operand::new(op.vreg(), OperandConstraint::Stack, op.kind(), op.pos());
                self.process_operand_allocation(inst, op, op_idx)?;
                self.vreg_spillslots[op.vreg().vreg()] = spillslot;
            } else {
                self.process_operand_allocation(inst, op, op_idx)?;
            }
        } else {
            self.process_operand_allocation(inst, op, op_idx)?;
        }
        let slot = self.vreg_spillslots[op.vreg().vreg()];
        if slot.is_valid() {
            self.vreg_to_live_inst_range[op.vreg().vreg()].2 = Allocation::stack(slot);
            let curr_alloc = self.vreg_allocs[op.vreg().vreg()];
            let new_alloc = Allocation::stack(self.vreg_spillslots[op.vreg().vreg()]);
            if curr_alloc != new_alloc {
                self.add_move(inst, curr_alloc, new_alloc, op.class(), InstPosition::After)?;
            }
        }
        self.vreg_to_live_inst_range[op.vreg().vreg()].0 = ProgPoint::after(inst);
        self.freealloc(op.vreg());
        Ok(())
    }

    fn alloc_use(&mut self, op_idx: usize, op: Operand, inst: Inst) -> Result<(), RegAllocError> {
        trace!("Allocating use op {op}");
        if self.reused_input_to_reuse_op[op_idx] != usize::MAX {
            let reuse_op_idx = self.reused_input_to_reuse_op[op_idx];
            let reuse_op_alloc = self.allocs[(inst.index(), reuse_op_idx)];
            let Some(preg) = reuse_op_alloc.as_reg() else {
                unreachable!();
            };
            let new_reused_input_constraint = OperandConstraint::FixedReg(preg);
            let new_reused_input =
                Operand::new(op.vreg(), new_reused_input_constraint, op.kind(), op.pos());
            trace!("Allocating reused input {op} as {new_reused_input}");
            self.process_operand_allocation(inst, new_reused_input, op_idx)?;
        } else {
            self.process_operand_allocation(inst, op, op_idx)?;
        }
        Ok(())
    }

    fn alloc_inst(&mut self, block: Block, inst: Inst) -> Result<(), RegAllocError> {
        trace!("Allocating instruction {:?}", inst);
        self.reset_available_pregs_and_scratch_regs();
        let operands = Operands::new(self.func.inst_operands(inst));
        let clobbers = self.func.inst_clobbers(inst);
        // Number of registers that can be used for reg-only operands
        // allocated to fixed-reg operands
        let mut num_fixed_regs_allocatable_clobbers = 0u16;
        trace!("init num avail pregs: {:?}", self.num_available_pregs);
        for (op_idx, op) in operands.0.iter().cloned().enumerate() {
            if let OperandConstraint::Reuse(reused_idx) = op.constraint() {
                trace!("Initializing reused_input_to_reuse_op for {op}");
                self.reused_input_to_reuse_op[reused_idx] = op_idx;
                if operands.0[reused_idx].constraint() == OperandConstraint::Reg {
                    trace!(
                        "Counting {op} as an any-reg op that needs a reg in phase {:?}",
                        ExclusiveOperandPos::Both
                    );
                    self.num_any_reg_ops[ExclusiveOperandPos::Both][op.class()] += 1;
                    // When the reg-only operand is encountered, this will be incremented
                    // Subtract by 1 to remove over-count.
                    trace!(
                        "Decreasing num any-reg ops in phase {:?}",
                        ExclusiveOperandPos::EarlyOnly
                    );
                    self.num_any_reg_ops[ExclusiveOperandPos::EarlyOnly][op.class()] -= 1;
                }
            } else if op.constraint() == OperandConstraint::Reg {
                trace!(
                    "Counting {op} as an any-reg op that needs a reg in phase {:?}",
                    Into::<ExclusiveOperandPos>::into(op)
                );
                self.num_any_reg_ops[op.into()][op.class()] += 1;
            };
        }
        let mut seen = PRegSet::empty();
        for (op_idx, op) in operands.fixed() {
            let OperandConstraint::FixedReg(preg) = op.constraint() else {
                unreachable!();
            };
            self.reserve_reg_for_operand(op, op_idx, preg)?;

            if !seen.contains(preg) {
                seen.add(preg);
                if self.allocatable_regs.contains(preg) {
                    self.lrus[preg.class()].poke(preg);
                    self.num_available_pregs[op.into()][op.class()] -= 1;
                    debug_assert!(self.num_available_pregs[op.into()][op.class()] >= 0);
                    if clobbers.contains(preg) {
                        num_fixed_regs_allocatable_clobbers += 1;
                    }
                }
            }
        }
        trace!("avail pregs after fixed: {:?}", self.num_available_pregs);

        self.remove_clobbers_from_available_pregs(clobbers);

        for (_, op) in operands.fixed() {
            let OperandConstraint::FixedReg(preg) = op.constraint() else {
                unreachable!();
            };
            // Eviction has to be done separately to avoid using a fixed register
            // as a scratch register.
            if self.vreg_in_preg[preg.index()] != VReg::invalid()
                && self.vreg_in_preg[preg.index()] != op.vreg()
            {
                trace!(
                    "Evicting {} from fixed register {preg}",
                    self.vreg_in_preg[preg.index()]
                );
                self.evict_vreg_in_preg(inst, preg, InstPosition::After)?;
                self.vreg_in_preg[preg.index()] = VReg::invalid();
            }
        }
        for preg in clobbers {
            if self.vreg_in_preg[preg.index()] != VReg::invalid() {
                trace!(
                    "Evicting {} from clobber {preg}",
                    self.vreg_in_preg[preg.index()]
                );
                self.evict_vreg_in_preg(inst, preg, InstPosition::After)?;
                self.vreg_in_preg[preg.index()] = VReg::invalid();
            }
            if self.allocatable_regs.contains(preg) {
                if num_fixed_regs_allocatable_clobbers == 0 {
                    trace!("Decrementing clobber avail preg");
                    self.num_available_pregs[ExclusiveOperandPos::LateOnly][preg.class()] -= 1;
                    self.num_available_pregs[ExclusiveOperandPos::Both][preg.class()] -= 1;
                    debug_assert!(
                        self.num_available_pregs[ExclusiveOperandPos::LateOnly][preg.class()] >= 0
                    );
                    debug_assert!(
                        self.num_available_pregs[ExclusiveOperandPos::Both][preg.class()] >= 0
                    );
                } else {
                    // Some fixed-reg operands may be clobbers and so the decrement
                    // of the num avail regs has already been done.
                    num_fixed_regs_allocatable_clobbers -= 1;
                }
            }
        }

        trace!(
            "Number of int, float, vector any-reg ops in early-only, respectively: {}",
            self.num_any_reg_ops[ExclusiveOperandPos::EarlyOnly]
        );
        trace!(
            "Number of any-reg ops in late-only: {}",
            self.num_any_reg_ops[ExclusiveOperandPos::LateOnly]
        );
        trace!(
            "Number of any-reg ops in both early and late: {}",
            self.num_any_reg_ops[ExclusiveOperandPos::Both]
        );
        trace!(
            "Number of available pregs for int, float, vector any-reg and any ops: {:?}",
            self.num_available_pregs
        );
        trace!(
            "registers available for early reg-only & any operands: {}",
            self.available_pregs[OperandPos::Early]
        );
        trace!(
            "registers available for late reg-only & any operands: {}",
            self.available_pregs[OperandPos::Late]
        );

        for (op_idx, op) in operands.late() {
            if op.kind() == OperandKind::Def {
                self.alloc_def_op(op_idx, op, operands.0, block, inst)?;
            } else {
                self.alloc_use(op_idx, op, inst)?;
            }
        }
        for (op_idx, op) in operands.early() {
            trace!("Allocating use operand {op}");
            if op.kind() == OperandKind::Use {
                self.alloc_use(op_idx, op, inst)?;
            } else {
                self.alloc_def_op(op_idx, op, operands.0, block, inst)?;
            }
        }

        for (op_idx, op) in operands.use_ops() {
            if op.as_fixed_nonallocatable().is_some() {
                continue;
            }
            let curr_alloc = self.vreg_allocs[op.vreg().vreg()];
            let new_alloc = self.allocs[(inst.index(), op_idx)];
            if curr_alloc != new_alloc {
                trace!("Adding edit from {curr_alloc:?} to {new_alloc:?} before inst {inst:?} for {op}");
                self.add_move(
                    inst,
                    curr_alloc,
                    new_alloc,
                    op.class(),
                    InstPosition::Before,
                )?;
            }
        }
        if self.func.is_branch(inst) {
            self.process_branch(block, inst)?;
        }
        for entry in self.reused_input_to_reuse_op.iter_mut() {
            *entry = usize::MAX;
        }
        if trace_enabled!() {
            self.log_post_inst_processing_state(inst);
        }
        Ok(())
    }

    /// At the beginning of every block, all virtual registers that are
    /// livein are expected to be in their respective spillslots.
    /// This function sets the current allocations of livein registers
    /// to their spillslots and inserts the edits to flow livein values to
    /// the allocations where they are expected to be before the first
    /// instruction.
    fn reload_at_begin(&mut self, block: Block) -> Result<(), RegAllocError> {
        trace!(
            "Reloading live registers at the beginning of block {:?}",
            block
        );
        trace!(
            "Live registers at the beginning of block {:?}: {:?}",
            block,
            self.live_vregs
        );
        trace!(
            "Block params at block {:?} beginning: {:?}",
            block,
            self.func.block_params(block)
        );
        self.reset_available_pregs_and_scratch_regs();
        let first_inst = self.func.block_insns(block).first();
        // We need to check for the registers that are still live.
        // These registers are either livein or block params
        // Liveins should be stack-allocated and block params should be freed.
        for vreg in self.func.block_params(block).iter().cloned() {
            trace!("Processing {}", vreg);
            if self.vreg_allocs[vreg.vreg()] == Allocation::none() {
                // If this block param was never used, its allocation will
                // be none at this point.
                continue;
            }
            // The allocation where the vreg is expected to be before
            // the first instruction.
            let prev_alloc = self.vreg_allocs[vreg.vreg()];
            let slot = Allocation::stack(self.get_spillslot(vreg));
            self.vreg_to_live_inst_range[vreg.vreg()].2 = slot;
            self.vreg_to_live_inst_range[vreg.vreg()].0 = ProgPoint::before(first_inst);
            trace!("{} is a block param. Freeing it", vreg);
            // A block's block param is not live before the block.
            // And `vreg_allocs[i]` of a virtual register i is none for
            // dead vregs.
            self.freealloc(vreg);
            if slot == prev_alloc {
                // No need to do any movements if the spillslot is where the vreg is expected to be.
                trace!(
                    "No need to reload {} because it's already in its expected allocation",
                    vreg
                );
                continue;
            }
            trace!(
                "Move reason: reload {} at begin - move from its spillslot",
                vreg
            );
            self.state.add_move(
                self.func.block_insns(block).first(),
                slot,
                prev_alloc,
                vreg.class(),
                InstPosition::Before,
            )?;
        }
        for vreg in self.live_vregs.iter() {
            trace!("Processing {}", vreg);
            trace!(
                "{} is not a block param. It's a liveout vreg from some predecessor",
                vreg
            );
            // The allocation where the vreg is expected to be before
            // the first instruction.
            let prev_alloc = self.vreg_allocs[vreg.vreg()];
            let slot = Allocation::stack(self.state.get_spillslot(vreg));
            trace!("Setting {}'s current allocation to its spillslot", vreg);
            self.state.vreg_allocs[vreg.vreg()] = slot;
            if let Some(preg) = prev_alloc.as_reg() {
                trace!("{} was in {}. Removing it", preg, vreg);
                // Nothing is in that preg anymore.
                self.state.vreg_in_preg[preg.index()] = VReg::invalid();
            }
            if slot == prev_alloc {
                // No need to do any movements if the spillslot is where the vreg is expected to be.
                trace!(
                    "No need to reload {} because it's already in its expected allocation",
                    vreg
                );
                continue;
            }
            trace!(
                "Move reason: reload {} at begin - move from its spillslot",
                vreg
            );
            self.state.add_move(
                first_inst,
                slot,
                prev_alloc,
                vreg.class(),
                InstPosition::Before,
            )?;
        }
        // Reset this, in case a fixed reg used by a branch arg defined on the branch
        // is used as a scratch reg in the previous loop.
        self.state.scratch_regs = self.state.dedicated_scratch_regs.clone();

        let get_succ_idx_of_pred = |pred, func: &F| {
            for (idx, pred_succ) in func.block_succs(pred).iter().enumerate() {
                if *pred_succ == block {
                    return idx;
                }
            }
            unreachable!(
                "{:?} was not found in the successor list of its predecessor {:?}",
                block, pred
            );
        };
        trace!(
            "Checking for predecessor branch args/livein vregs defined in the branch with fixed-reg constraint"
        );
        for (param_idx, block_param) in self.func.block_params(block).iter().cloned().enumerate() {
            // Block param is never used. Don't bother.
            if self.state.vreg_spillslots[block_param.vreg()].is_invalid() {
                continue;
            }
            for pred in self.func.block_preds(block).iter().cloned() {
                let pred_last_inst = self.func.block_insns(pred).last();
                let curr_block_succ_idx = get_succ_idx_of_pred(pred, self.func);
                let branch_arg_for_param =
                    self.func
                        .branch_blockparams(pred, pred_last_inst, curr_block_succ_idx)[param_idx];
                // If the branch arg is defined in the branch instruction, the move will have to be done
                // here, instead of at the end of the predecessor block.
                self.state.move_if_def_pred_branch(
                    block,
                    pred,
                    branch_arg_for_param,
                    self.state.vreg_spillslots[block_param.vreg()],
                )?;
            }
        }
        for vreg in self.live_vregs.iter() {
            for pred in self.func.block_preds(block).iter().cloned() {
                let slot = self.state.vreg_spillslots[vreg.vreg()];
                // Move from the reg into its spillslot if it's defined in a predecessor's
                // branch instruction.
                self.state
                    .move_if_def_pred_branch(block, pred, vreg, slot)?;
            }
        }
        if trace_enabled!() {
            self.log_post_reload_at_begin_state(block);
        }
        Ok(())
    }

    fn log_post_reload_at_begin_state(&self, block: Block) {
        trace!("");
        trace!("State after instruction reload_at_begin of {:?}", block);
        let mut map = FxHashMap::default();
        for (vreg_idx, alloc) in self.state.vreg_allocs.iter().enumerate() {
            if *alloc != Allocation::none() {
                map.insert(format!("vreg{vreg_idx}"), alloc);
            }
        }
        trace!("vreg_allocs: {:?}", map);
        let mut map = FxHashMap::default();
        for i in 0..self.state.vreg_in_preg.len() {
            if self.state.vreg_in_preg[i] != VReg::invalid() {
                map.insert(PReg::from_index(i), self.state.vreg_in_preg[i]);
            }
        }
        trace!("vreg_in_preg: {:?}", map);
        trace!("Int LRU: {:?}", self.state.lrus[RegClass::Int]);
        trace!("Float LRU: {:?}", self.state.lrus[RegClass::Float]);
        trace!("Vector LRU: {:?}", self.state.lrus[RegClass::Vector]);
    }

    fn log_post_inst_processing_state(&self, inst: Inst) {
        trace!("");
        trace!("State after instruction {:?}", inst);
        let mut map = FxHashMap::default();
        for (vreg_idx, alloc) in self.state.vreg_allocs.iter().enumerate() {
            if *alloc != Allocation::none() {
                map.insert(format!("vreg{vreg_idx}"), alloc);
            }
        }
        trace!("vreg_allocs: {:?}", map);
        let mut v = Vec::new();
        for i in 0..self.state.vreg_in_preg.len() {
            if self.state.vreg_in_preg[i] != VReg::invalid() {
                v.push(format!(
                    "{}: {}, ",
                    PReg::from_index(i),
                    self.state.vreg_in_preg[i]
                ));
            }
        }
        trace!("vreg_in_preg: {:?}", v);
        trace!("Int LRU: {:?}", self.state.lrus[RegClass::Int]);
        trace!("Float LRU: {:?}", self.state.lrus[RegClass::Float]);
        trace!("Vector LRU: {:?}", self.state.lrus[RegClass::Vector]);
        trace!(
            "Number of any-reg early-only to allocate for: {}",
            self.num_any_reg_ops[ExclusiveOperandPos::EarlyOnly]
        );
        trace!(
            "Number of any-reg late-only to allocate for: {}",
            self.num_any_reg_ops[ExclusiveOperandPos::LateOnly]
        );
        trace!(
            "Number of any-reg early & late to allocate for: {}",
            self.num_any_reg_ops[ExclusiveOperandPos::Both]
        );
        trace!("");
    }

    fn alloc_block(&mut self, block: Block) -> Result<(), RegAllocError> {
        trace!("{:?} start", block);
        for inst in self.func.block_insns(block).iter().rev() {
            self.alloc_inst(block, inst)?;
        }
        self.reload_at_begin(block)?;
        trace!("{:?} end\n", block);
        Ok(())
    }

    fn build_debug_info(&mut self) {
        trace!("Building debug location info");
        for &(vreg, start, end, label) in self.func.debug_value_labels() {
            let (point_start, point_end, alloc) = self.vreg_to_live_inst_range[vreg.vreg()];
            if point_start.inst() <= start && end <= point_end.inst().next() {
                self.debug_locations
                    .push((label, point_start, point_end, alloc));
            }
        }
        self.debug_locations.sort_by_key(|loc| loc.0);
    }

    fn run(&mut self) -> Result<(), RegAllocError> {
        debug_assert_eq!(self.func.entry_block().index(), 0);
        for block in (0..self.func.num_blocks()).rev() {
            self.alloc_block(Block::new(block))?;
        }
        self.state.edits.reverse();
        self.build_debug_info();
        Ok(())
    }
}

fn log_function<F: Function>(func: &F) {
    trace!("Processing a new function");
    for block in 0..func.num_blocks() {
        let block = Block::new(block);
        trace!(
            "Block {:?}. preds: {:?}. succs: {:?}, params: {:?}",
            block,
            func.block_preds(block),
            func.block_succs(block),
            func.block_params(block)
        );
        for inst in func.block_insns(block).iter() {
            let clobbers = func.inst_clobbers(inst);
            trace!(
                "inst{:?}: {:?}. Clobbers: {}",
                inst.index(),
                func.inst_operands(inst),
                clobbers
            );
            if func.is_branch(inst) {
                trace!("Block args: ");
                for (succ_idx, _succ) in func.block_succs(block).iter().enumerate() {
                    trace!(" {:?}", func.branch_blockparams(block, inst, succ_idx));
                }
            }
        }
        trace!("");
    }
}

fn log_output<'a, F: Function>(env: &Env<'a, F>) {
    trace!("Done!");
    let mut v = Vec::new();
    for i in 0..env.func.num_vregs() {
        if env.state.vreg_spillslots[i].is_valid() {
            v.push((
                format!("{}", VReg::new(i, RegClass::Int)),
                format!("{}", Allocation::stack(env.state.vreg_spillslots[i])),
            ));
        }
    }
    trace!("VReg spillslots: {:?}", v);
    trace!("Final edits: {:?}", env.state.edits);
}

pub fn run<F: Function>(
    func: &F,
    mach_env: &MachineEnv,
    verbose_log: bool,
    enable_ssa_checker: bool,
) -> Result<Output, RegAllocError> {
    if enable_ssa_checker {
        validate_ssa(func, &CFGInfo::new(func)?)?;
    }

    if trace_enabled!() || verbose_log {
        log_function(func);
    }

    let mut env = Env::new(func, mach_env);
    env.run()?;

    if trace_enabled!() || verbose_log {
        log_output(&env);
    }

    Ok(Output {
        edits: env.state.edits,
        allocs: env.allocs.allocs,
        inst_alloc_offsets: env.allocs.inst_alloc_offsets,
        num_spillslots: env.state.stack.num_spillslots as usize,
        debug_locations: env.debug_locations,
        stats: env.stats,
    })
}
