//! Verify a bounded packed U32 tree recurrence, including its memory effects.
use super::{Boundary, Effect, Path, Word};
use cranelift_codegen::ir::{
    Block, BlockCall, Function, Inst, Opcode as Op, Value, condcodes::IntCC, types,
};

const LAST: Word = Word::Input(24);
const LEN: Word = Word::Input(40);
const LEAF: Word = Word::Cursor(0);
const BRANCH: Word = Word::Cursor(1);
const BUILT: Word = Word::Cursor(2);
const FREQUENCY_MASK: u64 = 0xffff_fc00;
const SYMBOL_MASK: u64 = 1023;

pub(crate) struct Tree {
    pub entry: Inst,
    pub original: BlockCall,
    pub success: Block,
    pub cursors: [Value; 3],
    pub data: Value,
    pub length: Value,
    pub last: Value,
}

fn binary(op: Op, a: Word, b: Word) -> Word {
    Word::binary(op, a, b)
}

fn add(a: Word, n: u64) -> Word {
    binary(Op::Iadd, a, Word::Constant(n))
}

fn mask(a: Word, n: u64) -> Word {
    binary(Op::Band, a, Word::Constant(n))
}

fn compare(kind: IntCC, a: Word, b: Word) -> Word {
    Word::comparison(kind, a, b)
}

fn address(index: Word) -> Word {
    let offset = binary(Op::Ishl, index, Word::Constant(2));
    binary(Op::Iadd, Word::Input(32), offset)
}

fn read(address: &Word) -> Word {
    Word::Read(Box::new(address.clone()))
}

fn store(address: &Word, high: Word, low: Word) -> Effect {
    let value = binary(Op::Bor, high, mask(low, SYMBOL_MASK));
    Effect::Store(address.clone(), Word::LowWord(Box::new(value)))
}

fn cursor_offset(word: &Word) -> Option<(usize, u64)> {
    match word {
        Word::Cursor(which) => Some((*which, 0)),
        Word::Binary(Op::Iadd, base, offset) => {
            let (Word::Cursor(which), Word::Constant(offset)) = (&**base, &**offset) else {
                return None;
            };
            (*offset <= 4).then_some((*which, *offset))
        }
        _ => None,
    }
}

struct Choices {
    leaves: Word,
    branches: Word,
    both_leaves: bool,
    both_branches: bool,
}

impl Choices {
    fn evaluate(&self, test: &Word) -> Option<bool> {
        if test == &self.leaves {
            return Some(self.both_leaves);
        }
        if test == &self.branches {
            return Some(self.both_branches);
        }
        // The native entry admits last < 512 and length >= last + 3.
        // The reachable loop invariants are leaf <= last + 1 and
        // branch <= built < last. These are the exact indexed guards.
        match test {
            Word::Constant(value) if *value <= 1 => Some(*value != 0),
            Word::Compare(IntCC::UnsignedLessThan, index, limit) if **limit == LEN => {
                let (which, offset) = cursor_offset(index)?;
                (offset <= if which == 0 { 1 } else { 2 }).then_some(true)
            }
            Word::Compare(IntCC::UnsignedGreaterThanOrEqual, a, b) => {
                let (first, ahead) = cursor_offset(a)?;
                let (second, behind) = cursor_offset(b)?;
                (first == second && ahead >= behind).then_some(true)
            }
            Word::Binary(Op::Band, a, b) => match (self.evaluate(a), self.evaluate(b)) {
                (Some(false), _) | (_, Some(false)) => Some(false),
                (Some(true), Some(true)) => Some(true),
                _ => None,
            },
            Word::Binary(Op::Bor, a, b) => match (self.evaluate(a), self.evaluate(b)) {
                (Some(true), _) | (_, Some(true)) => Some(true),
                (Some(false), Some(false)) => Some(false),
                _ => None,
            },
            Word::Binary(Op::Bxor, a, b) => Some(self.evaluate(a)? ^ self.evaluate(b)?),
            _ => None,
        }
    }
}

fn packet_loop(path: &Path<'_>, edge: BlockCall, words: [&Word; 3]) -> Option<Block> {
    let lists = &path.f.dfg.value_lists;
    let target = edge.block(lists);
    let args = edge.args(lists).collect::<Vec<_>>();
    if args.len() != words.len()
        || path.f.dfg.block_params(target).len() != words.len()
        || !path
            .f
            .dfg
            .block_params(target)
            .iter()
            .all(|v| path.f.dfg.value_type(*v) == types::I64)
        || !args.into_iter().zip(words).all(|(arg, word)| {
            arg.as_value().and_then(|value| path.word(value)).as_ref() == Some(word)
        })
    {
        return None;
    }
    Some(target)
}

