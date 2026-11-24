//! Verify both active loop paths before grouping indexed U32 loads and stores.
use super::{Boundary, Effect, Path, Word};
use cranelift_codegen::ir::{
    Block, BlockCall, Function, Inst, Opcode, Value, condcodes::IntCC, types,
};

const INDEX: Word = Word::Cursor(0);
const USED: Word = Word::Cursor(1);

pub(crate) struct Pack {
    pub entry: Inst,
    pub original: BlockCall,
    pub input: Value,
    pub input_len: Value,
    pub output: Value,
    pub output_len: Value,
    pub start: Value,
    pub limit: Value,
    pub index: Value,
    pub used: Value,
}

fn carried(path: &Path<'_>, edge: BlockCall, expected: [Word; 2]) -> Option<Block> {
    let lists = &path.f.dfg.value_lists;
    let block = edge.block(lists);
    let [a, b] = path.f.dfg.block_params(block) else {
        return None;
    };
    if ![*a, *b]
        .iter()
        .all(|v| path.f.dfg.value_type(*v) == types::I64)
    {
        return None;
    }
    let mut args = edge.args(lists);
    for word in expected {
        if path.word(args.next()?.as_value()?)? != word {
            return None;
        }
    }
    args.next().is_none().then_some(block)
}

fn check(path: &mut Path<'_>, top: Block, active: bool) -> Option<()> {
    use IntCC::{NotEqual, UnsignedGreaterThanOrEqual, UnsignedLessThan};
    use Opcode::{Bor, Iadd, Ishl};
    let add_one = |word| Word::binary(Iadd, word, Word::Constant(1));
    let index = Word::binary(Iadd, Word::Input(0), INDEX);
    let address = |slot, at| {
        Word::binary(
            Iadd,
            Word::Input(slot),
            Word::binary(Ishl, at, Word::Constant(2)),
        )
    };
    let source = address(32, index.clone());
    let frequency = Word::Read(Box::new(source.clone()));
    let mut effects = vec![Effect::Read(source)];
    if active {
        let shifted = Word::binary(Ishl, frequency.clone(), Word::Constant(10));
        let key = Word::binary(Bor, INDEX, shifted);
        effects.push(Effect::Store(
            address(64, USED),
            Word::LowWord(Box::new(key)),
        ));
    }
    let checks = [
        (
            Word::comparison(UnsignedLessThan, INDEX, Word::Input(8)),
            true,
        ),
        (
            Word::comparison(UnsignedLessThan, index, Word::Input(40)),
            true,
        ),
        (
            Word::comparison(NotEqual, frequency, Word::Constant(0)),
            active,
        ),
        (
            Word::comparison(UnsignedLessThan, USED, Word::Input(72)),
            true,
        ),
        (
            Word::comparison(UnsignedGreaterThanOrEqual, add_one(USED), USED),
            true,
        ),
        (Word::Constant(1), true),
    ];
    let mut block = top;
    for _ in 0..96 {
        let edge = match path.step(block)? {
            Boundary::Jump(_, edge) => edge,
            Boundary::Branch(condition, edges) => {
                let (_, yes) = checks.iter().find(|(test, _)| *test == condition)?;
                edges[usize::from(!*yes)]
            }
            Boundary::Return => return None,
        };
        if edge.block(&path.f.dfg.value_lists) == top {
            carried(
                path,
                edge,
                [add_one(INDEX), if active { add_one(USED) } else { USED }],
            )?;
            return (path.effects == effects).then_some(());
        }
        block = path.enter(edge)?;
    }
    None
}

pub(crate) fn prove(f: &Function) -> Option<Pack> {
    // Grouping may only skip operations without their own trapping side effects.
    if f.layout.blocks().any(|b| {
        f.layout
            .block_insts(b)
            .any(|inst| f.dfg.insts[inst].opcode() == Opcode::Udiv)
    }) {
        return None;
    }
    let entry = f.layout.entry_block()?;
    let [heap, packet, result] = f.dfg.block_params(entry) else {
        return None;
    };
    if ![*heap, *packet, *result]
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
            (*result, Word::OutputPacket),
        ]
        .into(),
        effects: Vec::new(),
        input_bytes: 112,
    };
    let Boundary::Jump(entry, original) = path.step(entry)? else {
        return None;
    };
    let top = carried(&path, original, [Word::Input(16), Word::Input(24)])?;
    let [index, used] = f.dfg.block_params(top) else {
        return None;
    };
    path.words.extend([(*index, INDEX), (*used, USED)]);
    if !path.effects.is_empty() {
        return None;
    }
    check(&mut path.clone(), top, false)?;
    check(&mut path.clone(), top, true)?;
    Some(Pack {
        entry,
        original,
        input: path.packet_value(32)?,
        input_len: path.packet_value(40)?,
        output: path.packet_value(64)?,
        output_len: path.packet_value(72)?,
        start: path.packet_value(0)?,
        limit: path.packet_value(8)?,
        index: path.packet_value(16)?,
        used: path.packet_value(24)?,
    })
}
