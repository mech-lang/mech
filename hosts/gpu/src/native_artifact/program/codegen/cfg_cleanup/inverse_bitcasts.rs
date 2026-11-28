//! Lossless scalar round trips only. No FP operation, memory access, failure
//! edge, source debit or publication moves. Other uses keep the inner cast.
use cranelift_codegen::ir::{Function, InstructionData, Opcode, Value, ValueDef};

fn scalar(function: &Function, value: Value) -> bool {
    let ty = function.dfg.value_type(value);
    !ty.is_vector() && (ty.is_int() || ty.is_float())
}
pub(super) fn fold(function: &mut Function) -> usize {
    let instructions = function
        .layout
        .blocks()
        .flat_map(|block| function.layout.block_insts(block))
        .collect::<Vec<_>>();
    let mut changed = 0;
    for inst in instructions {
        let InstructionData::LoadNoOffset {
            opcode: Opcode::Bitcast,
            flags,
            arg,
        } = function.dfg.insts[inst]
        else {
            continue;
        };
        let [result] = function.dfg.inst_results(inst) else {
            continue;
        };
        let result = *result;
        let middle = function.dfg.resolve_aliases(arg);
        let ValueDef::Result(inner, 0) = function.dfg.value_def(middle) else {
            continue;
        };
        let InstructionData::LoadNoOffset {
            opcode: Opcode::Bitcast,
            flags: inner_flags,
            arg: source,
        } = function.dfg.insts[inner]
        else {
            continue;
        };
        let source = function.dfg.resolve_aliases(source);
        if flags != inner_flags
            || !scalar(function, source)
            || !scalar(function, middle)
            || function.dfg.value_type(source) != function.dfg.value_type(result)
        {
            continue;
        }
        function.dfg.detach_inst_results(inst);
        function.dfg.change_to_alias(result, source);
        function.layout.remove_inst(inst);
        changed += 1;
    }
    changed
}
#[cfg(test)]
#[path = "inverse_bitcasts_tests.rs"]
mod tests;
