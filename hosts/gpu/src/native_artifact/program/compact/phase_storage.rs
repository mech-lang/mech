//! Invocation-local argument/result space for synchronous numeric phase calls.
//! A call consumes current inputs and finishes before its caller loads results;
//! neither packet address is a source value or retained by the array helpers.
use crate::native_artifact::{
    NativeCompileError,
    compiler::{backend, offset},
};
use cranelift_codegen::ir::{InstBuilder, StackSlot, StackSlotData, StackSlotKind, Value, types};
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
            output: Some(allocate(self.output_words)?),
            #[cfg(feature = "early-region-emission")]
            borrowed_output: None,
            geometry: self,
        })
    }

    /// A synchronous source unit may use its caller's private result extent
    /// before publishing its own final result. That extent is disjoint from
    /// the unit's input/owner homes and stays live through every nested call.
    #[cfg(feature = "early-region-emission")]
    pub fn reserve_with_output(
        self,
        b: &mut FunctionBuilder<'_>,
        output: Value,
        words: usize,
    ) -> Result<Slots, NativeCompileError> {
        if self.output_words > words {
            return Err(backend("borrowed phase output exceeds its caller extent"));
        }
        let bytes = self
            .input_words
            .max(1)
            .checked_mul(8)
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| backend("native phase input exceeds stack-slot extent"))?;
        let input =
            b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, bytes, 3));
        Ok(Slots {
            input,
            output: None,
            borrowed_output: Some(output),
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
    output: Option<StackSlot>,
    #[cfg(feature = "early-region-emission")]
    borrowed_output: Option<Value>,
    geometry: Geometry,
}

impl Slots {
    #[cfg(feature = "early-region-emission")]
    pub fn for_call(
        &self,
        input: usize,
        output: usize,
    ) -> Result<(StackSlot, StackSlot), NativeCompileError> {
        let input = self.input_for_call(input, output)?;
        Ok((
            input,
            self.output
                .ok_or_else(|| backend("call requires a local output slot"))?,
        ))
    }

    pub fn input_for_call(
        &self,
        input: usize,
        output: usize,
    ) -> Result<StackSlot, NativeCompileError> {
        if input > self.geometry.input_words || output > self.geometry.output_words {
            return Err(backend(
                "native phase packet exceeds its prepared shared extent",
            ));
        }
        Ok(self.input)
    }

    pub fn output_address(
        &self,
        b: &mut FunctionBuilder<'_>,
        input: usize,
        output: usize,
    ) -> Result<Value, NativeCompileError> {
        self.input_for_call(input, output)?;
        #[cfg(feature = "early-region-emission")]
        if let Some(output) = self.borrowed_output {
            return Ok(output);
        }
        let output = self
            .output
            .ok_or_else(|| backend("phase output storage is absent"))?;
        Ok(b.ins().stack_addr(types::I64, output, 0))
    }

    pub fn read_output(
        &self,
        b: &mut FunctionBuilder<'_>,
        at: usize,
    ) -> Result<Value, NativeCompileError> {
        if at >= self.geometry.output_words {
            return Err(backend("phase output read exceeds its prepared extent"));
        }
        #[cfg(feature = "early-region-emission")]
        if let Some(output) = self.borrowed_output {
            return Ok(b.ins().load(
                types::I64,
                cranelift_codegen::ir::MemFlags::new(),
                output,
                offset(at)?,
            ));
        }
        let output = self
            .output
            .ok_or_else(|| backend("phase output storage is absent"))?;
        Ok(b.ins().stack_load(types::I64, output, offset(at)?))
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

/// Numeric phases receive an initialized, aligned stack input packet from
/// Slots, separate from their output packet. Its address does not escape the
/// callee. Preserve this contract in IR so unused packet loads can disappear
/// and repeated reads need not retain separate register values.
pub(super) fn qualify_input_packet_loads(
    function: &mut cranelift_codegen::ir::Function,
    input_words: usize,
) -> usize {
    use cranelift_codegen::ir::InstructionData;
    let Some(loads) = input_word_loads(function, input_words) else {
        return 0;
    };
    for &(inst, _) in loads.values() {
        let InstructionData::Load { flags, .. } = &mut function.dfg.insts[inst] else {
            unreachable!("validated input packet load");
        };
        flags.set_notrap();
        flags.set_readonly();
        flags.set_aligned();
    }
    loads.len()
}

/// Compact a caller's initialized, nonescaping input slot to the offsets
/// still read by this body's finalized IR. Return their original word order.
pub(super) fn pack_used_input_words(
    function: &mut cranelift_codegen::ir::Function,
    input_words: usize,
) -> Option<Vec<usize>> {
    use cranelift_codegen::ir::InstructionData;
    use std::collections::BTreeSet;

    let loads = input_word_loads(function, input_words)?;
    let live = function
        .layout
        .blocks()
        .flat_map(|block| function.layout.block_insts(block))
        .flat_map(|inst| function.dfg.inst_values(inst))
        .map(|word| function.dfg.resolve_aliases(word))
        .collect::<BTreeSet<_>>();
    let kept = loads
        .iter()
        .filter_map(|(value, &(_, word))| live.contains(value).then_some(word))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if kept.len() == input_words {
        return None;
    }
    for (value, (inst, word)) in loads {
        if !live.contains(&value) {
            // Every original offset was validated within the readable stack
            // extent. An unused input load cannot trap or observe other storage.
            function.layout.remove_inst(inst);
            continue;
        }
        let at = kept.binary_search(&word).expect("live input word");
        let InstructionData::Load { offset, .. } = &mut function.dfg.insts[inst] else {
            unreachable!("validated input load");
        };
        *offset = ((at * 8) as i32).into();
    }
    Some(kept)
}

/// The local numeric callee receives two disjoint stack packets from Slots.
/// Load an input word at its first copy on a terminal route. The caller's
/// input slot stays initialized and immutable throughout.
pub(super) fn sink_exit_packet_words(
    function: &mut cranelift_codegen::ir::Function,
    input_words: usize,
    output_words: usize,
    repeated_copies: bool,
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
            let (&first, remaining) = sites.split_first()?;
            if !remaining.is_empty() && !repeated_copies {
                return None;
            }
            let exit = function.layout.inst_block(first)?;
            if !terminal_route(function, exit) {
                return None;
            }
            for store in sites {
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
                    || function.layout.inst_block(store) != Some(exit)
                {
                    return None;
                }
            }
            Some((loads[&word].0, first))
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
