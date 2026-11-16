use super::{Env, InsertMovePrio, InsertedMoves};
use crate::{
    checker::Checker, ssa::validate_ssa, Allocation, Block, Ctx, Function, Inst, InstRange,
    MachineEnv, Operand, PReg, PRegSet, ProgPoint, RegClass, VReg,
};
use alloc::vec::Vec;

const DESTINATION: PReg = PReg::new(1, RegClass::Int);
const SOURCE: PReg = PReg::new(2, RegClass::Int);
const FIRST: VReg = VReg::new(0, RegClass::Int);
const REPLACEMENT: VReg = VReg::new(1, RegClass::Int);
const UPDATED: VReg = VReg::new(2, RegClass::Int);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Route {
    Chain,
    Join,
    Clobber,
    Rewrite,
    Reordered,
    Taken,
    TakenClobber,
    TakenDefinition,
    TakenSpillRewrite,
    TakenSpillEdit,
    TakenRegisterEdit,
}

impl Route {
    fn is_taken(self) -> bool {
        matches!(
            self,
            Self::Taken
                | Self::TakenClobber
                | Self::TakenDefinition
                | Self::TakenSpillRewrite
                | Self::TakenSpillEdit
                | Self::TakenRegisterEdit
        )
    }

    fn replaces_spill(self) -> bool {
        matches!(self, Self::TakenSpillRewrite | Self::TakenSpillEdit)
    }
}

struct Fixture {
    route: Route,
    blocks: [Block; 4],
    alternatives: [Block; 2],
    operands: [Operand; 6],
}

impl Function for Fixture {
    fn num_insts(&self) -> usize {
        9
    }

    fn num_blocks(&self) -> usize {
        4
    }

    fn entry_block(&self) -> Block {
        self.blocks[0]
    }

    fn block_insns(&self, block: Block) -> InstRange {
        let bounds = [0, 3, 5, 7, 9];
        InstRange::new(
            Inst::new(bounds[block.index()]),
            Inst::new(bounds[block.index() + 1]),
        )
    }

    fn block_succs(&self, block: Block) -> &[Block] {
        match block.index() {
            0 if self.route == Route::Reordered => &self.blocks[1..2],
            0 => &self.alternatives,
            1 if self.route == Route::Reordered => &self.blocks[3..],
            1 => &self.blocks[2..3],
            3 if matches!(self.route, Route::Join | Route::Reordered) => &self.blocks[2..3],
            _ => &[],
        }
    }

    fn block_preds(&self, block: Block) -> &[Block] {
        match block.index() {
            1 => &self.blocks[..1],
            2 if self.route == Route::Join => &self.alternatives,
            2 if self.route == Route::Reordered => &self.blocks[3..],
            2 => &self.blocks[1..2],
            3 if self.route == Route::Reordered => &self.blocks[1..2],
            3 => &self.blocks[..1],
            _ => &[],
        }
    }

    fn block_params(&self, _: Block) -> &[VReg] {
        &[]
    }

    fn is_ret(&self, inst: Inst) -> bool {
        inst.index() == 6 || inst.index() == 8 && !self.is_branch(inst)
    }

    fn is_branch(&self, inst: Inst) -> bool {
        matches!(inst.index(), 2 | 4)
            || inst.index() == 8 && matches!(self.route, Route::Join | Route::Reordered)
    }

    fn branch_blockparams(&self, _: Block, _: Inst, _: usize) -> &[VReg] {
        &[]
    }

    fn inst_operands(&self, inst: Inst) -> &[Operand] {
        match inst.index() {
            0 => &self.operands[..1],
            1 => &self.operands[1..2],
            3 if self.route == Route::Rewrite || self.route.is_taken() => &self.operands[2..3],
            5 if self.route == Route::Rewrite || self.route.is_taken() => &self.operands[3..4],
            7 if self.route == Route::TakenDefinition
                || self.route.replaces_spill()
                || self.route == Route::TakenRegisterEdit =>
            {
                &self.operands[4..5]
            }
            8 if self.route.replaces_spill() => &self.operands[5..6],
            8 if self.route.is_taken() => &self.operands[1..2],
            5 => &self.operands[1..2],
            _ => &[],
        }
    }

    fn inst_clobbers(&self, inst: Inst) -> PRegSet {
        match (self.route, inst.index()) {
            (Route::Clobber, 3) | (Route::Join, 7) | (Route::TakenClobber, 7) => {
                PRegSet::empty().with(DESTINATION)
            }
            _ => PRegSet::empty(),
        }
    }

    fn num_vregs(&self) -> usize {
        if self.route.is_taken() {
            3
        } else {
            1 + usize::from(self.route == Route::Rewrite)
        }
    }

    fn spillslot_size(&self, _: RegClass) -> usize {
        1
    }
}

