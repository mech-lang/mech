//! Invocation-local argument/result space for synchronous numeric phase calls.
//! A call consumes current inputs and finishes before its caller loads results;
//! neither packet address is a source value or retained by the array helpers.
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::ir::{StackSlot, StackSlotData, StackSlotKind};
use cranelift_frontend::FunctionBuilder;

#[derive(Clone, Copy)]
pub(super) struct Geometry {
    calls: usize,
    input_words: usize,
    output_words: usize,
    separate_bytes: usize,
}

impl Geometry {
    pub fn derive(
        requirements: impl IntoIterator<Item = (usize, usize)>,
    ) -> Result<Option<Self>, NativeCompileError> {
        let mut geometry = Self {
            calls: 0,
            input_words: 0,
            output_words: 0,
            separate_bytes: 0,
        };
        for (input, output) in requirements {
            geometry.calls += 1;
            geometry.input_words = geometry.input_words.max(input);
            geometry.output_words = geometry.output_words.max(output);
            let bytes = input
                .checked_add(output)
                .and_then(|n| n.checked_mul(8))
                .ok_or_else(|| backend("native phase packet geometry overflows"))?;
            geometry.separate_bytes = geometry
                .separate_bytes
                .checked_add(bytes)
                .ok_or_else(|| backend("native phase packet total overflows"))?;
        }
        Ok((geometry.calls != 0).then_some(geometry))
    }

    pub fn reserve(self, b: &mut FunctionBuilder<'_>) -> Result<Slots, NativeCompileError> {
        // Distinct nonempty slots also preserve disjoint addresses for an empty
        // input signature. No contents are assumed initialized by this reserve.
        let mut allocate =
            |words: usize| {
                let bytes = words
                    .max(1)
                    .checked_mul(8)
                    .and_then(|n| u32::try_from(n).ok())
                    .ok_or_else(|| backend("native phase packet exceeds stack-slot extent"))?;
                Ok(b.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    bytes,
                    3,
                )))
            };
        Ok(Slots {
            input: allocate(self.input_words)?,
            output: allocate(self.output_words)?,
            geometry: self,
        })
    }

    pub fn summary(self) -> serde_json::Value {
        serde_json::json!({"calls":self.calls,"slots":2,"input_bytes":self.input_words.max(1)*8,
            "output_bytes":self.output_words.max(1)*8,"previous_separate_bytes":self.separate_bytes,
            "invocation_local":true,"input_output_disjoint":true,"synchronous_non_escaping":true})
    }
}

pub(super) struct Slots {
    input: StackSlot,
    output: StackSlot,
    geometry: Geometry,
}

impl Slots {
    pub fn for_call(
        &self,
        input: usize,
        output: usize,
    ) -> Result<(StackSlot, StackSlot), NativeCompileError> {
        if input > self.geometry.input_words || output > self.geometry.output_words {
            return Err(backend(
                "native phase packet exceeds its prepared shared extent",
            ));
        }
        Ok((self.input, self.output))
    }
}

/// Current words loaded from the compact caller's immutable input packet.
pub(super) fn input_word_loads(
    function: &cranelift_codegen::ir::Function,
    input_words: usize,
) -> Option<
    std::collections::BTreeMap<cranelift_codegen::ir::Value, (cranelift_codegen::ir::Inst, usize)>,
> {
    use cranelift_codegen::ir::{InstructionData as Data, Opcode as Op, types};
    use std::collections::BTreeMap;
    let entry = function.layout.entry_block()?;
    let &[_, input, _] = function.dfg.block_params(entry) else {
        return None;
    };
    let within =
        |offset: i32| offset >= 0 && offset % 8 == 0 && (offset as usize / 8) < input_words;
    let mut loads = BTreeMap::new();
    for block in function.layout.blocks() {
        for inst in function.layout.block_insts(block) {
            if !function
                .dfg
                .inst_values(inst)
                .any(|v| function.dfg.resolve_aliases(v) == input)
            {
                continue;
            }
            // If the packet pointer itself escapes or is written, this local
            // transformation has no proof of an immutable input word.
            let Data::Load {
                opcode: Op::Load,
                arg,
                offset,
                ..
            } = function.dfg.insts[inst]
            else {
                return None;
            };
            let word = function.dfg.first_result(inst);
            if function.dfg.resolve_aliases(arg) != input
                || !within(i32::from(offset))
                || function.dfg.value_type(word) != types::I64
            {
                return None;
            }
            loads.insert(word, (inst, i32::from(offset) as usize / 8));
        }
    }
    Some(loads)
}

/// The local numeric callee receives two disjoint stack packets from Slots.
/// A directly copied input word can be loaded at its only use on a terminal
/// route. The caller's input slot stays initialized and immutable throughout.
pub(super) fn sink_exit_packet_words(
    function: &mut cranelift_codegen::ir::Function,
    input_words: usize,
    output_words: usize,
) -> usize {
    use cranelift_codegen::ir::{InstructionData as Data, Opcode as Op};
    use std::collections::BTreeMap;

    let Some(entry) = function.layout.entry_block() else {
        return 0;
    };
    let &[_, _, output] = function.dfg.block_params(entry) else {
        return 0;
    };
    let within =
        |offset: i32, words| offset >= 0 && offset % 8 == 0 && (offset as usize / 8) < words;
    let Some(loads) = input_word_loads(function, input_words) else {
        return 0;
    };
    let mut uses = loads
        .keys()
        .map(|&word| (word, Vec::new()))
        .collect::<BTreeMap<_, _>>();
    for block in function.layout.blocks() {
        for inst in function.layout.block_insts(block) {
            for value in function.dfg.inst_values(inst) {
                if let Some(sites) = uses.get_mut(&function.dfg.resolve_aliases(value)) {
                    sites.push(inst);
                }
            }
        }
    }
    let edits = uses
        .into_iter()
        .filter_map(|(word, sites)| {
            let [store] = sites[..] else {
                return None;
            };
            let Data::Store {
                opcode: Op::Store,
                args: [value, address],
                offset,
                ..
            } = function.dfg.insts[store]
            else {
                return None;
            };
            if function.dfg.resolve_aliases(value) != word
                || function.dfg.resolve_aliases(address) != output
                || !within(i32::from(offset), output_words)
                || !terminal_route(function, function.layout.inst_block(store)?)
            {
                return None;
            }
            Some((loads[&word].0, store))
        })
        .collect::<Vec<_>>();
    for &(load, store) in &edits {
        function.layout.remove_inst(load);
        function.layout.insert_inst(load, store);
    }
    edits.len()
}

fn terminal_route(
    function: &cranelift_codegen::ir::Function,
    mut block: cranelift_codegen::ir::Block,
) -> bool {
    use cranelift_codegen::ir::{InstructionData as Data, Opcode as Op};
    let mut visited = std::collections::BTreeSet::new();
    while visited.insert(block) {
        let Some(end) = function.layout.last_inst(block) else {
            return false;
        };
        match function.dfg.insts[end] {
            Data::Jump { destination, .. } => block = destination.block(&function.dfg.value_lists),
            Data::MultiAry {
                opcode: Op::Return, ..
            } => return true,
            _ => return false,
        }
    }
    false
}

#[cfg(test)]
#[path = "phase_storage_tests.rs"]
mod tests;
