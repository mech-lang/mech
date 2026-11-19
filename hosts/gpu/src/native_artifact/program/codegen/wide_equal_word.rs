//! Retain admitted saved-prefix proofs while using the original scalar words.
use super::*;

pub(super) fn apply(
    f: &mut Function,
    cert: &Certificate,
    change: Change,
    state: SavedInterval,
) -> Option<(usize, reuse::State, Value)> {
    let check = f.dfg.make_block();
    let initial = f.dfg.append_block_param(check, types::I64);
    f.layout.append_block(check);
    f.rewrite_branch_destination(change.entry, cert.header, check);
    let mut cursor = FuncCursor::new(f).at_bottom(check);
    let (initial, forwarded) = advance(&mut cursor, cert, change, initial, Some(state));
    cursor.ins().jump(cert.header, &[BlockArg::Value(initial)]);
    forwarded
}