fn resolved_loads(route: Route) -> Vec<usize> {
    let function = Fixture {
        route,
        blocks: [0, 1, 2, 3].map(Block::new),
        alternatives: [1, 3].map(Block::new),
        operands: [
            Operand::any_def(FIRST),
            Operand::reg_fixed_use(FIRST, DESTINATION),
            Operand::any_def(REPLACEMENT),
            Operand::reg_fixed_use(REPLACEMENT, DESTINATION),
            Operand::any_def(UPDATED),
            Operand::reg_fixed_use(UPDATED, DESTINATION),
        ],
    };
    let machine = MachineEnv {
        preferred_regs_by_class: [
            PRegSet::empty().with(DESTINATION).with(SOURCE),
            PRegSet::empty(),
            PRegSet::empty(),
        ],
        non_preferred_regs_by_class: [PRegSet::empty(); 3],
        scratch_by_class: [None; 3],
        fixed_stack_slots: Vec::new(),
    };
    let mut context = Ctx::default();
    context
        .cfginfo
        .init(&function, &mut context.cfginfo_ctx)
        .unwrap();
    validate_ssa(&function, &context.cfginfo).unwrap();
    let mut env = Env::new(&function, &machine, &mut context);
    env.create_pregs_and_vregs();
    let spill = env.allocate_spillslot(1);
    let destination = Allocation::reg(DESTINATION);
    env.set_alloc(Inst::new(0), 0, spill);
    env.set_alloc(Inst::new(1), 0, destination);
    env.set_alloc(Inst::new(5), 0, destination);
    if route == Route::Rewrite || route.is_taken() {
        env.set_alloc(Inst::new(3), 0, spill);
    }
    if route.is_taken() {
        env.set_alloc(Inst::new(8), 0, destination);
        match route {
            Route::TakenDefinition => env.set_alloc(Inst::new(7), 0, destination),
            Route::TakenSpillRewrite => env.set_alloc(Inst::new(7), 0, spill),
            Route::TakenSpillEdit | Route::TakenRegisterEdit => {
                env.set_alloc(Inst::new(7), 0, Allocation::reg(SOURCE));
            }
            _ => {}
        }
    }

    // Feed the real edit resolver a pair of already selected spill reloads.
    // The allocator's checker validates the resulting edits on every edge.
    let mut inserted = InsertedMoves::default();
    inserted.push(
        ProgPoint::before(Inst::new(1)),
        InsertMovePrio::Regular,
        spill,
        destination,
        FIRST,
    );
    let current = if route == Route::Rewrite || route.is_taken() {
        REPLACEMENT
    } else {
        FIRST
    };
    inserted.push(
        ProgPoint::before(Inst::new(5)),
        InsertMovePrio::Regular,
        spill,
        destination,
        current,
    );
    if route.is_taken() {
        let point = ProgPoint::before(Inst::new(8));
        if matches!(route, Route::TakenSpillEdit | Route::TakenRegisterEdit) {
            let target = if route == Route::TakenSpillEdit {
                spill
            } else {
                destination
            };
            inserted.push(
                point,
                InsertMovePrio::Regular,
                Allocation::reg(SOURCE),
                target,
                UPDATED,
            );
        }
        let needed = if route.replaces_spill() {
            UPDATED
        } else {
            FIRST
        };
        inserted.push(
            point,
            InsertMovePrio::MultiFixedRegSecondary,
            spill,
            destination,
            needed,
        );
    }

    let mut edits = env.resolve_inserted_moves(inserted);
    env.output.edits.extend(edits.drain_edits());
    let mut checker = Checker::new(&function, &machine);
    checker.prepare(&env.output);
    checker.run().unwrap();
    env.output
        .edits
        .iter()
        .map(|(point, _)| point.inst().index())
        .collect()
}

#[test]
fn keeps_a_loaded_register_over_two_single_predecessor_edges() {
    assert_eq!(resolved_loads(Route::Chain), [1]);
}

#[test]
fn reloads_after_an_alternate_predecessor_clobbers_the_register() {
    assert_eq!(resolved_loads(Route::Join), [1, 5]);
}

#[test]
fn reloads_after_an_intermediate_instruction_clobbers_the_register() {
    assert_eq!(resolved_loads(Route::Clobber), [1, 5]);
}

#[test]
fn reloads_the_current_value_after_a_definition_reuses_the_spill_slot() {
    assert_eq!(resolved_loads(Route::Rewrite), [1, 5]);
}

#[test]
fn retains_loads_when_instruction_order_cannot_prove_the_chain() {
    assert_eq!(resolved_loads(Route::Reordered), [1, 5]);
}

#[test]
fn taken_branch_keeps_its_own_copy_across_other_arm_edits() {
    let actual = resolved_loads(Route::Taken);
    // The layout visits a different spill definition and reload in the
    // skipped arm. They must not affect the saved state of the taken arm.
    if cfg!(r292_control) {
        assert_eq!(actual, [1, 5, 8]);
    } else {
        assert_eq!(actual, [1, 5]);
    }
}

#[test]
fn selected_arm_clobbers_and_definitions_invalidate_the_copy() {
    for route in [
        Route::TakenClobber,
        Route::TakenDefinition,
        Route::TakenSpillRewrite,
    ] {
        assert_eq!(resolved_loads(route), [1, 5, 8]);
    }
}

#[test]
fn selected_arm_edit_order_invalidates_either_allocation() {
    for route in [Route::TakenSpillEdit, Route::TakenRegisterEdit] {
        assert_eq!(resolved_loads(route), [1, 5, 8, 8]);
    }
}