fn visit(
    path: &mut Path<'_>,
    top: Block,
    mut edge: BlockCall,
    routes: &Choices,
    next: [&Word; 3],
    effects: &[Effect],
) -> Option<()> {
    for _ in 0..128 {
        if edge.block(&path.f.dfg.value_lists) == top {
            packet_loop(path, edge, next)?;
            return (path.effects == effects).then_some(());
        }
        let block = path.enter(edge)?;
        edge = match path.step(block)? {
            Boundary::Jump(_, target) => target,
            Boundary::Branch(condition, edges) => edges[usize::from(!routes.evaluate(&condition)?)],
            Boundary::Return => return None,
        };
    }
    None
}

pub(crate) fn prove(f: &Function) -> Option<Tree> {
    let root = f.layout.entry_block()?;
    let [heap, packet, output] = f.dfg.block_params(root) else {
        return None;
    };
    if ![*heap, *packet, *output]
        .iter()
        .all(|v| f.dfg.value_type(*v) == types::I64)
        || f.signature.returns.len() != 2
        || f.signature
            .returns
            .iter()
            .any(|r| r.value_type != types::I64)
    {
        return None;
    }
    let mut path = Path {
        f,
        words: [
            (*heap, Word::Heap),
            (*packet, Word::InputPacket),
            (*output, Word::OutputPacket),
        ]
        .into(),
        effects: Vec::new(),
        input_bytes: 80,
    };
    let Boundary::Jump(entry, original) = path.step(root)? else {
        return None;
    };
    let top = packet_loop(
        &path,
        original,
        [&Word::Input(0), &Word::Input(8), &Word::Input(16)],
    )?;
    let [leaf, branch, built] = f.dfg.block_params(top) else {
        return None;
    };
    path.words
        .extend([(*leaf, LEAF), (*branch, BRANCH), (*built, BUILT)]);
    let [body, end] = path.condition(top, &compare(IntCC::UnsignedLessThan, BUILT, LAST))?;
    let success = path.empty_edge(end)?;
    let mut finish = path.clone();
    if !path.effects.is_empty()
        || !matches!(finish.step(success)?, Boundary::Return)
        || !finish.effects.is_empty()
    {
        return None;
    }

    let at_leaf = address(LEAF);
    let at_leaf_next = address(add(LEAF, 1));
    let at_branch = address(BRANCH);
    let at_branch_next = address(add(BRANCH, 1));
    let at_built = address(BUILT);
    let leaf0 = mask(read(&at_leaf), FREQUENCY_MASK);
    let leaf1 = mask(read(&at_leaf_next), FREQUENCY_MASK);
    let branch0 = mask(read(&at_branch), FREQUENCY_MASK);
    let branch1 = mask(read(&at_branch_next), FREQUENCY_MASK);
    let parent = binary(Op::Ishl, BUILT, Word::Constant(10));
    let leaves = binary(
        Op::Band,
        compare(IntCC::UnsignedLessThanOrEqual, add(LEAF, 1), LAST),
        binary(
            Op::Bor,
            compare(IntCC::Equal, BRANCH, BUILT),
            compare(
                IntCC::UnsignedLessThanOrEqual,
                leaf1.clone(),
                branch0.clone(),
            ),
        ),
    );
    let branches = binary(
        Op::Band,
        binary(Op::Bxor, leaves.clone(), Word::Constant(1)),
        binary(
            Op::Band,
            compare(IntCC::UnsignedLessThanOrEqual, add(BRANCH, 2), BUILT),
            binary(
                Op::Bor,
                compare(IntCC::UnsignedGreaterThan, LEAF, LAST),
                compare(IntCC::UnsignedLessThan, branch1.clone(), leaf0.clone()),
            ),
        ),
    );
    let loads = [
        at_leaf,
        at_leaf_next,
        at_branch.clone(),
        at_branch_next.clone(),
    ]
    .map(Effect::Read);
    for (both_leaves, both_branches, left, right, leaf_step, branch_step) in [
        (true, false, leaf0.clone(), leaf1, 2, 0),
        (false, true, branch0.clone(), branch1, 0, 2),
        (false, false, leaf0, branch0, 1, 1),
    ] {
        let mut effects = Vec::from(loads.clone());
        if !both_leaves {
            effects.push(store(&at_branch, parent.clone(), read(&at_branch)));
        }
        if both_branches {
            effects.push(store(
                &at_branch_next,
                parent.clone(),
                read(&at_branch_next),
            ));
        }
        effects.push(Effect::Read(at_built.clone()));
        effects.push(store(
            &at_built,
            binary(Op::Iadd, left, right),
            read(&at_built),
        ));
        let choices = Choices {
            leaves: leaves.clone(),
            branches: branches.clone(),
            both_leaves,
            both_branches,
        };
        visit(
            &mut path.clone(),
            top,
            body,
            &choices,
            [
                &add(LEAF, leaf_step),
                &add(BRANCH, branch_step),
                &add(BUILT, 1),
            ],
            &effects,
        )?;
    }
    Some(Tree {
        entry,
        original,
        success,
        cursors: [
            path.packet_value(0)?,
            path.packet_value(8)?,
            path.packet_value(16)?,
        ],
        data: path.packet_value(32)?,
        length: path.packet_value(40)?,
        last: path.packet_value(24)?,
    })
}
